use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::{EmailVisibility, MailboxStatus, normalize_address};
use crate::mailboxes::{
    ConnectMailboxCommand, MAILBOX_ADDRESS_MISMATCH, MAILBOX_ALREADY_CONNECTED,
    MAILBOX_DISCONNECTED_CODE, MAILBOX_RECORD_TYPE, MailProviderInfo, MailboxDetails,
    MailboxSnapshot, UpdateMailboxCommand, mailbox_not_found, mailbox_secret_key,
    parse_display_name, parse_history_days, parse_visibility, public_settings, scope_user_uuid,
    settings_object, status_reason,
};
use crate::store::MailboxStore;
use crate::store::settings::{DirectoryStore, SettingsStore};
use crate::sync::sync_mailbox_job;
use crate::{
    CredentialPersistence, CredentialSink, MAILBOX_CONNECTED, MAILBOX_CREDENTIALS_SECRET,
    MAILBOX_DISCONNECTED, MAILBOX_SECRET_OWNER, MAILBOX_UPDATED, MailProviderError, MailboxAccount,
    MailboxChanges, MailboxConnectRequest, MailboxCredentials, MailboxSyncState, MailboxView,
    NewMailbox, SharedMailboxProvider, mailbox_settings_or_empty,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::secrets::SecretVault;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, JobQueue, RecordRef,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::platform::SharedSecretVault;

pub(crate) fn providers(ctx: &Ctx<'_>) -> Vec<MailProviderInfo> {
    crate::store::mail_providers()
        .available_for(ctx.scope)
        .into_iter()
        .map(|provider| MailProviderInfo {
            key: provider.key().to_owned(),
            label: provider.label().to_owned(),
            connection: provider.connection(),
        })
        .collect()
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<MailboxDetails>, AppError> {
    let mailboxes = MailboxStore::of(ctx)
        .list_for_user(ctx.scope.user_id())
        .await?;
    details(ctx, mailboxes).await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<MailboxDetails, AppError> {
    let mailbox = owned(ctx, id).await?;
    detail(ctx, mailbox).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: ConnectMailboxCommand,
) -> Result<MailboxDetails, AppError> {
    let scope = ctx.scope;
    let state = ctx.state();
    let mailboxes = MailboxStore::of(ctx);
    let provider = crate::store::mail_providers().require_for(scope, command.provider.trim())?;
    let mut violations = Vec::new();
    let display_name = parse_display_name(command.display_name, &mut violations);
    let default_visibility = match command.default_visibility.as_deref() {
        Some(value) => parse_visibility(value, &mut violations),
        None => Some(EmailVisibility::default()),
    };
    let history_days = match command.history_days {
        Some(days) => parse_history_days(days, &mut violations),
        None => Some(
            SettingsStore::of(ctx)
                .capture_settings()
                .await?
                .initial_history_days,
        ),
    };
    let settings = settings_object(command.settings, &mut violations);
    let (Some(default_visibility), Some(history_days), true) =
        (default_visibility, history_days, violations.is_empty())
    else {
        return Err(AppError::invalid_fields("invalid mailbox", violations));
    };
    let user_id = scope_user_uuid(scope)?;
    let connected = provider
        .connect(MailboxConnectRequest {
            user_id,
            settings,
            credentials: MailboxCredentials(command.credentials),
        })
        .await
        .map_err(AppError::from)?;
    let address = normalize_address(&connected.address).map_err(|message| {
        AppError::from(MailProviderError::rejected(format!(
            "the provider returned an invalid address: {message}"
        )))
    })?;
    if mailboxes
        .find_by_address(scope.user_id(), &address)
        .await?
        .is_some()
    {
        return Err(AppError::conflict_code(
            MAILBOX_ALREADY_CONNECTED,
            "this mailbox is already connected",
        ));
    }
    let mailbox = NewMailbox {
        id: Uuid::now_v7(),
        owner: scope.user_id().clone(),
        provider: provider.key().to_owned(),
        address,
        display_name: display_name.or(connected.display_name),
        settings: public_settings(&mailbox_settings_or_empty(Some(connected.settings))),
        history_days,
        default_visibility,
    };
    let now = ctx.now();
    let snapshot = MailboxSnapshot {
        provider: mailbox.provider.clone(),
        address: mailbox.address.clone(),
        display_name: mailbox.display_name.clone(),
        settings: mailbox.settings.clone(),
        history_days: mailbox.history_days,
        default_visibility: mailbox.default_visibility.as_str(),
    };
    let mut changes = diff_changes(&Value::Null, &snapshot, &DiffOptions::default())?;
    changes.push(FieldChange::redacted("credentials"));
    let mut tx = ctx.begin().await?;
    mailboxes.insert(&mut tx, &mailbox, now).await?;
    state
        .secrets
        .store(
            &mut tx,
            scope,
            &mailbox_secret_key(mailbox.id),
            &connected.credentials.to_secret(),
        )
        .await?;
    ctx.jobs()
        .enqueue(&mut tx, sync_mailbox_job(mailbox.id, scope.user_id()))
        .await?;
    record(
        ctx,
        &mut tx,
        MAILBOX_CONNECTED,
        mailbox.id,
        &mailbox.provider,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    get(ctx, &mailbox.id.to_string()).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateMailboxCommand,
) -> Result<MailboxDetails, AppError> {
    let scope = ctx.scope;
    let current = owned(ctx, id).await?;
    let mut violations = Vec::new();
    let mut changes = MailboxChanges::default();
    if let Some(display_name) = command.display_name {
        changes.display_name = Some(parse_display_name(display_name, &mut violations));
    }
    if let Some(value) = command.default_visibility.as_deref() {
        changes.default_visibility = parse_visibility(value, &mut violations);
    }
    if let Some(days) = command.history_days {
        changes.history_days = parse_history_days(days, &mut violations);
    }
    let settings = command
        .settings
        .map(|settings| settings_object(Some(settings), &mut violations));
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid mailbox", violations));
    }
    let reconnect = settings.is_some() || command.credentials.is_some();
    let mut credentials = None;
    if reconnect {
        let provider = provider(ctx, &current)?;
        let mut stored = stored_credentials(ctx, current.id).await?;
        if let Some(new_values) = command.credentials {
            for (key, value) in new_values {
                stored.0.insert(key, value);
            }
        }
        let mut merged = match &current.settings {
            Value::Object(values) => values.clone(),
            _ => Map::new(),
        };
        if let Some(Value::Object(values)) = settings {
            for (key, value) in values {
                merged.insert(key, value);
            }
        }
        let user_id = scope_user_uuid(scope)?;
        let connected = provider
            .connect(MailboxConnectRequest {
                user_id,
                settings: Value::Object(merged),
                credentials: stored,
            })
            .await
            .map_err(AppError::from)?;
        let address = normalize_address(&connected.address).unwrap_or_default();
        if address != current.address {
            return Err(AppError::validation_code(
                MAILBOX_ADDRESS_MISMATCH,
                "the new settings belong to a different mailbox address",
            ));
        }
        changes.settings = Some(public_settings(&mailbox_settings_or_empty(Some(
            connected.settings,
        ))));
        credentials = Some(connected.credentials);
    }
    let mut next = current.clone();
    if let Some(display_name) = &changes.display_name {
        next.display_name = display_name.clone();
    }
    if let Some(settings) = &changes.settings {
        next.settings = settings.clone();
    }
    if let Some(days) = changes.history_days {
        next.history_days = days;
    }
    if let Some(visibility) = changes.default_visibility {
        next.default_visibility = visibility;
    }
    let mut diff = diff_changes(
        &MailboxSnapshot::from(&current),
        &MailboxSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    if credentials.is_some() {
        diff.push(FieldChange::redacted("credentials"));
    }
    if diff.is_empty() {
        return detail(ctx, current).await;
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    MailboxStore::of(ctx)
        .update(&mut tx, current.id, &changes, now)
        .await?;
    if let Some(credentials) = &credentials {
        ctx.state()
            .secrets
            .store(
                &mut tx,
                scope,
                &mailbox_secret_key(current.id),
                &credentials.to_secret(),
            )
            .await?;
    }
    record(
        ctx,
        &mut tx,
        MAILBOX_UPDATED,
        current.id,
        &current.provider,
        diff,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    if reconnect {
        DirectoryStore::of(ctx)
            .clear_sync_failure(current.id)
            .await?;
        save_state(ctx, current.id, MailboxStatus::Active, None).await?;
    }
    get(ctx, id).await
}

pub(crate) async fn test(ctx: &Ctx<'_>, id: &str) -> Result<MailboxDetails, AppError> {
    let mailbox = owned(ctx, id).await?;
    let provider = provider(ctx, &mailbox)?;
    let account = account(ctx, &mailbox).await?;
    match provider.test(&account).await {
        Ok(refreshed) => {
            if let Some(credentials) = refreshed {
                persist_credentials(ctx, mailbox.id, &credentials).await?;
            }
            DirectoryStore::of(ctx)
                .clear_sync_failure(mailbox.id)
                .await?;
            save_state(ctx, mailbox.id, MailboxStatus::Active, None).await?;
            get(ctx, id).await
        }
        Err(error) => {
            save_state(
                ctx,
                mailbox.id,
                MailboxStatus::Error,
                Some(status_reason(&error.to_string())),
            )
            .await?;
            Err(error.into())
        }
    }
}

pub(crate) async fn request_sync(ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
    let mailbox = owned(ctx, id).await?;
    if !mailbox.status.can_sync() {
        return Err(AppError::conflict_code(
            MAILBOX_DISCONNECTED_CODE,
            "the mailbox is disconnected",
        ));
    }
    DirectoryStore::of(ctx)
        .clear_sync_failure(mailbox.id)
        .await?;
    ctx.jobs()
        .enqueue_now(sync_mailbox_job(mailbox.id, ctx.scope.user_id()))
        .await?;
    Ok(())
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let scope = ctx.scope;
    let state = ctx.state();
    let directory = DirectoryStore::of(ctx);
    let mailbox = owned(ctx, id).await?;
    let files = directory.mailbox_files(mailbox.id).await?;
    let mut changes = diff_changes(
        &MailboxSnapshot::from(&mailbox),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    changes.push(FieldChange::redacted("credentials"));
    let mut tx = ctx.begin().await?;
    if !MailboxStore::of(ctx).delete(&mut tx, mailbox.id).await? {
        return Err(mailbox_not_found());
    }
    state
        .secrets
        .remove_owner(
            &mut tx,
            scope,
            MAILBOX_SECRET_OWNER,
            &mailbox.id.to_string(),
        )
        .await?;
    let file_ids: Vec<Uuid> = files.iter().map(|file| file.id).collect();
    directory.delete_files(&mut tx, &file_ids).await?;
    record(
        ctx,
        &mut tx,
        MAILBOX_DISCONNECTED,
        mailbox.id,
        &mailbox.provider,
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    for file in files {
        if let Err(error) = state.storage.delete(&file.storage_key).await {
            tracing::warn!(file_id = %file.id, "could not delete a mailbox file: {error}");
        }
    }
    Ok(())
}

pub(crate) async fn account(
    ctx: &Ctx<'_>,
    mailbox: &MailboxView,
) -> Result<MailboxAccount, AppError> {
    let user_id = Uuid::parse_str(mailbox.owner.as_str())
        .map_err(|_| AppError::Internal("user id is not a UUID".to_owned()))?;
    Ok(MailboxAccount {
        mailbox_id: mailbox.id,
        user_id,
        address: mailbox.address.clone(),
        settings: mailbox.settings.clone(),
        credentials: stored_credentials(ctx, mailbox.id).await?,
        persistence: credential_persistence(ctx),
    })
}

struct VaultCredentialSink {
    pool: MySqlPool,
    secrets: SharedSecretVault,
    scope: AccessScope,
}

#[async_trait]
impl CredentialSink for VaultCredentialSink {
    async fn persist(
        &self,
        mailbox_id: Uuid,
        credentials: &MailboxCredentials,
    ) -> Result<(), AppError> {
        let mut tx = aphel_kernel::database::transaction::MySqlTx::from(self.pool.begin().await?);
        self.secrets
            .store(
                &mut tx,
                &self.scope,
                &mailbox_secret_key(mailbox_id),
                &credentials.to_secret(),
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

pub(crate) fn credential_persistence(ctx: &Ctx<'_>) -> CredentialPersistence {
    CredentialPersistence::new(Arc::new(VaultCredentialSink {
        pool: ctx.pool().clone(),
        secrets: ctx.state().secrets,
        scope: ctx.scope.clone(),
    }))
}

pub(crate) async fn persist_credentials(
    ctx: &Ctx<'_>,
    mailbox_id: Uuid,
    credentials: &MailboxCredentials,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    ctx.state()
        .secrets
        .store(
            &mut tx,
            ctx.scope,
            &mailbox_secret_key(mailbox_id),
            &credentials.to_secret(),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn stored_credentials(
    ctx: &Ctx<'_>,
    mailbox_id: Uuid,
) -> Result<MailboxCredentials, AppError> {
    match ctx
        .state()
        .secrets
        .reveal(ctx.scope, &mailbox_secret_key(mailbox_id))
        .await?
    {
        Some(secret) => MailboxCredentials::from_secret(&secret),
        None => Ok(MailboxCredentials::default()),
    }
}

fn provider(ctx: &Ctx<'_>, mailbox: &MailboxView) -> Result<SharedMailboxProvider, AppError> {
    crate::store::mail_providers().require_for(ctx.scope, &mailbox.provider)
}

async fn save_state(
    ctx: &Ctx<'_>,
    id: Uuid,
    status: MailboxStatus,
    reason: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    MailboxStore::of(ctx)
        .save_sync_state(
            id,
            &MailboxSyncState {
                status,
                status_reason: reason,
                cursor: None,
                sync_started_at: None,
                last_synced_at: None,
            },
            now,
        )
        .await
}

async fn owned(ctx: &Ctx<'_>, id: &str) -> Result<MailboxView, AppError> {
    let id = Uuid::parse_str(id).map_err(|_| mailbox_not_found())?;
    MailboxStore::of(ctx)
        .find(id)
        .await?
        .filter(|mailbox| mailbox.is_owned_by(ctx.scope.user_id()))
        .ok_or_else(mailbox_not_found)
}

async fn detail(ctx: &Ctx<'_>, mailbox: MailboxView) -> Result<MailboxDetails, AppError> {
    details(ctx, vec![mailbox])
        .await?
        .into_iter()
        .next()
        .ok_or_else(mailbox_not_found)
}

async fn details(
    ctx: &Ctx<'_>,
    mailboxes: Vec<MailboxView>,
) -> Result<Vec<MailboxDetails>, AppError> {
    let state = ctx.state();
    let ids: Vec<String> = mailboxes
        .iter()
        .map(|mailbox| mailbox.id.to_string())
        .collect();
    let configured = if ids.is_empty() {
        Vec::new()
    } else {
        state
            .secrets
            .configured(ctx.scope, MAILBOX_SECRET_OWNER, &ids)
            .await?
    };
    Ok(mailboxes
        .into_iter()
        .map(|mailbox| {
            let provider_label = crate::store::mail_providers()
                .find(&mailbox.provider)
                .map(|provider| provider.label().to_owned())
                .unwrap_or_else(|| mailbox.provider.clone());
            let id = mailbox.id.to_string();
            MailboxDetails {
                credentials_configured: configured
                    .iter()
                    .any(|key| key.owner_id == id && key.name == MAILBOX_CREDENTIALS_SECRET),
                provider_label,
                mailbox: MailboxView {
                    settings: public_settings(&mailbox.settings),
                    ..mailbox
                },
            }
        })
        .collect())
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    event_type: &'static str,
    mailbox_id: Uuid,
    provider: &str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(MAILBOX_RECORD_TYPE, mailbox_id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "mailbox_id": mailbox_id,
                    "owner_id": ctx.scope.user_id().as_str(),
                    "provider": provider,
                })),
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
