use aphel_contract_crm::CrmPermissions;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{DuplicateFamily, MergeSide};
use crate::service::{field_set, visible_ids};
use crate::store::merges::RecordVersions;
use crate::store::{DuplicateStore, MergeStore};
use crate::{
    COMPANY_MERGE_KEYS, CONTACT_MERGE_KEYS, CandidateResolution, DuplicateType, MERGE_SAME_RECORD,
    MergeCommand, MergeOutcome, company_field_is_empty, contact_field_is_empty, copy_company_field,
    copy_contact_field, deleted_event, merged_event, parse_duplicate_type, record_not_found,
    updated_event,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::{
    CustomFieldMap, CustomFieldReader, CustomFieldWriter, FieldSet, RecordPipeline, RecordWrite,
    WriteMode,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{changes_payload, permission_denied};
use companies::domain::{CompanyView, company_not_found, company_snapshot};
use companies::hierarchy::{HierarchyStore, Reparent, blocked_parents, reparent_children};
use companies::store::CompanyStore;
use contacts::domain::{ContactView, NewContactCompanyLink, contact_not_found, contact_snapshot};
use contacts::store::ContactStore;
use leads::service::record_stage_history;
use leads::store::LeadRecords;

const STAGE_FIELD: &str = "lifecycle_stage";

pub(crate) fn system_merge_keys(kind: &DuplicateType) -> Vec<String> {
    match kind.family {
        DuplicateFamily::Contact => CONTACT_MERGE_KEYS
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        DuplicateFamily::Company => COMPANY_MERGE_KEYS
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        DuplicateFamily::Generic => kind
            .generic_merge_fields()
            .into_iter()
            .map(|field| field.key.as_str().to_owned())
            .collect(),
    }
}

fn not_found(kind: &DuplicateType) -> AppError {
    match kind.family {
        DuplicateFamily::Contact => contact_not_found(),
        DuplicateFamily::Company => company_not_found(),
        DuplicateFamily::Generic => record_not_found(),
    }
}

fn tracks_lifecycle(ctx: &Ctx<'_>, kind: &DuplicateType) -> bool {
    kind.family != DuplicateFamily::Generic && ctx.app().is_enabled(leads::KEY)
}

async fn lifecycle_stage(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    kind: &DuplicateType,
    id: Uuid,
) -> Result<Option<Option<String>>, AppError> {
    if !tracks_lifecycle(ctx, kind) {
        return Ok(None);
    }
    LeadRecords::of(ctx)
        .current_stage_in(tx, kind.record_type(), id)
        .await
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
    let primary_id = Uuid::parse_str(command.primary_id.trim()).map_err(|_| not_found(&kind))?;
    let duplicate_id =
        Uuid::parse_str(command.duplicate_id.trim()).map_err(|_| not_found(&kind))?;
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
    match kind.family {
        DuplicateFamily::Contact => merge_contacts(ctx, kind, ids, read, command).await,
        DuplicateFamily::Company => merge_companies(ctx, kind, ids, read, command).await,
        DuplicateFamily::Generic => merge_generic(ctx, kind, ids, read, command).await,
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
    tx: &mut MySqlTx,
    kind: &DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
) -> Result<(), AppError> {
    let current = MergeStore::of(ctx)
        .lock_versions(tx, kind, &[primary_id, duplicate_id])
        .await?;
    for id in [primary_id, duplicate_id] {
        let (Some(before), Some(now)) = (read.versions.get(&id), current.get(&id)) else {
            return Err(not_found(kind));
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

async fn contact_for(
    ctx: &Ctx<'_>,
    contacts: &ContactStore,
    id: Uuid,
    permission: &Permission,
) -> Result<ContactView, AppError> {
    let scope = ctx.scope;
    let read = scope.record_filter(&Permission::CONTACTS_READ)?;
    let view = contacts
        .find(scope, &read, id)
        .await?
        .ok_or_else(contact_not_found)?;
    let allowed = scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && contacts.find(scope, &allowed, id).await?.is_none()
    {
        return Err(permission_denied());
    }
    Ok(view)
}

async fn company_for(
    ctx: &Ctx<'_>,
    companies: &CompanyStore,
    id: Uuid,
    permission: &Permission,
) -> Result<CompanyView, AppError> {
    let scope = ctx.scope;
    let read = scope.record_filter(&Permission::COMPANIES_READ)?;
    let view = companies
        .find(scope, &read, id)
        .await?
        .ok_or_else(company_not_found)?;
    let allowed = scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && companies.find(scope, &allowed, id).await?.is_none()
    {
        return Err(permission_denied());
    }
    Ok(view)
}

async fn apply_custom(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
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
    tx: &mut MySqlTx,
    kind: &DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    now: DateTime<Utc>,
) -> Result<BTreeMap<String, u64>, AppError> {
    let scope = ctx.scope;
    let merged = RecordRef::new(kind.record_type().clone(), duplicate_id);
    let pipeline = ctx.pipeline();
    let merges = MergeStore::of(ctx);
    let moved = merges
        .move_links(tx, kind, ctx.registry(), duplicate_id, primary_id)
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
    match kind.family {
        DuplicateFamily::Contact => {
            ContactStore::of(ctx)
                .soft_delete(tx, duplicate_id, now)
                .await?
        }
        DuplicateFamily::Company => {
            CompanyStore::of(ctx)
                .soft_delete(tx, duplicate_id, now)
                .await?
        }
        DuplicateFamily::Generic => merges.soft_delete(tx, kind, duplicate_id, now).await?,
    }
    pipeline.deleted(tx, scope, &merged).await?;
    Ok(moved)
}

async fn finish_merge(
    ctx: &Ctx<'_>,
    mut tx: MySqlTx,
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
    {
        record_stage_history(ctx, &mut tx, &survivor, before, after).await?;
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

async fn publish(ctx: &Ctx<'_>, tx: &mut MySqlTx, published: Published) -> Result<(), AppError> {
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

async fn merge_contacts(
    ctx: &Ctx<'_>,
    kind: DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
    command: MergeCommand,
) -> Result<MergeOutcome, AppError> {
    let fields = read.fields;
    let scope = ctx.scope;
    let contacts = ContactStore::of(ctx);
    let primary = contact_for(ctx, &contacts, primary_id, &Permission::CONTACTS_UPDATE).await?;
    let duplicate = contact_for(ctx, &contacts, duplicate_id, &Permission::CONTACTS_DELETE).await?;
    let (primary_custom, duplicate_custom) =
        load_custom(ctx, &kind, primary_id, duplicate_id).await?;
    let plan = plan_merge(
        &command.fields,
        &system_merge_keys(&kind),
        fields,
        &primary_custom,
        &duplicate_custom,
    )?;

    let old = &primary.contact.data;
    let source = &duplicate.contact.data;
    let mut merged = old.clone();
    for key in CONTACT_MERGE_KEYS {
        let side = side_for(
            &plan.choices,
            key,
            contact_field_is_empty(old, key),
            contact_field_is_empty(source, key),
        );
        if side == MergeSide::Duplicate {
            copy_contact_field(&mut merged, source, key);
        }
    }
    if merged.first_name.trim().is_empty() {
        merged.first_name = old.first_name.clone();
    }

    let mut extra_companies = Vec::new();
    for company_id in [old.company_id, source.company_id].into_iter().flatten() {
        if merged.company_id != Some(company_id)
            && !extra_companies.contains(&company_id)
            && contacts
                .find_link(scope, primary_id, company_id)
                .await?
                .is_none()
        {
            extra_companies.push(company_id);
        }
    }

    let now = ctx.now();
    let snapshot = contact_snapshot(&merged);
    let mut tx = ctx.begin().await?;
    lock_pair(ctx, &mut tx, &kind, (primary_id, duplicate_id), read).await?;
    let primary_stage = lifecycle_stage(ctx, &mut tx, &kind, primary_id).await?;
    let moved = retire_duplicate(ctx, &mut tx, &kind, (primary_id, duplicate_id), now).await?;
    let custom = apply_custom(ctx, &mut tx, &kind, primary_id, &snapshot, &plan).await?;
    if &merged != old {
        contacts.update(&mut tx, primary_id, &merged, now).await?;
    }
    for company_id in extra_companies {
        contacts
            .insert_link(
                &mut tx,
                &NewContactCompanyLink {
                    id: Uuid::now_v7(),
                    contact_id: primary_id,
                    company_id,
                    role: None,
                    created_at: now,
                },
            )
            .await?;
    }
    let mut changes = redact_hidden(
        diff_values(&contact_snapshot(old), &snapshot, &DiffOptions::default())?,
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
            duplicate_name: duplicate.contact.full_name(),
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

async fn merge_companies(
    ctx: &Ctx<'_>,
    kind: DuplicateType,
    (primary_id, duplicate_id): (Uuid, Uuid),
    read: Read<'_>,
    command: MergeCommand,
) -> Result<MergeOutcome, AppError> {
    let fields = read.fields;
    let scope = ctx.scope;
    let companies = CompanyStore::of(ctx);
    let hierarchy = HierarchyStore::of(ctx);
    let primary = company_for(ctx, &companies, primary_id, &Permission::COMPANIES_UPDATE).await?;
    let duplicate =
        company_for(ctx, &companies, duplicate_id, &Permission::COMPANIES_DELETE).await?;
    let (primary_custom, duplicate_custom) =
        load_custom(ctx, &kind, primary_id, duplicate_id).await?;
    let plan = plan_merge(
        &command.fields,
        &system_merge_keys(&kind),
        fields,
        &primary_custom,
        &duplicate_custom,
    )?;

    let old = &primary.company.data;
    let source = &duplicate.company.data;
    let mut merged = old.clone();
    for key in COMPANY_MERGE_KEYS {
        let side = side_for(
            &plan.choices,
            key,
            company_field_is_empty(old, key),
            company_field_is_empty(source, key),
        );
        if side == MergeSide::Duplicate {
            copy_company_field(&mut merged, source, key);
        }
    }
    if merged.name.trim().is_empty() {
        merged.name = old.name.clone();
    }

    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    lock_pair(ctx, &mut tx, &kind, (primary_id, duplicate_id), read).await?;
    let primary_stage = lifecycle_stage(ctx, &mut tx, &kind, primary_id).await?;
    let blocked = blocked_parents(&hierarchy, &mut tx, &[primary_id, duplicate_id]).await?;
    merged.parent_company_id = [
        merged.parent_company_id,
        old.parent_company_id,
        source.parent_company_id,
    ]
    .into_iter()
    .find(|candidate| candidate.is_none_or(|id| !blocked.contains(&id)))
    .flatten();
    let snapshot = company_snapshot(&merged);
    reparent_children(
        &hierarchy,
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        Reparent {
            from_parent: duplicate_id,
            to_parent: Some(primary_id),
            except: Some(primary_id),
            now,
            ip: command.ip.clone(),
        },
    )
    .await?;
    let moved = retire_duplicate(ctx, &mut tx, &kind, (primary_id, duplicate_id), now).await?;
    let custom = apply_custom(ctx, &mut tx, &kind, primary_id, &snapshot, &plan).await?;
    if &merged != old {
        companies.update(&mut tx, primary_id, &merged, now).await?;
    }
    let mut changes = redact_hidden(
        diff_values(&company_snapshot(old), &snapshot, &DiffOptions::default())?,
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
            duplicate_name: duplicate.company.data.name.clone(),
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
        return Err(not_found(kind));
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
    let system_keys = system_merge_keys(&kind);
    let primary = merges
        .system_values(&kind, &system_fields, primary_id)
        .await?
        .ok_or_else(|| not_found(&kind))?;
    let duplicate = merges
        .system_values(&kind, &system_fields, duplicate_id)
        .await?
        .ok_or_else(|| not_found(&kind))?;
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
    merges
        .copy_columns(&mut tx, &kind, duplicate_id, primary_id, &copied, now)
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
