use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::domain::{
    ACTIVITY_TYPE_ACTIVATED, ACTIVITY_TYPE_CREATED, ACTIVITY_TYPE_DEACTIVATED,
    ACTIVITY_TYPE_KEY_TAKEN, ACTIVITY_TYPE_REORDERED, ACTIVITY_TYPE_UPDATED, ActivityTypeView,
    CreateActivityTypeCommand, LAST_ACTIVE_ACTIVITY_TYPE, MAX_ACTIVITY_TYPES,
    UpdateActivityTypeCommand, activity_type_key_from_name, activity_type_not_found,
    normalize_activity_type_name, validate_activity_type_color, validate_activity_type_icon,
    validate_activity_type_key,
};
use crate::store::ActivityTypeStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

#[derive(Debug, Serialize)]
struct ActivityTypeSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    icon: Option<&'a str>,
    color: Option<&'a str>,
    is_active: bool,
}

impl<'a> From<&'a ActivityTypeView> for ActivityTypeSnapshot<'a> {
    fn from(view: &'a ActivityTypeView) -> Self {
        Self {
            key: &view.key,
            name: &view.name,
            icon: view.icon.as_deref(),
            color: view.color.as_deref(),
            is_active: view.is_active,
        }
    }
}

pub fn require_activity_type_reader(scope: &AccessScope) -> Result<(), AppError> {
    if scope.require(&Permission::SETTINGS_MANAGE).is_ok() {
        return Ok(());
    }
    scope.require(&Permission::TASKS_READ).map(|_| ())
}

fn optional_text(
    value: Option<&str>,
    field: &str,
    validate: fn(&str) -> Result<String, &'static str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    match validate(value) {
        Ok(value) => Some(value),
        Err(message) => {
            violations.push(FieldViolation::new(field, message));
            None
        }
    }
}

pub async fn list(
    ctx: &Ctx<'_>,
    include_inactive: bool,
    window: ListWindow,
) -> Result<Listing<ActivityTypeView>, AppError> {
    search(ctx, include_inactive, None, window).await
}

pub(crate) async fn search(
    ctx: &Ctx<'_>,
    include_inactive: bool,
    term: Option<&str>,
    window: ListWindow,
) -> Result<Listing<ActivityTypeView>, AppError> {
    require_activity_type_reader(ctx.scope)?;
    let needle = term
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase);
    let types: Vec<ActivityTypeView> = ActivityTypeStore::of(ctx)
        .list(include_inactive)
        .await?
        .into_iter()
        .filter(|view| {
            needle.as_deref().is_none_or(|needle| {
                view.name.to_lowercase().contains(needle)
                    || view.key.to_lowercase().contains(needle)
            })
        })
        .collect();
    let total = types.len() as u64;
    Ok(Listing {
        items: types
            .into_iter()
            .skip(window.offset as usize)
            .take(window.limit as usize)
            .collect(),
        total,
    })
}

