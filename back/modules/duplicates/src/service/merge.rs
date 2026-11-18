use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{DuplicateFamily, MergeSide};
use crate::service::{field_set, visible_ids};
use crate::store::merges::RecordVersions;
use crate::store::{DuplicateStore, MergeStore};
use crate::{
    CandidateResolution, DuplicateType, MERGE_SAME_RECORD, MergeCommand, MergeOutcome,
    deleted_event, merged_event, parse_duplicate_type, record_not_found, updated_event,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{changes_payload, permission_denied};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_values};
use aphel_sdk::platform::application::fields::{
    CustomFieldMap, CustomFieldReader, CustomFieldWriter, FieldSet, RecordPipeline, RecordWrite,
    WriteMode,
};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, RecordScopeFilter};
const STAGE_FIELD: &str = "lifecycle_stage";

pub(crate) fn system_merge_keys(ctx: &Ctx<'_>, kind: &DuplicateType) -> Vec<String> {
    if let Some(owner) = aphel_sdk::merges::owner(ctx, kind.as_str()) {
        return owner.fields().iter().map(|key| (*key).to_owned()).collect();
    }
    kind.generic_merge_fields()
        .into_iter()
        .map(|field| field.key.as_str().to_owned())
        .collect()
}

fn not_found(ctx: &Ctx<'_>, kind: &DuplicateType) -> AppError {
    aphel_sdk::merges::owner(ctx, kind.as_str())
        .map_or_else(record_not_found, |owner| owner.not_found())
}

fn tracks_lifecycle(ctx: &Ctx<'_>, kind: &DuplicateType) -> bool {
    kind.family != DuplicateFamily::Generic && aphel_sdk::leads::merge_history(ctx).is_some()
}

async fn lifecycle_stage(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    kind: &DuplicateType,
    id: Uuid,
) -> Result<Option<Option<String>>, AppError> {
    if !tracks_lifecycle(ctx, kind) {
        return Ok(None);
    }
    match aphel_sdk::leads::merge_history(ctx) {
        Some(source) => source.stage(ctx, tx, kind.record_type(), id).await,
        None => Ok(None),
    }
}

fn redact_hidden(changes: Vec<FieldChange>, fields: &FieldSet) -> Vec<FieldChange> {
    changes
        .into_iter()
        .map(|change| {
            if fields.is_hidden(&change.field) {
                FieldChange::redacted(change.field)
            } else {
                change
            }
        })
        .collect()
}

struct CompletedMerge {
    kind: DuplicateType,
    primary_id: Uuid,
    duplicate_id: Uuid,
    duplicate_name: String,
    values: Value,
    changes: Vec<FieldChange>,
    now: DateTime<Utc>,
    ip: Option<String>,
    primary_stage: Option<Option<String>>,
    moved: BTreeMap<String, u64>,
}

