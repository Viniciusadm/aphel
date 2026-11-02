use aphel_contract_crm::traits;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEFAULT_RETENTION_DAYS, MAX_RETENTION_DAYS, MIN_RETENTION_DAYS, PURGE_BATCH_SIZE,
    TRASH_ITEM_NOT_FOUND, TRASH_SETTINGS_UPDATED, TrashEntry, TrashItem, TrashQuery, TrashSettings,
    expires_at, is_valid_retention, purge_threshold, purged_event, restored_event, trashable_type,
    trashable_types,
};
use crate::store::{TrashHidden, TrashSource, TrashStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldWriter, EntityDefinition, FieldSet, RecordPipeline, RecordWrite,
    WriteMode,
};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::permission_denied;

pub(crate) fn trash_item_not_found() -> AppError {
    AppError::not_found_code(TRASH_ITEM_NOT_FOUND, "the record is not in the trash")
}

fn instant(value: DateTime<Utc>) -> Value {
    json!(value.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
}

fn parse_type(ctx: &Ctx<'_>, value: &str) -> Result<EntityDefinition, AppError> {
    trashable_type(ctx.registry(), value)
        .cloned()
        .ok_or_else(|| {
            AppError::invalid_fields(
                "invalid record type",
                vec![FieldViolation::new(
                    "record_type",
                    "is not a record type with a trash",
                )],
            )
        })
}

async fn source(ctx: &Ctx<'_>, definition: EntityDefinition) -> Result<TrashSource, AppError> {
    let capabilities = &definition.capabilities;
    let contact_columns = capabilities.trait_columns(traits::CONTACT_POINTS);
    let candidates: Vec<String> = if contact_columns.is_empty() {
        capabilities
            .label_aliases
            .iter()
            .map(|alias| alias.column.clone())
            .collect()
    } else {
        contact_columns
    };
    if candidates.is_empty() {
        return Ok(TrashSource::new(definition));
    }
    let fields = FieldSet::for_scope(ctx.fields().definitions(), &definition, ctx.scope).await?;
    let visible = candidates
        .into_iter()
        .filter(|column| {
            definition
                .system_fields
                .iter()
                .find(|field| field.column == *column && !field.is_computed())
                .is_some_and(|field| fields.find_active(field.key.as_str()).is_some())
        })
        .collect();
    Ok(TrashSource::new(definition).with_subtitles(visible))
}

struct TrashMutation {
    actor: Actor,
    event_type: String,
    record: RecordRef,
    payload: Value,
    changes: Vec<FieldChange>,
    now: DateTime<Utc>,
    ip: Option<String>,
}

async fn publish(ctx: &Ctx<'_>, tx: &mut MySqlTx, mutation: TrashMutation) -> Result<(), AppError> {
    let TrashMutation {
        actor,
        event_type,
        record,
        payload,
        changes,
        now,
        ip,
    } = mutation;
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type.clone(), now)
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

async fn retention(ctx: &Ctx<'_>) -> Result<u32, AppError> {
    Ok(TrashStore::of(ctx)
        .retention_days()
        .await?
        .unwrap_or(DEFAULT_RETENTION_DAYS))
}

fn delete_permission(definition: &EntityDefinition) -> Permission {
    definition
        .action_permission("delete")
        .or_else(|| definition.update_permission.clone())
        .unwrap_or_else(|| definition.read_permission.clone())
}

fn readable_types(ctx: &Ctx<'_>) -> Vec<(EntityDefinition, RecordScopeFilter)> {
    trashable_types(ctx.registry())
        .into_iter()
        .filter_map(|definition| {
            let visibility = ctx.scope.record_filter(&definition.read_permission).ok()?;
            Some((definition, visibility))
        })
        .collect()
}

pub(crate) fn types(ctx: &Ctx<'_>) -> Result<Vec<EntityDefinition>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    Ok(readable_types(ctx)
        .into_iter()
        .map(|(definition, _)| definition)
        .collect())
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: TrashQuery,
    window: ListWindow,
) -> Result<Listing<TrashEntry>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let definitions = match query.record_type.as_deref().map(str::trim) {
        None | Some("") => readable_types(ctx),
        Some(value) => {
            let definition = parse_type(ctx, value)?;
            let visibility = ctx.scope.record_filter(&definition.read_permission)?;
            vec![(definition, visibility)]
        }
    };
    let mut sources = Vec::with_capacity(definitions.len());
    for (definition, visibility) in definitions {
        sources.push(source(ctx, definition).await?.with_visibility(visibility));
    }
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| term.chars().take(100).collect::<String>());
    let retention = retention(ctx).await?;
    let listing = TrashStore::of(ctx)
        .list(ctx.scope, &sources, search.as_deref(), window)
        .await?;
    Ok(Listing {
        items: listing
            .items
            .into_iter()
            .map(|item| TrashEntry {
                expires_at: expires_at(item.deleted_at, retention),
                item,
            })
            .collect(),
        total: listing.total,
    })
}

async fn item(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<(EntityDefinition, TrashItem), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let definition = parse_type(ctx, record_type)?;
    let id = Uuid::parse_str(record_id.trim()).map_err(|_| trash_item_not_found())?;
    let readable = scope.record_filter(&definition.read_permission)?;
    let store = TrashStore::of(ctx);
    let item = store
        .find(
            scope,
            &TrashSource::new(definition.clone()).with_visibility(readable),
            id,
            TrashHidden::Excluded,
        )
        .await?
        .ok_or_else(trash_item_not_found)?;
    let deletable = scope.record_filter(&delete_permission(&definition))?;
    if deletable != RecordScopeFilter::Organization
        && store
            .find(
                scope,
                &TrashSource::new(definition.clone()).with_visibility(deletable),
                id,
                TrashHidden::Excluded,
            )
            .await?
            .is_none()
    {
        return Err(permission_denied());
    }
    Ok((definition, item))
}

