use std::collections::HashMap;

use chrono::{DateTime, Utc};

use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{DEFAULT_TAG_COLOR, Tag, normalize_tag_color, normalize_tag_name};
use crate::store::{TagStore, Tagging};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterOperator, ID_FIELD, MAX_LIST_VALUES, RecordQuery,
    RecordQueryEngine,
};
use aphel_kernel::application::shared::{FieldChange, RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;
use aphel_kernel::records::{RecordMutation, publish_mutation};

pub(crate) const TAG_NOT_FOUND: &str = "tag_not_found";
pub(crate) const TAG_NAME_TAKEN: &str = "tag_name_taken";
pub(crate) const TAG_NOT_ALLOWED: &str = "tag_not_allowed";
pub(crate) const RECORD_TYPE_NOT_TAGGABLE: &str = "record_type_not_taggable";

pub(crate) const TAG_CREATED: &str = "tag.created";
pub(crate) const TAG_UPDATED: &str = "tag.updated";
pub(crate) const TAG_DELETED: &str = "tag.deleted";
pub(crate) const RECORD_TAGGED: &str = "record.tagged";
pub(crate) const RECORD_UNTAGGED: &str = "record.untagged";

pub(crate) const TAG_RECORD_TYPE: RecordType = RecordType::from_static("tagging.tag");
pub(crate) const MAX_TAGS_PER_ORGANIZATION: usize = 500;

pub(crate) fn tag_not_found() -> AppError {
    AppError::not_found_code(TAG_NOT_FOUND, "tag was not found")
}

fn record_denied() -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        "the record does not exist or cannot be changed",
    )
}

