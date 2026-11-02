use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use super::store::IntegrationStore;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::integrations::{
    ConfigInput, ConnectionTestOptions, INTEGRATION_SECRET_OWNER, Integration, IntegrationError,
    IntegrationErrorSource, IntegrationStatus, ProviderKind, ResolvedConfig,
    SharedIntegrationProvider, missing_required, secret_key, validate_config,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, RecordRef, RecordType, ensure_version,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::platform::SharedSecretVault;

pub(crate) const INTEGRATION_PROVIDER_NOT_FOUND: &str = "integration_provider_not_found";
pub(crate) const INTEGRATION_NOT_CONFIGURABLE: &str = "integration_not_configurable";
pub(crate) const INTEGRATION_NOT_CONFIGURED: &str = "integration_not_configured";
pub(crate) const INTEGRATION_TEST_FAILED: &str = "integration_test_failed";
pub(crate) const INTEGRATION_ACTIVATION_FAILED: &str = "integration_activation_failed";

pub(crate) const INTEGRATION_CONFIGURED: &str = "integration.configured";
pub(crate) const INTEGRATION_ACTIVATED: &str = "integration.activated";
pub(crate) const INTEGRATION_DEACTIVATED: &str = "integration.deactivated";
pub(crate) const INTEGRATION_TESTED: &str = "integration.tested";
pub(crate) const INTEGRATION_REMOVED: &str = "integration.removed";

pub(crate) const INTEGRATION_RECORD_TYPE: RecordType =
    RecordType::from_static("integrations.integration");

#[derive(Clone)]
pub(crate) struct IntegrationView {
    pub(crate) provider: SharedIntegrationProvider,
    pub(crate) instance: Option<Integration>,
    pub(crate) configured_secrets: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub(crate) struct ConfigureIntegrationCommand {
    pub(crate) input: ConfigInput,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionTestResult {
    pub(crate) ok: bool,
    pub(crate) message: Option<String>,
    pub(crate) tested_at: DateTime<Utc>,
}

fn provider_not_found() -> AppError {
    AppError::not_found_code(
        INTEGRATION_PROVIDER_NOT_FOUND,
        "integration provider was not found",
    )
}

fn not_configurable() -> AppError {
    AppError::validation_code(
        INTEGRATION_NOT_CONFIGURABLE,
        "this integration has no configuration of its own",
    )
}

fn not_configured() -> AppError {
    AppError::validation_code(
        INTEGRATION_NOT_CONFIGURED,
        "the integration is not configured yet",
    )
}

fn vault(ctx: &Ctx<'_>) -> Result<SharedSecretVault, AppError> {
    ctx.secrets()
}

fn provider(key: &str) -> Result<SharedIntegrationProvider, AppError> {
    crate::registry::builtin_integrations()
        .get(key.trim())
        .cloned()
        .ok_or_else(provider_not_found)
}

fn configurable(key: &str) -> Result<SharedIntegrationProvider, AppError> {
    let provider = provider(key)?;
    match provider.kind() {
        ProviderKind::Configurable => Ok(provider),
        ProviderKind::Shortcut { .. } => Err(not_configurable()),
    }
}

async fn configured_secrets(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
) -> Result<BTreeMap<String, BTreeSet<String>>, AppError> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let owner_ids: Vec<String> = ids.iter().map(Uuid::to_string).collect();
    let mut configured: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for key in vault(ctx)?
        .configured(ctx.scope, INTEGRATION_SECRET_OWNER, &owner_ids)
        .await?
    {
        configured.entry(key.owner_id).or_default().insert(key.name);
    }
    Ok(configured)
}

async fn view(
    ctx: &Ctx<'_>,
    provider: SharedIntegrationProvider,
    instance: Option<Integration>,
) -> Result<IntegrationView, AppError> {
    let configured_secrets = match &instance {
        Some(instance) => configured_secrets(ctx, &[instance.id])
            .await?
            .remove(&instance.id.to_string())
            .unwrap_or_default(),
        None => BTreeSet::new(),
    };
    Ok(IntegrationView {
        provider,
        instance,
        configured_secrets,
    })
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<IntegrationView>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut instances = IntegrationStore::of(ctx).list().await?;
    let ids: Vec<Uuid> = instances.iter().map(|instance| instance.id).collect();
    let mut configured = configured_secrets(ctx, &ids).await?;
    Ok(crate::registry::builtin_integrations()
        .providers()
        .iter()
        .filter(|provider| provider.is_available(ctx.scope))
        .map(|provider| {
            let instance = instances
                .iter()
                .position(|instance| instance.provider_key == provider.key())
                .map(|index| instances.swap_remove(index));
            let configured_secrets = instance
                .as_ref()
                .and_then(|instance| configured.remove(&instance.id.to_string()))
                .unwrap_or_default();
            IntegrationView {
                provider: provider.clone(),
                instance,
                configured_secrets,
            }
        })
        .collect())
}

pub(crate) async fn get(ctx: &Ctx<'_>, provider_key: &str) -> Result<IntegrationView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = provider(provider_key)?;
    let instance = IntegrationStore::of(ctx).find(provider.key()).await?;
    view(ctx, provider, instance).await
}

async fn resolve(
    ctx: &Ctx<'_>,
    provider: &SharedIntegrationProvider,
    instance: &Integration,
) -> Result<ResolvedConfig, AppError> {
    let mut resolved = ResolvedConfig {
        values: instance.config.clone(),
        secrets: BTreeMap::new(),
    };
    for name in provider.config_schema().secret_keys() {
        if let Some(secret) = vault(ctx)?
            .reveal(ctx.scope, &secret_key(instance.id, name))
            .await?
        {
            resolved.secrets.insert(name.to_owned(), secret);
        }
    }
    Ok(resolved)
}

async fn ready_config(
    ctx: &Ctx<'_>,
    provider: &SharedIntegrationProvider,
    instance: &Integration,
) -> Result<ResolvedConfig, AppError> {
    let resolved = resolve(ctx, provider, instance).await?;
    let mut violations = missing_required(&provider.config_schema(), &resolved);
    violations.extend(provider.validate(&resolved));
    if violations.is_empty() {
        Ok(resolved)
    } else {
        Err(AppError::invalid_fields_code(
            INTEGRATION_NOT_CONFIGURED,
            "the integration configuration is incomplete",
            violations,
        ))
    }
}

pub(crate) async fn configure(
    ctx: &Ctx<'_>,
    provider_key: &str,
    command: ConfigureIntegrationCommand,
) -> Result<IntegrationView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = configurable(provider_key)?;
    let schema = provider.config_schema();
    let now = ctx.now();

    let mut tx = ctx.begin().await?;
    let existing = IntegrationStore::of(ctx)
        .find_for_update(&mut tx, provider.key())
        .await?;
    if let Some(existing) = &existing {
        ensure_version(command.expected_updated_at, existing.updated_at)?;
    }
    let configured = match &existing {
        Some(existing) => configured_secrets(ctx, &[existing.id])
            .await?
            .remove(&existing.id.to_string())
            .unwrap_or_default(),
        None => BTreeSet::new(),
    };
    let validated = validate_config(&schema, command.input, &configured)
        .map_err(|violations| AppError::invalid_fields("invalid configuration", violations))?;
    let preview = ResolvedConfig {
        values: validated.values.clone(),
        secrets: BTreeMap::new(),
    };
    let violations = provider.validate(&preview);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid configuration",
            violations,
        ));
    }

    let created = existing.is_none();
    let mut integration = existing.unwrap_or_else(|| Integration::new(provider.key(), now));
    let secret_names: Vec<&str> = schema.secret_keys().collect();
    let mut changes = diff_changes(
        &integration.config,
        &validated.values,
        &DiffOptions::default().redacting(&secret_names),
    )?;
    integration.config = validated.values;
    integration.updated_at = now;
    IntegrationStore::of(ctx)
        .save(&mut tx, &integration)
        .await?;
    for (name, value) in &validated.secrets_to_store {
        vault(ctx)?
            .store(&mut tx, ctx.scope, &secret_key(integration.id, name), value)
            .await?;
        changes.push(FieldChange::redacted(name.clone()));
    }
    for name in &validated.secrets_to_clear {
        if configured.contains(name) {
            vault(ctx)?
                .remove(&mut tx, ctx.scope, &secret_key(integration.id, name))
                .await?;
            changes.push(FieldChange::new(
                name.clone(),
                Value::String(aphel_kernel::application::shared::REDACTED.to_owned()),
                Value::Null,
            ));
        }
    }
    publish(
        ctx,
        &mut tx,
        &integration,
        INTEGRATION_CONFIGURED,
        json!({ "created": created }),
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    view(ctx, provider, Some(integration)).await
}