struct MergePlan {
    choices: BTreeMap<String, MergeSide>,
    custom_updates: Map<String, Value>,
    protected_updates: Map<String, Value>,
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

fn plan_merge(
    command_fields: &BTreeMap<String, String>,
    system_keys: &[String],
    fields: &FieldSet,
    primary_custom: &CustomFieldMap,
    duplicate_custom: &CustomFieldMap,
) -> Result<MergePlan, AppError> {
    let custom_choosable = |key: &str| {
        fields.find(key).is_some_and(|field| {
            !field.is_system()
                && field.definition.is_active
                && !field.is_hidden()
                && field.is_editable()
        })
    };
    let mut violations = Vec::new();
    let mut choices = BTreeMap::new();
    for (key, side) in command_fields {
        let known = (system_keys.iter().any(|system| system == key)
            && !fields.is_hidden(key)
            && fields.find(key).is_none_or(|field| field.is_editable()))
            || custom_choosable(key);
        let Some(side) = MergeSide::parse(side) else {
            violations.push(FieldViolation::new(
                format!("fields.{key}"),
                "must be primary or duplicate",
            ));
            continue;
        };
        if !known {
            violations.push(FieldViolation::new(
                format!("fields.{key}"),
                "is not a mergeable field",
            ));
            continue;
        }
        choices.insert(key.clone(), side);
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid merge", violations));
    }
    let keys: BTreeSet<&String> = primary_custom
        .keys()
        .chain(duplicate_custom.keys())
        .collect();
    let mut custom_updates = Map::new();
    let mut protected_updates = Map::new();
    for key in keys {
        let Some(field) = fields.find(key) else {
            continue;
        };
        if field.is_system() || !field.definition.is_active || field.definition.is_formula() {
            continue;
        }
        let primary_value = primary_custom.get(key).cloned().unwrap_or(Value::Null);
        let duplicate_value = duplicate_custom.get(key).cloned().unwrap_or(Value::Null);
        if !custom_choosable(key) {
            if is_empty_value(&primary_value) && !is_empty_value(&duplicate_value) {
                protected_updates.insert(key.clone(), duplicate_value);
            }
            continue;
        }
        let side = choices
            .get(key)
            .copied()
            .unwrap_or(if is_empty_value(&primary_value) {
                MergeSide::Duplicate
            } else {
                MergeSide::Primary
            });
        if side == MergeSide::Duplicate && duplicate_value != primary_value {
            if is_empty_value(&duplicate_value) && !choices.contains_key(key) {
                continue;
            }
            custom_updates.insert(key.clone(), duplicate_value);
        }
    }
    Ok(MergePlan {
        choices,
        custom_updates,
        protected_updates,
    })
}

fn side_for(
    choices: &BTreeMap<String, MergeSide>,
    key: &str,
    primary_empty: bool,
    duplicate_empty: bool,
) -> MergeSide {
    choices
        .get(key)
        .copied()
        .unwrap_or(if primary_empty && !duplicate_empty {
            MergeSide::Duplicate
        } else {
            MergeSide::Primary
        })
}

pub(crate) async fn merge(ctx: &Ctx<'_>, command: MergeCommand) -> Result<MergeOutcome, AppError> {
    let scope = ctx.scope;
    let kind = parse_duplicate_type(ctx.registry(), &command.record_type)?;
    scope.require(&kind.update)?;
    scope.require(&kind.delete)?;
    let primary_id =
        Uuid::parse_str(command.primary_id.trim()).map_err(|_| not_found(ctx, &kind))?;
    let duplicate_id =
        Uuid::parse_str(command.duplicate_id.trim()).map_err(|_| not_found(ctx, &kind))?;
    if primary_id == duplicate_id {
        return Err(AppError::validation_code(
            MERGE_SAME_RECORD,
            "a record cannot be merged into itself",
        ));
    }
    let fields = field_set(ctx, &kind).await?;
    let ids = (primary_id, duplicate_id);
    let versions = MergeStore::of(ctx)
        .versions(&kind, &[primary_id, duplicate_id])
        .await?;
    if let Some(Some(current)) = versions.get(&primary_id) {
        ensure_version(command.expected_version, current.and_utc())?;
    }
    let read = Read {
        fields: &fields,
        versions: &versions,
        expected: command.expected_version,
    };
    if aphel_sdk::merges::owner(ctx, kind.as_str()).is_some() {
        merge_owned(ctx, kind, ids, read, command).await
    } else if kind.definition.is_custom() {
        merge_generic(ctx, kind, ids, read, command).await
    } else {
        Err(AppError::Validation(
            "this record owner does not support merging".into(),
        ))
    }
}

#[derive(Clone, Copy)]
struct Read<'a> {
    fields: &'a FieldSet,
    versions: &'a RecordVersions,
    expected: Option<DateTime<Utc>>,
}

async fn lock_pair(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    kind: &DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
) -> Result<(), AppError> {
    let current = MergeStore::of(ctx)
        .lock_versions(tx, kind, &[primary_id, duplicate_id])
        .await?;
    for id in [primary_id, duplicate_id] {
        let (Some(before), Some(now)) = (read.versions.get(&id), current.get(&id)) else {
            return Err(not_found(ctx, kind));
        };
        if let (Some(before), Some(now)) = (before, now) {
            ensure_version(Some(before.and_utc()), now.and_utc())?;
        }
    }
    if let Some(Some(now)) = current.get(&primary_id) {
        ensure_version(read.expected, now.and_utc())?;
    }
    Ok(())
}