pub(crate) fn taggable_record_type(record_type: &str) -> Result<RecordType, AppError> {
    let record_type = record_type.trim();
    if is_custom_record_type(record_type) {
        return RecordType::parse(record_type);
    }
    let app = aphel_kernel::modules::app();
    if app
        .record_types()
        .capabilities(record_type)
        .is_some_and(|capabilities| capabilities.has_trait(aphel_sdk::tagging::TAGGABLE))
    {
        RecordType::parse(record_type)
    } else {
        Err(AppError::validation_code(
            RECORD_TYPE_NOT_TAGGABLE,
            format!("{record_type} records cannot be tagged"),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CreateTagCommand {
    pub(crate) name: String,
    pub(crate) color: Option<String>,
    pub(crate) record_types: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateTagCommand {
    pub(crate) name: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) record_types: Option<Vec<String>>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    pub tag: Tag,
    pub(crate) usage_count: u64,
}

fn tag_snapshot(tag: &Tag) -> Value {
    json!({
        "name": tag.name,
        "color": tag.color,
        "record_types": tag.record_types,
    })
}

fn parse_record_types(values: &[String], violations: &mut Vec<FieldViolation>) -> Vec<String> {
    let mut record_types = Vec::new();
    for (index, value) in values.iter().enumerate() {
        match taggable_record_type(value) {
            Ok(record_type) => {
                let value = record_type.as_str().to_owned();
                if !record_types.contains(&value) {
                    record_types.push(value);
                }
            }
            Err(_) => violations.push(FieldViolation::new(
                format!("record_types.{index}"),
                "must be contact, company, deal or a custom object (custom.<key>)",
            )),
        }
    }
    record_types
}

fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

pub async fn list(ctx: &Ctx<'_>, record_type: Option<&str>) -> Result<Vec<TagView>, AppError> {
    let record_type = record_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(taggable_record_type)
        .transpose()?;
    let usage = TagStore::of(ctx).usage().await?;
    Ok(TagStore::of(ctx)
        .list()
        .await?
        .into_iter()
        .filter(|tag| {
            record_type
                .as_ref()
                .is_none_or(|record_type| tag.accepts(record_type.as_str()))
        })
        .map(|tag| TagView {
            usage_count: usage.get(&tag.id).copied().unwrap_or_default(),
            tag,
        })
        .collect())
}

pub async fn get(ctx: &Ctx<'_>, tag_id: &str) -> Result<TagView, AppError> {
    let tag = find(ctx, tag_id).await?;
    let usage = TagStore::of(ctx).usage().await?;
    Ok(TagView {
        usage_count: usage.get(&tag.id).copied().unwrap_or_default(),
        tag,
    })
}

pub(crate) async fn create(ctx: &Ctx<'_>, command: CreateTagCommand) -> Result<TagView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let name = normalize_tag_name(&command.name)
        .map_err(|message| violations.push(FieldViolation::new("name", message)))
        .ok();
    let color = normalize_tag_color(command.color.as_deref().unwrap_or(DEFAULT_TAG_COLOR))
        .map_err(|message| violations.push(FieldViolation::new("color", message)))
        .ok();
    let record_types = parse_record_types(&command.record_types, &mut violations);
    let (Some(name), Some(color)) = (name, color) else {
        return Err(AppError::invalid_fields("invalid tag", violations));
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid tag", violations));
    }
    ensure_name_free(ctx, &name, None).await?;
    if TagStore::of(ctx).list().await?.len() >= MAX_TAGS_PER_ORGANIZATION {
        return Err(AppError::validation_code(
            "too_many_tags",
            format!("an organization can have at most {MAX_TAGS_PER_ORGANIZATION} tags"),
        ));
    }
    let now = ctx.now();
    let tag = Tag {
        id: Uuid::now_v7(),
        name,
        color,
        record_types,
        created_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    TagStore::of(ctx).insert(&mut tx, &tag).await?;
    publish(
        ctx,
        &mut tx,
        &tag,
        TAG_CREATED,
        changes(&Value::Null, &tag_snapshot(&tag)),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(TagView {
        tag,
        usage_count: 0,
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    tag_id: &str,
    command: UpdateTagCommand,
) -> Result<TagView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = find(ctx, tag_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut tag = current.clone();
    let mut violations = Vec::new();
    if let Some(name) = &command.name {
        match normalize_tag_name(name) {
            Ok(name) => tag.name = name,
            Err(message) => violations.push(FieldViolation::new("name", message)),
        }
    }
    if let Some(color) = &command.color {
        match normalize_tag_color(color) {
            Ok(color) => tag.color = color,
            Err(message) => violations.push(FieldViolation::new("color", message)),
        }
    }
    if let Some(record_types) = &command.record_types {
        tag.record_types = parse_record_types(record_types, &mut violations);
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid tag", violations));
    }
    let usage_count = TagStore::of(ctx)
        .usage()
        .await?
        .get(&tag.id)
        .copied()
        .unwrap_or_default();
    let mut diff = changes(&tag_snapshot(&current), &tag_snapshot(&tag));
    if diff.is_empty() {
        return Ok(TagView {
            tag: current,
            usage_count,
        });
    }
    if tag.name != current.name {
        ensure_name_free(ctx, &tag.name, Some(tag.id)).await?;
    }
    tag.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    let removed = TagStore::of(ctx).update(&mut tx, &tag).await?;
    if removed > 0 {
        diff.push(FieldChange::new("taggings", json!(removed), Value::Null));
    }
    publish(ctx, &mut tx, &tag, TAG_UPDATED, diff, command.ip).await?;
    tx.commit().await?;
    Ok(TagView {
        tag,
        usage_count: usage_count.saturating_sub(removed),
    })
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    tag_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let tag = find(ctx, tag_id).await?;
    let mut tx = ctx.begin().await?;
    let removed = TagStore::of(ctx).delete(&mut tx, tag.id).await?;
    let mut diff = changes(&tag_snapshot(&tag), &Value::Null);
    diff.push(FieldChange::new("taggings", json!(removed), Value::Null));
    publish(ctx, &mut tx, &tag, TAG_DELETED, diff, ip).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn record_tags(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<Tag>, AppError> {
    let record = record_ref(ctx, record_type, record_id)?;
    if !record_access(ctx).can_read(ctx.scope, &record).await? {
        return Err(record_denied());
    }
    Ok(TagStore::of(ctx)
        .tags_of(record.record_type.as_str(), &[record.record_id])
        .await?
        .into_iter()
        .map(|(_, tag)| tag)
        .collect())
}

pub(crate) async fn tags_for_records(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_ids: &[String],
) -> Result<HashMap<Uuid, Vec<Tag>>, AppError> {
    let record_type = taggable_record_type(record_type)?;
    if !aphel_sdk::records::enabled(ctx, &record_type) {
        return Err(record_denied());
    }
    let mut ids: Vec<Uuid> = Vec::new();
    for value in record_ids {
        let id = Uuid::parse_str(value.trim()).map_err(|_| {
            AppError::invalid_fields(
                "invalid ids",
                vec![FieldViolation::new("ids", "must be a list of record ids")],
            )
        })?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() > MAX_LIST_VALUES {
        return Err(AppError::invalid_fields(
            "invalid ids",
            vec![FieldViolation::new(
                "ids",
                format!("at most {MAX_LIST_VALUES} ids are allowed"),
            )],
        ));
    }
    let mut result: HashMap<Uuid, Vec<Tag>> = HashMap::new();
    if ids.is_empty() {
        return Ok(result);
    }
    let visible = RecordQueryEngine::new(&ctx.query())
        .query(
            ctx.scope,
            record_type.as_str(),
            RecordQuery {
                filter: Some(Filter::condition(
                    ID_FIELD,
                    FilterOperator::In,
                    json!(ids.iter().map(Uuid::to_string).collect::<Vec<_>>()),
                )),
                archived: ArchivedFilter::Include,
                window: ListWindow {
                    offset: 0,
                    limit: ids.len() as u64,
                },
                ..RecordQuery::default()
            },
        )
        .await?
        .items;
    for (record_id, tag) in TagStore::of(ctx)
        .tags_of(record_type.as_str(), &visible)
        .await?
    {
        result.entry(record_id).or_default().push(tag);
    }
    Ok(result)
}

pub async fn tag_record(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    tag_id: &str,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let (record, tag) = changeable(ctx, record_type, record_id, tag_id).await?;
    if !tag.accepts(record.record_type.as_str()) {
        return Err(AppError::validation_code(
            TAG_NOT_ALLOWED,
            format!(
                "tag {} cannot be used on {} records",
                tag.name, record.record_type
            ),
        ));
    }
    let now = ctx.now();
    let tagging = Tagging {
        tag_id: tag.id,
        record: record.clone(),
        created_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
    };
    let mut tx = ctx.begin().await?;
    if !TagStore::of(ctx).add_tagging(&mut tx, &tagging).await? {
        return Ok(false);
    }
    record_change(ctx, &mut tx, record, &tag, RECORD_TAGGED, ip).await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn untag_record(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    tag_id: &str,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let (record, tag) = changeable(ctx, record_type, record_id, tag_id).await?;
    let mut tx = ctx.begin().await?;
    if !TagStore::of(ctx)
        .remove_tagging(&mut tx, tag.id, &record)
        .await?
    {
        return Ok(false);
    }
    record_change(ctx, &mut tx, record, &tag, RECORD_UNTAGGED, ip).await?;
    tx.commit().await?;
    Ok(true)
}

async fn changeable(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    tag_id: &str,
) -> Result<(RecordRef, Tag), AppError> {
    let record = record_ref(ctx, record_type, record_id)?;
    let tag = find(ctx, tag_id).await?;
    if !record_access(ctx).can_update(ctx.scope, &record).await? {
        return Err(record_denied());
    }
    Ok((record, tag))
}

fn record_ref(ctx: &Ctx<'_>, record_type: &str, record_id: &str) -> Result<RecordRef, AppError> {
    let record_type = taggable_record_type(record_type)?;
    if !aphel_sdk::records::enabled(ctx, &record_type) {
        return Err(record_denied());
    }
    let record_id = Uuid::parse_str(record_id.trim()).map_err(|_| record_denied())?;
    Ok(RecordRef::new(record_type, record_id))
}

async fn find(ctx: &Ctx<'_>, tag_id: &str) -> Result<Tag, AppError> {
    let id = Uuid::parse_str(tag_id.trim()).map_err(|_| tag_not_found())?;
    TagStore::of(ctx).find(id).await?.ok_or_else(tag_not_found)
}

async fn ensure_name_free(ctx: &Ctx<'_>, name: &str, except: Option<Uuid>) -> Result<(), AppError> {
    match TagStore::of(ctx).find_by_name(name).await? {
        Some(existing) if Some(existing.id) != except => Err(AppError::conflict_code(
            TAG_NAME_TAKEN,
            format!("a tag named {name} already exists"),
        )),
        _ => Ok(()),
    }
}

async fn record_change(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: RecordRef,
    tag: &Tag,
    event_type: &'static str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let (old, new) = if event_type == RECORD_TAGGED {
        (Value::Null, json!(tag.name))
    } else {
        (json!(tag.name), Value::Null)
    };
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            payload: json!({
                "record_type": record.record_type.as_str(),
                "tag_id": tag.id.to_string(),
                "tag_name": tag.name,
                "tag_color": tag.color,
            }),
            record,
            event_type,
            changes: vec![FieldChange::new("tags", old, new)],
            ip,
        },
    )
    .await
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    tag: &Tag,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(TAG_RECORD_TYPE, tag.id),
            event_type,
            payload: json!({ "name": tag.name, "color": tag.color }),
            changes,
            ip,
        },
    )
    .await
}

fn changes(old: &Value, new: &Value) -> Vec<FieldChange> {
    let empty = serde_json::Map::new();
    let old = old.as_object().unwrap_or(&empty);
    let new = new.as_object().unwrap_or(&empty);
    let mut keys: Vec<&String> = old.keys().chain(new.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter_map(|key| {
            let before = old.get(key).cloned().unwrap_or(Value::Null);
            let after = new.get(key).cloned().unwrap_or(Value::Null);
            (before != after).then(|| FieldChange::new(key.clone(), before, after))
        })
        .collect()
}
