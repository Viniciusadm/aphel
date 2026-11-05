use serde_json::{Value, json};
use uuid::Uuid;

use crate::capture::{
    AddBlockCommand, BlockSnapshot, CaptureSnapshot, EMAIL_BLOCK_EXISTS, EMAIL_BLOCK_RECORD_TYPE,
    EMAIL_BLOCKLIST_ADDED, EMAIL_BLOCKLIST_REMOVED, EMAIL_CAPTURE_SETTINGS_UPDATED,
    MAX_BLOCK_REASON_LENGTH, UpdateCaptureSettingsCommand, block_not_found, normalize_block_value,
};
use crate::domain::{MAX_ADDRESS_LENGTH, validate_history_days};
use crate::store::settings::SettingsStore;
use crate::sync::{BlockKind, BlockedEntry, CaptureSettings};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<CaptureSettings, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    SettingsStore::of(ctx).capture_settings().await
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    command: UpdateCaptureSettingsCommand,
) -> Result<CaptureSettings, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = SettingsStore::of(ctx);
    let current = store.capture_settings().await?;
    let mut next = current;
    if let Some(value) = command.create_contacts {
        next.create_contacts = value;
    }
    if let Some(value) = command.create_companies {
        next.create_companies = value;
    }
    if let Some(value) = command.link_open_deals {
        next.link_open_deals = value;
    }
    if let Some(days) = command.initial_history_days {
        next.initial_history_days = validate_history_days(days).map_err(|message| {
            AppError::invalid_fields(
                "invalid capture settings",
                vec![FieldViolation::new("initial_history_days", message)],
            )
        })?;
    }
    let changes = diff_changes(
        &CaptureSnapshot::from(&current),
        &CaptureSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    store
        .save_capture_settings(&mut tx, ctx.scope, &next, now)
        .await?;
    record(
        ctx,
        &mut tx,
        EMAIL_CAPTURE_SETTINGS_UPDATED,
        None,
        json!({
            "create_contacts": next.create_contacts,
            "create_companies": next.create_companies,
            "link_open_deals": next.link_open_deals,
            "initial_history_days": next.initial_history_days,
        }),
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn blocklist(ctx: &Ctx<'_>) -> Result<Vec<BlockedEntry>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    SettingsStore::of(ctx).blocklist().await
}

pub(crate) async fn add_block(
    ctx: &Ctx<'_>,
    command: AddBlockCommand,
) -> Result<BlockedEntry, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = SettingsStore::of(ctx);
    let mut violations = Vec::new();
    let kind = BlockKind::parse(command.kind.trim());
    if kind.is_none() {
        violations.push(FieldViolation::new("kind", "must be domain or address"));
    }
    let value = kind.and_then(|kind| {
        normalize_block_value(kind, &command.value)
            .map_err(|message| violations.push(FieldViolation::new("value", message)))
            .ok()
    });
    if command.value.chars().count() > MAX_ADDRESS_LENGTH {
        violations.push(FieldViolation::new(
            "value",
            "must have at most 320 characters",
        ));
    }
    let reason = command
        .reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    if reason
        .as_ref()
        .is_some_and(|reason| reason.chars().count() > MAX_BLOCK_REASON_LENGTH)
    {
        violations.push(FieldViolation::new(
            "reason",
            "must have at most 500 characters",
        ));
    }
    let (Some(kind), Some(value), true) = (kind, value, violations.is_empty()) else {
        return Err(AppError::invalid_fields(
            "invalid blocklist entry",
            violations,
        ));
    };
    if store.find_block_by_value(kind, &value).await?.is_some() {
        return Err(AppError::conflict_code(
            EMAIL_BLOCK_EXISTS,
            "this entry is already in the blocklist",
        ));
    }
    let now = ctx.now();
    let entry = BlockedEntry {
        id: Uuid::now_v7(),
        kind,
        value,
        reason,
        created_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &BlockSnapshot::from(&entry),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    store.insert_block(&mut tx, ctx.scope, &entry).await?;
    record(
        ctx,
        &mut tx,
        EMAIL_BLOCKLIST_ADDED,
        Some(RecordRef::new(EMAIL_BLOCK_RECORD_TYPE, entry.id)),
        json!({ "kind": entry.kind.as_str() }),
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(entry)
}

pub(crate) async fn remove_block(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = SettingsStore::of(ctx);
    let id = Uuid::parse_str(id).map_err(|_| block_not_found())?;
    let entry = store.find_block(id).await?.ok_or_else(block_not_found)?;
    let changes = diff_changes(
        &BlockSnapshot::from(&entry),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    if !store.delete_block(&mut tx, id).await? {
        return Err(block_not_found());
    }
    record(
        ctx,
        &mut tx,
        EMAIL_BLOCKLIST_REMOVED,
        Some(RecordRef::new(EMAIL_BLOCK_RECORD_TYPE, entry.id)),
        json!({ "kind": entry.kind.as_str() }),
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    event_type: &'static str,
    record: Option<RecordRef>,
    payload: Value,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut event = DomainEvent::new(actor.clone(), event_type, now).with_payload(payload);
    let mut entry = AuditEntry::new(actor, event_type, now)
        .with_changes(changes)
        .with_ip(ip);
    if let Some(record) = record {
        event = event.with_record(record.clone());
        entry = entry.with_record(record);
    }
    ctx.events().publish(tx, event).await?;
    ctx.audit().record(tx, entry).await
}