async fn load_custom(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
    primary_id: Uuid,
    duplicate_id: Uuid,
) -> Result<(CustomFieldMap, CustomFieldMap), AppError> {
    let fields = ctx.fields();
    let mut values: HashMap<Uuid, CustomFieldMap> = CustomFieldReader::new(&fields)
        .load_unrestricted(kind.record_type(), &[primary_id, duplicate_id])
        .await?;
    Ok((
        values.remove(&primary_id).unwrap_or_default(),
        values.remove(&duplicate_id).unwrap_or_default(),
    ))
}

async fn apply_custom(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    kind: &DuplicateType,
    primary_id: Uuid,
    snapshot: &Value,
    plan: &MergePlan,
) -> Result<Vec<FieldChange>, AppError> {
    let fields = ctx.fields();
    let writer = CustomFieldWriter::new(&fields).with_record_values(snapshot);
    let survivor = RecordRef::new(kind.record_type().clone(), primary_id);
    let mut changes = Vec::new();
    if !plan.custom_updates.is_empty() {
        changes.extend(
            writer
                .apply(
                    tx,
                    ctx.scope,
                    &survivor,
                    &plan.custom_updates,
                    WriteMode::Update,
                )
                .await?,
        );
    }
    if !plan.protected_updates.is_empty() {
        let system = AccessScope::system();
        let protected = writer
            .apply(
                tx,
                &system,
                &survivor,
                &plan.protected_updates,
                WriteMode::Update,
            )
            .await?;
        changes.extend(
            protected
                .into_iter()
                .map(|change| FieldChange::redacted(change.field)),
        );
    }
    Ok(changes)
}

async fn retire_duplicate(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    kind: &DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    now: DateTime<Utc>,
) -> Result<BTreeMap<String, u64>, AppError> {
    let scope = ctx.scope;
    let merged = RecordRef::new(kind.record_type().clone(), duplicate_id);
    let pipeline = ctx.pipeline();
    let merges = MergeStore::of(ctx);
    let moved = merges
        .move_links(ctx, tx, kind, duplicate_id, primary_id)
        .await?;
    merges
        .record_merge(tx, scope, kind.as_str(), duplicate_id, primary_id, now)
        .await?;
    DuplicateStore::of(ctx)
        .resolve_merged(
            tx,
            kind.as_str(),
            duplicate_id,
            primary_id,
            &CandidateResolution {
                resolved_by: scope.user_id().clone(),
                resolved_at: now,
            },
        )
        .await?;
    pipeline.deleting(tx, scope, &merged).await?;
    if let Some(owner) = aphel_sdk::merges::owner(ctx, kind.as_str()) {
        owner.retire(ctx, tx, duplicate_id, now).await?;
    } else {
        aphel_sdk::records::retire_custom(tx, &kind.definition, duplicate_id, now).await?;
    }
    pipeline.deleted(tx, scope, &merged).await?;
    Ok(moved)
}