pub(crate) async fn test(
    ctx: &Ctx<'_>,
    provider_key: &str,
    options: ConnectionTestOptions,
    ip: Option<String>,
) -> Result<(IntegrationView, ConnectionTestResult), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = configurable(provider_key)?;
    let instance = IntegrationStore::of(ctx)
        .find(provider.key())
        .await?
        .ok_or_else(not_configured)?;
    let resolved = ready_config(ctx, &provider, &instance).await?;
    let outcome = provider.test_connection(&resolved, &options).await;
    let integration =
        record_test(ctx, &provider, &outcome, IntegrationErrorSource::Test, ip).await?;
    let result = ConnectionTestResult {
        ok: outcome.is_ok(),
        message: outcome.err().map(|message| truncate(&message)),
        tested_at: integration.last_tested_at.unwrap_or_else(Utc::now),
    };
    Ok((view(ctx, provider, Some(integration)).await?, result))
}

async fn record_test(
    ctx: &Ctx<'_>,
    provider: &SharedIntegrationProvider,
    outcome: &Result<(), String>,
    source: IntegrationErrorSource,
    ip: Option<String>,
) -> Result<Integration, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let mut integration = IntegrationStore::of(ctx)
        .find_for_update(&mut tx, provider.key())
        .await?
        .ok_or_else(not_configured)?;
    let previous_status = integration.status;
    integration.last_tested_at = Some(now);
    integration.last_test_ok = Some(outcome.is_ok());
    match outcome {
        Ok(()) if integration.status == IntegrationStatus::Error => {
            integration.status = IntegrationStatus::Active;
        }
        Err(message) => {
            integration.push_error(IntegrationError::new(now, source, message));
            if integration.status.is_enabled() {
                integration.status = IntegrationStatus::Error;
            }
        }
        Ok(()) => {}
    }
    IntegrationStore::of(ctx)
        .save(&mut tx, &integration)
        .await?;
    let mut changes = vec![FieldChange::new(
        "last_test_ok",
        Value::Null,
        json!(outcome.is_ok()),
    )];
    if previous_status != integration.status {
        changes.push(FieldChange::new(
            "status",
            json!(previous_status.as_str()),
            json!(integration.status.as_str()),
        ));
    }
    publish(
        ctx,
        &mut tx,
        &integration,
        INTEGRATION_TESTED,
        json!({ "ok": outcome.is_ok() }),
        changes,
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(integration)
}