pub async fn get(ctx: &Ctx<'_>, id: &str) -> Result<ActivityTypeView, AppError> {
    require_activity_type_reader(ctx.scope)?;
    require_type(ctx, id).await
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateActivityTypeCommand,
) -> Result<ActivityTypeView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let name = normalize_activity_type_name(&command.name)
        .map_err(|message| violations.push(FieldViolation::new("name", message)))
        .ok();
    let explicit_key = optional_text(
        command.key.as_deref(),
        "key",
        validate_activity_type_key,
        &mut violations,
    );
    let icon = optional_text(
        command.icon.as_deref(),
        "icon",
        validate_activity_type_icon,
        &mut violations,
    );
    let color = optional_text(
        command.color.as_deref(),
        "color",
        validate_activity_type_color,
        &mut violations,
    );
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid activity type",
            violations,
        ));
    }
    let name = name.unwrap_or_default();
    let existing = ActivityTypeStore::of(ctx).list(true).await?;
    if existing.len() >= MAX_ACTIVITY_TYPES {
        return Err(AppError::invalid_fields(
            "invalid activity type",
            vec![FieldViolation::new(
                "name",
                "an organization can have at most 100 activity types",
            )],
        ));
    }
    let key = match explicit_key {
        Some(key) => {
            if ActivityTypeStore::of(ctx).key_exists(&key).await? {
                return Err(AppError::conflict_code(
                    ACTIVITY_TYPE_KEY_TAKEN,
                    "another activity type already uses this key",
                ));
            }
            key
        }
        None => free_key(ctx, &activity_type_key_from_name(&name)).await?,
    };

    let now = ctx.now();
    let view = ActivityTypeView {
        id: Uuid::now_v7(),
        key,
        name,
        icon,
        color,
        is_system: false,
        is_active: true,
        position: existing
            .iter()
            .map(|existing| existing.position)
            .max()
            .map_or(0, |position| position + 1),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &serde_json::Value::Null,
        &ActivityTypeSnapshot::from(&view),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ActivityTypeStore::of(ctx).insert(&mut tx, &view).await?;
    record(
        ctx,
        &mut tx,
        ACTIVITY_TYPE_CREATED,
        Some(&view),
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    require_type(ctx, &view.id.to_string()).await
}

pub async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateActivityTypeCommand,
) -> Result<ActivityTypeView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_type(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut updated = current.clone();
    if let Some(name) = &command.name {
        match normalize_activity_type_name(name) {
            Ok(name) => updated.name = name,
            Err(message) => violations.push(FieldViolation::new("name", message)),
        }
    }
    if let Some(icon) = &command.icon {
        updated.icon = optional_text(
            icon.as_deref(),
            "icon",
            validate_activity_type_icon,
            &mut violations,
        );
    }
    if let Some(color) = &command.color {
        updated.color = optional_text(
            color.as_deref(),
            "color",
            validate_activity_type_color,
            &mut violations,
        );
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid activity type",
            violations,
        ));
    }
    let changes = diff_changes(
        &ActivityTypeSnapshot::from(&current),
        &ActivityTypeSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    ActivityTypeStore::of(ctx).update(&mut tx, &updated).await?;
    record(
        ctx,
        &mut tx,
        ACTIVITY_TYPE_UPDATED,
        Some(&updated),
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    require_type(ctx, id).await
}

pub(crate) async fn set_active(
    ctx: &Ctx<'_>,
    id: &str,
    active: bool,
    ip: Option<String>,
) -> Result<ActivityTypeView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_type(ctx, id).await?;
    if current.is_active == active {
        return Ok(current);
    }
    if !active {
        let active_count = ActivityTypeStore::of(ctx).list(false).await?.len();
        if active_count <= 1 {
            return Err(AppError::conflict_code(
                LAST_ACTIVE_ACTIVITY_TYPE,
                "the organization needs at least one active activity type",
            ));
        }
    }
    let mut updated = current.clone();
    updated.is_active = active;
    updated.updated_at = ctx.now();
    let event = if active {
        ACTIVITY_TYPE_ACTIVATED
    } else {
        ACTIVITY_TYPE_DEACTIVATED
    };
    let mut tx = ctx.begin().await?;
    ActivityTypeStore::of(ctx).update(&mut tx, &updated).await?;
    record(
        ctx,
        &mut tx,
        event,
        Some(&updated),
        vec![FieldChange::new("is_active", json!(!active), json!(active))],
        ip,
    )
    .await?;
    tx.commit().await?;
    require_type(ctx, id).await
}

pub(crate) async fn reorder(
    ctx: &Ctx<'_>,
    ids: &[String],
    ip: Option<String>,
) -> Result<Vec<ActivityTypeView>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let existing = ActivityTypeStore::of(ctx).list(true).await?;
    let invalid = || {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every activity type of the organization exactly once",
            )],
        )
    };
    let ordered = ids
        .iter()
        .map(|id| Uuid::parse_str(id).map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let requested: BTreeSet<Uuid> = ordered.iter().copied().collect();
    let known: BTreeSet<Uuid> = existing.iter().map(|view| view.id).collect();
    if requested.len() != ordered.len() || requested != known {
        return Err(invalid());
    }
    let previous: Vec<String> = existing.iter().map(|view| view.id.to_string()).collect();
    let next: Vec<String> = ordered.iter().map(Uuid::to_string).collect();
    if previous == next {
        return Ok(existing);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    ActivityTypeStore::of(ctx)
        .reorder(&mut tx, &ordered)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), ACTIVITY_TYPE_REORDERED, now)
                .with_payload(json!({ "ids": next })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, ACTIVITY_TYPE_REORDERED, now)
                .with_changes(vec![FieldChange::new(
                    "order",
                    json!(previous),
                    json!(next),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    ActivityTypeStore::of(ctx).list(true).await
}

async fn free_key(ctx: &Ctx<'_>, base: &str) -> Result<String, AppError> {
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while ActivityTypeStore::of(ctx).key_exists(&candidate).await? {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

async fn require_type(ctx: &Ctx<'_>, id: &str) -> Result<ActivityTypeView, AppError> {
    let id = Uuid::parse_str(id).map_err(|_| activity_type_not_found())?;
    ActivityTypeStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(activity_type_not_found)
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    action: &'static str,
    view: Option<&ActivityTypeView>,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut event = DomainEvent::new(actor.clone(), action, now);
    let mut entry = AuditEntry::new(actor, action, now)
        .with_changes(changes)
        .with_ip(ip);
    if let Some(view) = view {
        let record = RecordRef::new(RecordType::ACTIVITY_TYPE, view.id);
        event = event.with_record(record.clone()).with_payload(json!({
            "key": view.key,
            "name": view.name,
            "icon": view.icon,
            "color": view.color,
            "is_active": view.is_active,
        }));
        entry = entry.with_record(record);
    }
    ctx.events().publish(tx, event).await?;
    ctx.audit().record(tx, entry).await
}