async fn finish_merge(
    ctx: &Ctx<'_>,
    mut tx: Tx,
    merge: CompletedMerge,
) -> Result<MergeOutcome, AppError> {
    let scope = ctx.scope;
    let CompletedMerge {
        kind,
        primary_id,
        duplicate_id,
        duplicate_name,
        values,
        changes,
        now,
        ip,
        primary_stage,
        moved,
    } = merge;
    let record_kind = kind.record_type().clone();
    let survivor = RecordRef::new(record_kind.clone(), primary_id);
    let merged = RecordRef::new(record_kind.clone(), duplicate_id);
    let pipeline = ctx.pipeline();
    let write = RecordWrite {
        record: &survivor,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    if !changes.is_empty() {
        pipeline.validate(&mut tx, scope, &write).await?;
        pipeline.saved(&mut tx, scope, &write).await?;
    }
    if let Some(before) = primary_stage
        && write.changed(STAGE_FIELD).is_none()
        && let Some(after) = lifecycle_stage(ctx, &mut tx, &kind, primary_id).await?
        && let Some(source) = aphel_sdk::leads::merge_history(ctx)
    {
        source
            .changed(ctx, &mut tx, &survivor, before, after)
            .await?;
    }
    let actor = Actor::from_scope(scope);
    if !changes.is_empty() {
        publish(
            ctx,
            &mut tx,
            Published {
                actor: actor.clone(),
                event_type: updated_event(kind.as_str()),
                record: survivor.clone(),
                payload: changes_payload(&changes),
                changes: changes.clone(),
                now,
                ip: ip.clone(),
            },
        )
        .await?;
    }
    publish(
        ctx,
        &mut tx,
        Published {
            actor: actor.clone(),
            event_type: deleted_event(kind.as_str()),
            record: merged.clone(),
            payload: json!({
                "name": duplicate_name,
                "merged_into": primary_id.to_string(),
            }),
            changes: vec![FieldChange::new(
                "merged_into",
                Value::Null,
                json!(primary_id.to_string()),
            )],
            now,
            ip: ip.clone(),
        },
    )
    .await?;
    let mut survivor_changes = changes.clone();
    survivor_changes.push(FieldChange::new(
        "merged_id",
        Value::Null,
        json!(duplicate_id.to_string()),
    ));
    publish(
        ctx,
        &mut tx,
        Published {
            actor,
            event_type: merged_event(kind.as_str()),
            record: survivor.clone(),
            payload: json!({
                "survivor_id": primary_id.to_string(),
                "merged_id": duplicate_id.to_string(),
                "merged_name": duplicate_name,
                "moved": moved,
                "changes": changes
                    .iter()
                    .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
                    .collect::<Vec<_>>(),
            }),
            changes: survivor_changes,
            now,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    if !changes.is_empty() {
        pipeline.committed(scope, &write).await;
    }
    Ok(MergeOutcome {
        record_type: record_kind,
        survivor_id: primary_id,
        merged_id: duplicate_id,
        moved,
    })
}

struct Published {
    actor: Actor,
    event_type: String,
    record: RecordRef,
    payload: Value,
    changes: Vec<FieldChange>,
    now: DateTime<Utc>,
    ip: Option<String>,
}

async fn publish(ctx: &Ctx<'_>, tx: &mut Tx, published: Published) -> Result<(), AppError> {
    let Published {
        actor,
        event_type,
        record,
        payload,
        changes,
        now,
        ip,
    } = published;
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

async fn merge_owned(
    ctx: &Ctx<'_>,
    kind: DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
    command: MergeCommand,
) -> Result<MergeOutcome, AppError> {
    let owner = aphel_sdk::merges::owner(ctx, kind.as_str()).ok_or_else(record_not_found)?;
    let fields = read.fields;
    let (primary_custom, duplicate_custom) =
        load_custom(ctx, &kind, primary_id, duplicate_id).await?;
    let plan = plan_merge(
        &command.fields,
        &system_merge_keys(ctx, &kind),
        fields,
        &primary_custom,
        &duplicate_custom,
    )?;
    let choices = plan
        .choices
        .iter()
        .map(|(key, side)| (key.clone(), *side == MergeSide::Duplicate))
        .collect();
    let mut prepared = owner
        .prepare(ctx, primary_id, duplicate_id, &choices)
        .await?;
    let before = prepared.before();
    let duplicate_name = prepared.duplicate_name();
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    lock_pair(ctx, &mut tx, &kind, (primary_id, duplicate_id), read).await?;
    let primary_stage = lifecycle_stage(ctx, &mut tx, &kind, primary_id).await?;
    let snapshot = prepared
        .apply(ctx, &mut tx, now, command.ip.clone())
        .await?;
    let moved = retire_duplicate(ctx, &mut tx, &kind, (primary_id, duplicate_id), now).await?;
    let custom = apply_custom(ctx, &mut tx, &kind, primary_id, &snapshot, &plan).await?;
    prepared.persist(ctx, &mut tx, now).await?;
    let mut changes = redact_hidden(
        diff_values(&before, &snapshot, &DiffOptions::default())?,
        fields,
    );
    changes.extend(custom);
    finish_merge(
        ctx,
        tx,
        CompletedMerge {
            kind,
            primary_id,
            duplicate_id,
            duplicate_name,
            values: snapshot,
            changes,
            now,
            ip: command.ip,
            primary_stage,
            moved,
        },
    )
    .await
}

async fn ensure_allowed(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
    id: Uuid,
    permission: &Permission,
) -> Result<(), AppError> {
    let read = ctx.scope.record_filter(&kind.read)?;
    if visible_ids(ctx, kind, &read, &[id]).await?.is_empty() {
        return Err(not_found(ctx, kind));
    }
    let allowed = ctx.scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && visible_ids(ctx, kind, &allowed, &[id]).await?.is_empty()
    {
        return Err(permission_denied());
    }
    Ok(())
}

async fn merge_generic(
    ctx: &Ctx<'_>,
    kind: DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
    command: MergeCommand,
) -> Result<MergeOutcome, AppError> {
    let fields = read.fields;
    ensure_allowed(ctx, &kind, primary_id, &kind.update).await?;
    ensure_allowed(ctx, &kind, duplicate_id, &kind.delete).await?;
    let merges = MergeStore::of(ctx);
    let system_fields = kind.generic_merge_fields();
    let system_keys = system_merge_keys(ctx, &kind);
    let primary = merges
        .system_values(&kind, &system_fields, primary_id)
        .await?
        .ok_or_else(|| not_found(ctx, &kind))?;
    let duplicate = merges
        .system_values(&kind, &system_fields, duplicate_id)
        .await?
        .ok_or_else(|| not_found(ctx, &kind))?;
    let (primary_custom, duplicate_custom) =
        load_custom(ctx, &kind, primary_id, duplicate_id).await?;
    let plan = plan_merge(
        &command.fields,
        &system_keys,
        fields,
        &primary_custom,
        &duplicate_custom,
    )?;

    let mut merged = primary.clone();
    let mut copied = Vec::new();
    for field in &system_fields {
        let key = field.key.as_str();
        let primary_value = primary.get(key).unwrap_or(&Value::Null);
        let duplicate_value = duplicate.get(key).unwrap_or(&Value::Null);
        let side = side_for(
            &plan.choices,
            key,
            is_empty_value(primary_value),
            is_empty_value(duplicate_value),
        );
        if side == MergeSide::Duplicate && primary_value != duplicate_value {
            merged.insert(key.to_owned(), duplicate_value.clone());
            copied.push(field.column.clone());
        }
    }
    if let Some(title) = kind.definition.title_field.as_deref()
        && merged.get(title).is_none_or(is_empty_value)
        && let Some(value) = primary.get(title)
    {
        merged.insert(title.to_owned(), value.clone());
        let title_column = kind.title_column().to_owned();
        copied.retain(|column| *column != title_column);
    }

    let now = ctx.now();
    let snapshot = Value::Object(merged.clone());
    let mut tx = ctx.begin().await?;
    lock_pair(ctx, &mut tx, &kind, (primary_id, duplicate_id), read).await?;
    let primary_stage = lifecycle_stage(ctx, &mut tx, &kind, primary_id).await?;
    aphel_sdk::records::copy_custom_columns(
        &mut tx,
        &kind.definition,
        duplicate_id,
        primary_id,
        &copied,
        now,
    )
    .await?;
    let moved = retire_duplicate(ctx, &mut tx, &kind, (primary_id, duplicate_id), now).await?;
    let custom = apply_custom(ctx, &mut tx, &kind, primary_id, &snapshot, &plan).await?;
    let mut changes = redact_hidden(
        diff_values(
            &Value::Object(primary.clone()),
            &snapshot,
            &DiffOptions::default(),
        )?,
        fields,
    );
    changes.extend(custom);
    let duplicate_name = kind
        .definition
        .title_field
        .as_deref()
        .and_then(|title| duplicate.get(title))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    finish_merge(
        ctx,
        tx,
        CompletedMerge {
            kind,
            primary_id,
            duplicate_id,
            duplicate_name,
            values: snapshot,
            changes,
            now,
            ip: command.ip,
            primary_stage,
            moved,
        },
    )
    .await
}