pub(crate) async fn activate(
    ctx: &Ctx<'_>,
    provider_key: &str,
    ip: Option<String>,
) -> Result<IntegrationView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = configurable(provider_key)?;
    let instance = IntegrationStore::of(ctx)
        .find(provider.key())
        .await?
        .ok_or_else(not_configured)?;
    if instance.status == IntegrationStatus::Active {
        return view(ctx, provider, Some(instance)).await;
    }
    let resolved = ready_config(ctx, &provider, &instance).await?;
    if provider.requires_successful_test_to_activate() {
        let outcome = provider
            .test_connection(&resolved, &ConnectionTestOptions::default())
            .await;
        if let Err(message) = &outcome {
            record_test(
                ctx,
                &provider,
                &outcome,
                IntegrationErrorSource::Activation,
                ip,
            )
            .await?;
            return Err(AppError::validation_code(
                INTEGRATION_TEST_FAILED,
                format!("connection test failed: {}", truncate(message)),
            ));
        }
    }
    if let Err(message) = provider.on_activate(&resolved).await {
        return Err(AppError::validation_code(
            INTEGRATION_ACTIVATION_FAILED,
            format!("activation failed: {}", truncate(&message)),
        ));
    }
    let tested = provider.requires_successful_test_to_activate();
    change_status(
        ctx,
        provider,
        IntegrationStatus::Active,
        INTEGRATION_ACTIVATED,
        tested,
        ip,
    )
    .await
}