pub(crate) async fn restore(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    ip: Option<String>,
) -> Result<TrashItem, AppError> {
    let (definition, item) = item(ctx, record_type, record_id).await?;
    let now = ctx.now();
    let record = RecordRef::new(item.record_type.clone(), item.id);
    let changes = vec![FieldChange::new(
        "deleted_at",
        instant(item.deleted_at),
        Value::Null,
    )];
    let values = json!({});
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    if !TrashStore::of(ctx)
        .restore(&mut tx, &definition, item.id)
        .await?
    {
        return Err(trash_item_not_found());
    }
    pipeline.validate(&mut tx, ctx.scope, &write).await?;
    pipeline.restored(&mut tx, ctx.scope, &record).await?;
    pipeline.saved(&mut tx, ctx.scope, &write).await?;
    publish(
        ctx,
        &mut tx,
        TrashMutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: restored_event(item.record_type.as_str()),
            record: record.clone(),
            payload: json!({ "name": item.title }),
            changes: changes.clone(),
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(ctx.scope, &write).await;
    Ok(item)
}

pub(crate) async fn purge(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let (definition, item) = item(ctx, record_type, record_id).await?;
    if purge_item(ctx, Actor::from_scope(ctx.scope), &definition, &item, ip).await? {
        Ok(())
    } else {
        Err(trash_item_not_found())
    }
}

async fn purge_item(
    ctx: &Ctx<'_>,
    actor: Actor,
    definition: &EntityDefinition,
    item: &TrashItem,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let now = ctx.now();
    let record = RecordRef::new(item.record_type.clone(), item.id);
    let fields = ctx.fields();
    let mut tx = ctx.begin().await?;
    let purged = CustomFieldWriter::new(&fields)
        .purge_record(&mut tx, ctx.scope, &record)
        .await?;
    if !TrashStore::of(ctx)
        .purge(&mut tx, definition, item.id)
        .await?
    {
        return Ok(false);
    }
    publish(
        ctx,
        &mut tx,
        TrashMutation {
            actor,
            event_type: purged_event(item.record_type.as_str()),
            record,
            payload: json!({ "name": item.title, "deleted_at": instant(item.deleted_at) }),
            changes: vec![FieldChange::new(
                "record",
                json!({ "name": item.title }),
                Value::Null,
            )],
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    remove_stored_files(ctx, &purged.orphan_storage_keys).await;
    Ok(true)
}

async fn remove_stored_files(ctx: &Ctx<'_>, storage_keys: &[String]) {
    if storage_keys.is_empty() {
        return;
    }
    let storage = match ctx.storage() {
        Ok(storage) => storage,
        Err(error) => {
            tracing::warn!(%error, files = storage_keys.len(), "purged record files left in storage");
            return;
        }
    };
    for key in storage_keys {
        if let Err(error) = storage.delete(key).await {
            tracing::warn!(%error, storage_key = %key, "could not remove a purged record file");
        }
    }
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<TrashSettings, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    Ok(TrashSettings {
        retention_days: retention(ctx).await?,
    })
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    retention_days: u32,
    ip: Option<String>,
) -> Result<TrashSettings, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    if !is_valid_retention(retention_days) {
        return Err(AppError::invalid_fields(
            "invalid trash settings",
            vec![FieldViolation::new(
                "retention_days",
                format!("must be between {MIN_RETENTION_DAYS} and {MAX_RETENTION_DAYS}"),
            )],
        ));
    }
    let current = retention(ctx).await?;
    if current == retention_days {
        return Ok(TrashSettings { retention_days });
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    TrashStore::of(ctx)
        .save_retention(&mut tx, retention_days)
        .await?;
    publish(
        ctx,
        &mut tx,
        TrashMutation {
            actor: Actor::from_scope(ctx.scope),
            event_type: TRASH_SETTINGS_UPDATED.to_owned(),
            record: RecordRef::organization(),
            payload: json!({ "retention_days": retention_days }),
            changes: vec![FieldChange::new(
                "retention_days",
                json!(current),
                json!(retention_days),
            )],
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(TrashSettings { retention_days })
}

pub(crate) async fn purge_expired(ctx: &Ctx<'_>) -> Result<usize, AppError> {
    let retention = retention(ctx).await?;
    let threshold = purge_threshold(ctx.now(), retention);
    let store = TrashStore::of(ctx);
    let mut purged = 0;
    for definition in trashable_types(ctx.registry()) {
        let source = TrashSource::new(definition.clone());
        let mut failed: Vec<Uuid> = Vec::new();
        loop {
            let ids = store
                .expired(&definition, threshold, PURGE_BATCH_SIZE, &failed)
                .await?;
            let mut progressed = false;
            for id in &ids {
                let Some(item) = store
                    .find(ctx.scope, &source, *id, TrashHidden::Included)
                    .await?
                else {
                    continue;
                };
                match purge_item(ctx, Actor::System, &definition, &item, None).await {
                    Ok(true) => {
                        purged += 1;
                        progressed = true;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            record_type = definition.record_type.as_str(),
                            record_id = %id,
                            "expired trash record could not be purged"
                        );
                        failed.push(*id);
                    }
                }
            }
            if ids.len() < PURGE_BATCH_SIZE as usize || !progressed {
                break;
            }
        }
    }
    Ok(purged)
}