pub(crate) async fn deactivate(
    ctx: &Ctx<'_>,
    provider_key: &str,
    ip: Option<String>,
) -> Result<IntegrationView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = configurable(provider_key)?;
    let instance = IntegrationStore::of(ctx)
        .find(provider.key())
        .await?
        .ok_or_else(not_configured)?;
    if !instance.status.is_enabled() {
        return view(ctx, provider, Some(instance)).await;
    }
    if let Err(message) = provider.on_deactivate().await {
        tracing::warn!(
            provider = provider.key(),
            "integration deactivation hook failed: {message}"
        );
    }
    change_status(
        ctx,
        provider,
        IntegrationStatus::Inactive,
        INTEGRATION_DEACTIVATED,
        false,
        ip,
    )
    .await
}

async fn change_status(
    ctx: &Ctx<'_>,
    provider: SharedIntegrationProvider,
    status: IntegrationStatus,
    event_type: &'static str,
    tested: bool,
    ip: Option<String>,
) -> Result<IntegrationView, AppError> {
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let mut integration = IntegrationStore::of(ctx)
        .find_for_update(&mut tx, provider.key())
        .await?
        .ok_or_else(not_configured)?;
    let previous = integration.status;
    integration.status = status;
    integration.updated_at = now;
    if tested {
        integration.last_tested_at = Some(now);
        integration.last_test_ok = Some(true);
    }
    IntegrationStore::of(ctx)
        .save(&mut tx, &integration)
        .await?;
    publish(
        ctx,
        &mut tx,
        &integration,
        event_type,
        json!({}),
        vec![FieldChange::new(
            "status",
            json!(previous.as_str()),
            json!(status.as_str()),
        )],
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    view(ctx, provider, Some(integration)).await
}

pub(crate) async fn remove(
    ctx: &Ctx<'_>,
    provider_key: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let provider = configurable(provider_key)?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let Some(integration) = IntegrationStore::of(ctx)
        .find_for_update(&mut tx, provider.key())
        .await?
    else {
        return Ok(());
    };
    if integration.status.is_enabled()
        && let Err(message) = provider.on_deactivate().await
    {
        tracing::warn!(
            provider = provider.key(),
            "integration deactivation hook failed: {message}"
        );
    }
    IntegrationStore::of(ctx)
        .delete(&mut tx, integration.id)
        .await?;
    vault(ctx)?
        .remove_owner(
            &mut tx,
            ctx.scope,
            INTEGRATION_SECRET_OWNER,
            &integration.id.to_string(),
        )
        .await?;
    let changes = vec![FieldChange::new(
        "status",
        json!(integration.status.as_str()),
        Value::Null,
    )];
    publish(
        ctx,
        &mut tx,
        &integration,
        INTEGRATION_REMOVED,
        json!({}),
        changes,
        ip,
        now,
    )
    .await?;
    Ok(tx.commit().await?)
}

#[allow(clippy::too_many_arguments)]
async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    integration: &Integration,
    event_type: &'static str,
    extra: Value,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(INTEGRATION_RECORD_TYPE, integration.id);
    let mut payload = json!({
        "provider_key": integration.provider_key,
        "status": integration.status.as_str(),
    });
    if let (Some(payload), Value::Object(extra)) = (payload.as_object_mut(), extra) {
        payload.extend(extra);
    }
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

fn truncate(message: &str) -> String {
    aphel_kernel::application::integrations::truncate_error(message)
}
