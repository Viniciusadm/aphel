use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::KEY;
use crate::domain::{
    ACTIVITY_ALREADY_COMPLETED, ACTIVITY_ALREADY_OPEN, ACTIVITY_COMPLETED, ACTIVITY_CREATED,
    ACTIVITY_DELETED, ACTIVITY_REOPENED, ACTIVITY_UPDATED, ActivityData, ActivityLocation,
    ActivityLocationInput, ActivityPriority, ActivityRecord, ActivityScheduleInput,
    ActivitySnapshot, ActivityStatus, ActivityTypeView, ActivityView, CalendarListing,
    CalendarQuery, CreateActivityCommand, DEAL_AMOUNT_FIELD, DEFAULT_REMINDER_MINUTES_BEFORE,
    InboxGroup, InboxGroupKey, InboxItem, InboxListing, InboxQuery, InboxSort, LinkedDealValue,
    ListActivitiesQuery, MAX_ACTIVITY_LINKS, MAX_CALENDAR_ITEMS, MAX_CALENDAR_RANGE_DAYS,
    MAX_INBOX_ITEMS, MAX_LINKED_ACTIVITY_IDS, RecordLinkInput, UpdateActivityCommand,
    activity_not_found, activity_payload, normalize_activity_description, normalize_activity_title,
    validate_duration_minutes, validate_reminder_minutes_before,
};
use crate::hooks::{ActivityCompleting, CompletingActivity};
use crate::operations::DEFAULT_TASK_TYPE;
use crate::recurrence::{
    ActivityTemplate, ApplyTo, OccurrenceRef, RecurrenceSeries, parse_rule, to_local,
};
use crate::store::{ActivityStore, ActivityTypeStore, RecurrenceStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, FieldSet, RecordDraft, RecordPipeline,
    RecordWrite, ReferenceChecker, WriteMode, saving_unsupported,
};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    CompareOp, FieldTarget, Filter, FilterOperator, PlanCondition, PlanNode, PlanValue, Predicate,
    RecordQuery, RecordQueryExecutor, RecordQueryPorts, ResolveContext, SortSpec, resolve_query,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;
use aphel_kernel::records::PendingCommit;
use aphel_sdk::calendar::local_day;

pub fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

const UNLINKABLE_RECORD_TYPES: [RecordType; 2] = [RecordType::ACTIVITY, RecordType::NOTE];
const RECURRING_NEEDS_DUE_DATE: &str = "recurring tasks need a due date";
const LOCATION_CLOCK_SKEW_MINUTES: i64 = 5;

#[derive(Debug, Clone)]
pub struct PreparedActivity {
    pub id: Uuid,
    pub data: ActivityData,
    pub(crate) type_key: String,
    series: Option<RecurrenceSeries>,
    custom_fields: Map<String, Value>,
    ip: Option<String>,
    now: DateTime<Utc>,
}

impl PreparedActivity {
    pub fn ip(&self) -> Option<String> {
        self.ip.clone()
    }
}

struct NextOccurrence {
    id: Uuid,
    data: ActivityData,
    type_key: String,
    custom_fields: Map<String, Value>,
}

struct PreparedUpdate {
    current: ActivityView,
    data: ActivityData,
    type_key: String,
    custom_fields: Map<String, Value>,
    events: Vec<&'static str>,
    effects: RecurrenceEffects,
    ip: Option<String>,
}

#[derive(Default)]
struct RecurrenceEffects {
    save: Option<RecurrenceSeries>,
    next: Option<NextOccurrence>,
    rule_change: Option<FieldChange>,
    discard_next: bool,
}

fn rule_text(series: Option<&RecurrenceSeries>) -> Value {
    series
        .filter(|series| series.is_active())
        .map(|series| Value::String(series.rule.to_rrule()))
        .unwrap_or(Value::Null)
}

fn activity_denied(action: &str) -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        format!("missing permission to {action} this activity"),
    )
}

fn invalid(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid activity", violations)
}

fn parse_instant(
    value: &str,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value.trim()) {
        Ok(instant) => Some(instant.with_timezone(&Utc)),
        Err(_) => {
            violations.push(FieldViolation::new(field, "must be an RFC 3339 instant"));
            None
        }
    }
}

fn parse_status(value: &str, violations: &mut Vec<FieldViolation>) -> Option<ActivityStatus> {
    let status = ActivityStatus::parse(value);
    if status.is_none() {
        violations.push(FieldViolation::new(
            "status",
            "must be open, done or canceled",
        ));
    }
    status
}

fn parse_priority(value: &str, violations: &mut Vec<FieldViolation>) -> Option<ActivityPriority> {
    let priority = ActivityPriority::parse(value);
    if priority.is_none() {
        violations.push(FieldViolation::new(
            "priority",
            "must be low, normal, high or urgent",
        ));
    }
    priority
}

fn parse_duration(value: i64, violations: &mut Vec<FieldViolation>) -> Option<u32> {
    validate_duration_minutes(value)
        .map_err(|message| violations.push(FieldViolation::new("duration_minutes", message)))
        .ok()
}

fn parse_location(
    input: &ActivityLocationInput,
    now: DateTime<Utc>,
    violations: &mut Vec<FieldViolation>,
) -> Option<ActivityLocation> {
    let captured_at = match input.captured_at.as_deref() {
        Some(value) => parse_instant(value, "location.captured_at", violations)?,
        None => now,
    };
    if captured_at > now + Duration::minutes(LOCATION_CLOCK_SKEW_MINUTES) {
        violations.push(FieldViolation::new(
            "location.captured_at",
            "must not be in the future",
        ));
        return None;
    }
    ActivityLocation::new(
        input.latitude,
        input.longitude,
        input.accuracy_m,
        captured_at,
    )
    .map_err(|errors| {
        violations.extend(
            errors
                .into_iter()
                .map(|error| FieldViolation::new(error.field, error.message)),
        )
    })
    .ok()
}

fn parse_reminder(value: i64, violations: &mut Vec<FieldViolation>) -> Option<u32> {
    validate_reminder_minutes_before(value)
        .map_err(|message| violations.push(FieldViolation::new("reminder_minutes_before", message)))
        .ok()
}

pub async fn list(
    ctx: &Ctx<'_>,
    query: ListActivitiesQuery,
) -> Result<Listing<ActivityRecord>, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let ListActivitiesQuery {
        record_type,
        record_id,
        my_open,
        mut query,
    } = query;
    let mut restrictions = Vec::new();
    match (record_type, record_id) {
        (Some(record_type), Some(record_id)) => {
            let record = RecordRef::parse(&record_type, &record_id)?;
            if !record_access(ctx).can_read(ctx.scope, &record).await? {
                return Err(AppError::forbidden_code(
                    PERMISSION_DENIED,
                    format!(
                        "missing permission to access {} records",
                        record.record_type
                    ),
                ));
            }
            let linked = ActivityStore::of(ctx)
                .linked_activity_ids(&record, MAX_LINKED_ACTIVITY_IDS)
                .await?;
            restrictions.push(record_restriction(&record, &linked));
        }
        (None, None) => {}
        _ => {
            return Err(AppError::Validation(
                "record_type and record_id must be sent together".to_owned(),
            ));
        }
    }
    if my_open {
        restrictions.push(column_equals("owner_id", ctx.scope.user_id().as_str()));
        restrictions.push(column_equals("status", ActivityStatus::Open.as_str()));
        if query.sort.is_empty() {
            query.sort = vec![SortSpec::asc("due_at"), SortSpec::asc("created_at")];
        }
    }
    let listing = query_activity_ids(&ctx.query(), ctx.scope, query, restrictions).await?;
    Ok(Listing {
        items: load_records(ctx, &listing.items).await?,
        total: listing.total,
    })
}

pub async fn get(ctx: &Ctx<'_>, id: &str) -> Result<ActivityRecord, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let current = readable(ctx, id).await?;
    require_record(ctx, current.id).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: CreateActivityCommand,
) -> Result<ActivityRecord, AppError> {
    create_with(ctx, command, ActivityScheduleInput::default()).await
}

pub(crate) async fn create_with(
    ctx: &Ctx<'_>,
    command: CreateActivityCommand,
    schedule: ActivityScheduleInput,
) -> Result<ActivityRecord, AppError> {
    let prepared = prepare_create(ctx, command, schedule).await?;
    let mut tx = ctx.begin().await?;
    insert_prepared(ctx, &mut tx, &prepared).await?;
    tx.commit().await?;
    require_record(ctx, prepared.id).await
}

pub async fn prepare_create(
    ctx: &Ctx<'_>,
    command: CreateActivityCommand,
    schedule: ActivityScheduleInput,
) -> Result<PreparedActivity, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let now = ctx.now();
    let mut violations = Vec::new();

    let title = normalize_activity_title(&command.title)
        .map_err(|message| violations.push(FieldViolation::new("title", message)))
        .ok();
    let activity_type = if command.activity_type_id.trim().is_empty() {
        resolve_default_type(ctx, &mut violations).await?
    } else {
        resolve_type(ctx, &command.activity_type_id, None, &mut violations).await?
    };
    let status = match command.status.as_deref() {
        Some(value) => parse_status(value, &mut violations),
        None => Some(ActivityStatus::Open),
    };
    let priority = match command.priority.as_deref() {
        Some(value) => parse_priority(value, &mut violations),
        None => Some(ActivityPriority::default()),
    };
    let due_at = command
        .due_at
        .as_deref()
        .and_then(|value| parse_instant(value, "due_at", &mut violations));
    let requested_completion = command
        .completed_at
        .as_deref()
        .and_then(|value| parse_instant(value, "completed_at", &mut violations));
    let completed_at = match status {
        Some(ActivityStatus::Done) => Some(requested_completion.unwrap_or(now)),
        _ => {
            if command.completed_at.is_some() {
                violations.push(FieldViolation::new(
                    "completed_at",
                    "only done tasks have a completion time",
                ));
            }
            None
        }
    };
    let owner_id = match command.owner_id.as_deref() {
        Some(owner) => resolve_owner(ctx, owner, &mut violations).await?,
        None => Some(ctx.scope.user_id().clone()),
    };
    let record = resolve_record(
        ctx,
        command.record_type.as_deref(),
        command.record_id.as_deref(),
        &mut violations,
    )
    .await?;
    let links = resolve_links(ctx, &command.links, record.as_ref(), &mut violations).await?;
    let duration_minutes = command
        .duration_minutes
        .and_then(|value| parse_duration(value, &mut violations));
    let description = normalize_activity_description(command.description.as_deref())
        .map_err(|message| violations.push(FieldViolation::new("description", message)))
        .ok()
        .flatten();
    let location = command
        .location
        .as_ref()
        .and_then(|input| parse_location(input, now, &mut violations));
    let reminder_minutes_before = match schedule.reminder_minutes_before {
        None => Some(DEFAULT_REMINDER_MINUTES_BEFORE),
        Some(None) => None,
        Some(Some(value)) => parse_reminder(value, &mut violations),
    };
    let mut rule = None;
    if let Some(input) = &schedule.recurrence {
        match due_at {
            None => violations.push(FieldViolation::new("due_at", RECURRING_NEEDS_DUE_DATE)),
            Some(_) if status != Some(ActivityStatus::Open) => violations.push(
                FieldViolation::new("recurrence", "only open tasks can recur"),
            ),
            Some(due) => {
                let anchor = to_local(timezone(ctx).await?, due);
                rule = parse_rule(input, anchor, &mut violations).map(|rule| (rule, anchor));
            }
        }
    }

    let (Some(title), Some(activity_type), Some(status), Some(priority), true) = (
        title,
        activity_type,
        status,
        priority,
        violations.is_empty(),
    ) else {
        return Err(invalid(violations));
    };
    let mut data = ActivityData {
        title,
        activity_type_id: activity_type.id,
        status,
        priority,
        due_at,
        completed_at,
        owner_id,
        record,
        links,
        duration_minutes,
        description,
        location,
        reminder_minutes_before,
        occurrence: None,
    };
    let series = rule.map(|(rule, anchor)| RecurrenceSeries {
        id: Uuid::now_v7(),
        rule,
        anchor,
        template: ActivityTemplate::from_data(&data),
        ended_at: None,
    });
    if let Some(series) = &series {
        data.occurrence = Some(OccurrenceRef {
            series_id: series.id,
            index: 1,
        });
    }

    check_field_access(ctx, &command.provided_keys(), None, &data).await?;
    Ok(PreparedActivity {
        id: Uuid::now_v7(),
        data,
        type_key: activity_type.key,
        series,
        custom_fields: command.custom_fields,
        ip: command.ip,
        now,
    })
}

pub async fn insert_prepared(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    prepared: &PreparedActivity,
) -> Result<(), AppError> {
    let PreparedActivity {
        id,
        data,
        type_key,
        series,
        custom_fields,
        ip,
        now,
    } = prepared;
    let record_ref = RecordRef::new(RecordType::ACTIVITY, *id);
    let (data, custom_fields) =
        saving(ctx, tx, &record_ref, WriteMode::Create, data, custom_fields).await?;
    let data = &data;
    let values = activity_values(data);
    if let Some(series) = series {
        RecurrenceStore::of(ctx).save(tx, series, *now).await?;
    }
    ActivityStore::of(ctx)
        .insert(tx, ctx.scope, *id, data, *now)
        .await?;
    let custom_changes = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(
            tx,
            ctx.scope,
            &record_ref,
            &custom_fields,
            WriteMode::Create,
        )
        .await?;
    let mut changes = diff_changes(
        &Value::Null,
        &ActivitySnapshot::from(data),
        &DiffOptions::default(),
    )?;
    if series.is_some() {
        changes.push(FieldChange::new(
            "recurrence",
            Value::Null,
            rule_text(series.as_ref()),
        ));
    }
    changes.extend(custom_changes);
    let write = RecordWrite {
        record: &record_ref,
        mode: WriteMode::Create,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(tx, ctx.scope, &write).await?;
    pipeline.saved(tx, ctx.scope, &write).await?;
    publish(
        ctx,
        tx,
        *id,
        data,
        type_key,
        &[ACTIVITY_CREATED],
        changes.clone(),
        ip.clone(),
    )
    .await
}

async fn saving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    mode: WriteMode,
    data: &ActivityData,
    custom_fields: &Map<String, Value>,
) -> Result<(ActivityData, Map<String, Value>), AppError> {
    let values = activity_values(data);
    let draft = ctx
        .pipeline()
        .saving(
            tx,
            ctx.scope,
            RecordDraft::new(record.clone(), mode, &values, custom_fields.clone()),
        )
        .await?;
    let changed = draft.changed_values(&values);
    let mut data = data.clone();
    if !changed.is_empty() {
        apply_saving_changes(ctx, record, &mut data, &changed).await?;
    }
    Ok((data, draft.custom_fields))
}

async fn apply_saving_changes(
    ctx: &Ctx<'_>,
    record: &RecordRef,
    data: &mut ActivityData,
    changed: &Map<String, Value>,
) -> Result<(), AppError> {
    let mut violations = Vec::new();
    for (key, value) in changed {
        let text = value.as_str();
        match key.as_str() {
            "title" => match normalize_activity_title(text.unwrap_or_default()) {
                Ok(title) => data.title = title,
                Err(message) => violations.push(FieldViolation::new("title", message)),
            },
            "priority" => {
                if let Some(priority) = parse_priority(text.unwrap_or_default(), &mut violations) {
                    data.priority = priority;
                }
            }
            "due_at" => {
                data.due_at =
                    text.and_then(|value| parse_instant(value, "due_at", &mut violations));
            }
            "description" => {
                data.description = normalize_activity_description(text)
                    .map_err(|message| violations.push(FieldViolation::new("description", message)))
                    .ok()
                    .flatten();
            }
            "duration_minutes" => {
                data.duration_minutes = value
                    .as_i64()
                    .and_then(|minutes| parse_duration(minutes, &mut violations));
            }
            "reminder_minutes_before" => {
                data.reminder_minutes_before = value
                    .as_i64()
                    .and_then(|minutes| parse_reminder(minutes, &mut violations));
            }
            "owner_id" => {
                data.owner_id = match text {
                    Some(owner) => resolve_owner(ctx, owner, &mut violations).await?,
                    None => None,
                };
            }
            _ => return Err(saving_unsupported(record, key)),
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(invalid(violations))
    }
}

pub async fn find_record(ctx: &Ctx<'_>, id: Uuid) -> Result<ActivityRecord, AppError> {
    require_record(ctx, id).await
}

pub async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateActivityCommand,
) -> Result<ActivityRecord, AppError> {
    let prepared = prepare_update(ctx, id, command).await?;
    write_update(
        ctx,
        &prepared.current,
        &prepared.data,
        &prepared.type_key,
        &prepared.custom_fields,
        &prepared.events,
        prepared.effects,
        prepared.ip,
    )
    .await?;
    require_record(ctx, prepared.current.id).await
}

pub async fn update_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    id: &str,
    command: UpdateActivityCommand,
) -> Result<Option<PendingCommit>, AppError> {
    let prepared = prepare_update(ctx, id, command).await?;
    write_update_in_tx(
        ctx,
        tx,
        &prepared.current,
        &prepared.data,
        &prepared.type_key,
        &prepared.custom_fields,
        &prepared.events,
        prepared.effects,
        prepared.ip,
    )
    .await
}

async fn prepare_update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateActivityCommand,
) -> Result<PreparedUpdate, AppError> {
    ctx.scope.require(&Permission::TASKS_UPDATE)?;
    let current = updatable(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let now = ctx.now();
    let mut violations = Vec::new();
    let mut data = current.data();
    let mut type_key = current.activity_type.key.clone();
    let apply_to = command.apply_to.unwrap_or_default();

    if let Some(title) = &command.title {
        match normalize_activity_title(title) {
            Ok(title) => data.title = title,
            Err(message) => violations.push(FieldViolation::new("title", message)),
        }
    }
    if let Some(type_id) = &command.activity_type_id
        && let Some(activity_type) = resolve_type(
            ctx,
            type_id,
            Some(current.activity_type.id),
            &mut violations,
        )
        .await?
    {
        data.activity_type_id = activity_type.id;
        type_key = activity_type.key;
    }
    if let Some(status) = command
        .status
        .as_deref()
        .and_then(|value| parse_status(value, &mut violations))
    {
        data.status = status;
    }
    if let Some(priority) = command
        .priority
        .as_deref()
        .and_then(|value| parse_priority(value, &mut violations))
    {
        data.priority = priority;
    }
    if let Some(due_at) = &command.due_at {
        data.due_at = due_at
            .as_deref()
            .and_then(|value| parse_instant(value, "due_at", &mut violations));
    }
    let requested_completion = match &command.completed_at {
        Some(Some(value)) => parse_instant(value, "completed_at", &mut violations),
        _ => None,
    };
    data.completed_at = if data.status.is_done() {
        requested_completion
            .or(if current.status.is_done() {
                current.completed_at
            } else {
                None
            })
            .or(Some(now))
    } else {
        if matches!(command.completed_at, Some(Some(_))) {
            violations.push(FieldViolation::new(
                "completed_at",
                "only done tasks have a completion time",
            ));
        }
        None
    };
    if let Some(owner) = &command.owner_id {
        data.owner_id = match owner.as_deref() {
            Some(owner) if Some(owner) == current.owner.as_ref().map(|user| user.id.as_str()) => {
                data.owner_id.clone()
            }
            Some(owner) => resolve_owner(ctx, owner, &mut violations).await?,
            None => None,
        };
    }
    if command.record_type.is_some() || command.record_id.is_some() {
        let current_type = current
            .record
            .as_ref()
            .map(|linked| linked.record.record_type.as_str().to_owned());
        let current_id = current
            .record
            .as_ref()
            .map(|linked| linked.record.record_id.to_string());
        let record_type = command.record_type.clone().unwrap_or(current_type);
        let record_id = command.record_id.clone().unwrap_or(current_id);
        data.record = resolve_record(
            ctx,
            record_type.as_deref(),
            record_id.as_deref(),
            &mut violations,
        )
        .await?;
    }
    if let Some(links) = &command.links {
        data.links = resolve_links(ctx, links, data.record.as_ref(), &mut violations).await?;
    } else if let Some(record) = &data.record {
        data.links.retain(|link| link != record);
    }
    if let Some(duration) = &command.duration_minutes {
        data.duration_minutes = duration.and_then(|value| parse_duration(value, &mut violations));
    }
    if let Some(description) = &command.description {
        data.description = normalize_activity_description(description.as_deref())
            .map_err(|message| violations.push(FieldViolation::new("description", message)))
            .ok()
            .flatten();
    }
    if let Some(location) = &command.location {
        data.location = location
            .as_ref()
            .and_then(|input| parse_location(input, now, &mut violations));
    }
    if let Some(reminder) = command.reminder_minutes_before {
        data.reminder_minutes_before =
            reminder.and_then(|value| parse_reminder(value, &mut violations));
    }
    let series_before = current_series(ctx, &current).await?;
    let mut save_series = None;
    match &command.recurrence {
        Some(Some(input)) => match data.due_at {
            None => violations.push(FieldViolation::new("due_at", RECURRING_NEEDS_DUE_DATE)),
            Some(due) => {
                let anchor = to_local(timezone(ctx).await?, due);
                if let Some(rule) = parse_rule(input, anchor, &mut violations) {
                    let series_id = series_before
                        .as_ref()
                        .map(|series| series.id)
                        .unwrap_or_else(Uuid::now_v7);
                    if data
                        .occurrence
                        .is_none_or(|occurrence| occurrence.series_id != series_id)
                    {
                        data.occurrence = Some(OccurrenceRef {
                            series_id,
                            index: 1,
                        });
                    }
                    save_series = Some(RecurrenceSeries {
                        id: series_id,
                        rule,
                        anchor,
                        template: ActivityTemplate::from_data(&data),
                        ended_at: None,
                    });
                }
            }
        },
        Some(None) => {
            if let Some(mut series) = series_before.clone().filter(RecurrenceSeries::is_active) {
                series.ended_at = Some(now);
                save_series = Some(series);
            }
        }
        None => {
            if apply_to == ApplyTo::Following
                && let Some(mut series) = series_before.clone().filter(RecurrenceSeries::is_active)
            {
                if data.due_at != current.due_at
                    && let Some(due) = data.due_at
                {
                    series.anchor = to_local(timezone(ctx).await?, due);
                }
                series.template = ActivityTemplate::from_data(&data);
                save_series = Some(series);
            }
        }
    }
    let active_series = save_series
        .as_ref()
        .or(series_before.as_ref())
        .filter(|series| series.is_active())
        .cloned();
    if active_series.is_some()
        && data.due_at.is_none()
        && !violations
            .iter()
            .any(|violation| violation.field == "due_at")
    {
        violations.push(FieldViolation::new("due_at", RECURRING_NEEDS_DUE_DATE));
    }
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    check_field_access(ctx, &command.provided_keys(), Some(&current.data()), &data).await?;

    let mut events = vec![ACTIVITY_UPDATED];
    let completing = !current.status.is_done() && data.status.is_done();
    match (current.status.is_done(), data.status.is_done()) {
        (false, true) => events.push(ACTIVITY_COMPLETED),
        (true, false) if data.status == ActivityStatus::Open => events.push(ACTIVITY_REOPENED),
        _ => {}
    }
    let old_rule = rule_text(series_before.as_ref());
    let new_rule = rule_text(active_series.as_ref());
    let mut effects = RecurrenceEffects {
        rule_change: (old_rule != new_rule)
            .then(|| FieldChange::new("recurrence", old_rule, new_rule)),
        save: save_series,
        next: None,
        discard_next: events.contains(&ACTIVITY_REOPENED),
    };
    if completing && let Some(series) = &active_series {
        effects.next = prepare_next(ctx, current.id, &data, series).await?;
    }
    Ok(PreparedUpdate {
        current,
        data,
        type_key,
        custom_fields: command.custom_fields,
        events,
        effects,
        ip: command.ip,
    })
}

pub(crate) async fn complete(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<ActivityRecord, AppError> {
    ctx.scope.require(&Permission::TASKS_UPDATE)?;
    let current = updatable(ctx, id).await?;
    if current.status.is_done() {
        return Err(AppError::conflict_code(
            ACTIVITY_ALREADY_COMPLETED,
            "the activity is already done",
        ));
    }
    let mut data = current.data();
    data.status = ActivityStatus::Done;
    data.completed_at = Some(ctx.now());
    let mut effects = RecurrenceEffects::default();
    if let Some(series) = current_series(ctx, &current)
        .await?
        .filter(RecurrenceSeries::is_active)
    {
        effects.next = prepare_next(ctx, current.id, &data, &series).await?;
    }
    write_update(
        ctx,
        &current,
        &data,
        &current.activity_type.key,
        &Map::new(),
        &[ACTIVITY_COMPLETED],
        effects,
        ip,
    )
    .await?;
    require_record(ctx, current.id).await
}

pub(crate) async fn reopen(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<ActivityRecord, AppError> {
    ctx.scope.require(&Permission::TASKS_UPDATE)?;
    let current = updatable(ctx, id).await?;
    if current.status == ActivityStatus::Open {
        return Err(AppError::conflict_code(
            ACTIVITY_ALREADY_OPEN,
            "the activity is already open",
        ));
    }
    let mut data = current.data();
    data.status = ActivityStatus::Open;
    data.completed_at = None;
    write_update(
        ctx,
        &current,
        &data,
        &current.activity_type.key,
        &Map::new(),
        &[ACTIVITY_REOPENED],
        RecurrenceEffects {
            discard_next: true,
            ..RecurrenceEffects::default()
        },
        ip,
    )
    .await?;
    require_record(ctx, current.id).await
}

pub async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let filter = ctx.scope.record_filter(&Permission::TASKS_DELETE)?;
    let current = readable(ctx, id).await?;
    if !ActivityStore::of(ctx)
        .within_scope(ctx.scope, current.id, &filter)
        .await?
    {
        return Err(activity_denied("delete"));
    }
    let now = ctx.now();
    let data = current.data();
    let changes = diff_changes(
        &ActivitySnapshot::from(&data),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let record = current.record_ref();
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    pipeline.deleting(&mut tx, ctx.scope, &record).await?;
    if !ActivityStore::of(ctx)
        .soft_delete(&mut tx, current.id, now)
        .await?
    {
        return Err(activity_not_found());
    }
    pipeline.deleted(&mut tx, ctx.scope, &record).await?;
    publish(
        ctx,
        &mut tx,
        current.id,
        &data,
        &current.activity_type.key,
        &[ACTIVITY_DELETED],
        changes,
        ip,
    )
    .await?;
    Ok(tx.commit().await?)
}

async fn check_field_access(
    ctx: &Ctx<'_>,
    provided: &[&str],
    old: Option<&ActivityData>,
    new: &ActivityData,
) -> Result<(), AppError> {
    let snapshot = |data: &ActivityData| {
        serde_json::to_value(ActivitySnapshot::from(data)).map_err(|error| {
            AppError::Internal(format!("failed to snapshot the activity: {error}"))
        })
    };
    let old = match old {
        Some(old) => snapshot(old)?,
        None => Value::Null,
    };
    check_record_writes(
        ctx.fields().definitions(),
        ctx.scope,
        &RecordType::ACTIVITY,
        provided,
        &old,
        &snapshot(new)?,
    )
    .await
}

fn unchanged(
    current: &ActivityView,
    data: &ActivityData,
    custom_fields: &Map<String, Value>,
    effects: &RecurrenceEffects,
) -> bool {
    current.data() == *data
        && effects.rule_change.is_none()
        && custom_fields.is_empty()
        && effects.save.is_none()
        && effects.next.is_none()
}

#[allow(clippy::too_many_arguments)]
async fn write_update(
    ctx: &Ctx<'_>,
    current: &ActivityView,
    data: &ActivityData,
    type_key: &str,
    custom_fields: &Map<String, Value>,
    events: &[&'static str],
    effects: RecurrenceEffects,
    ip: Option<String>,
) -> Result<bool, AppError> {
    if unchanged(current, data, custom_fields, &effects) {
        return Ok(false);
    }
    let mut tx = ctx.begin().await?;
    match write_update_in_tx(
        ctx,
        &mut tx,
        current,
        data,
        type_key,
        custom_fields,
        events,
        effects,
        ip,
    )
    .await?
    {
        Some(pending) => {
            tx.commit().await?;
            pending.committed(ctx).await;
            Ok(true)
        }
        None => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
async fn write_update_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    current: &ActivityView,
    data: &ActivityData,
    type_key: &str,
    custom_fields: &Map<String, Value>,
    events: &[&'static str],
    effects: RecurrenceEffects,
    ip: Option<String>,
) -> Result<Option<PendingCommit>, AppError> {
    if unchanged(current, data, custom_fields, &effects) {
        return Ok(None);
    }
    let old = current.data();
    let RecurrenceEffects {
        save,
        next,
        rule_change,
        discard_next,
    } = effects;
    let now = ctx.now();
    let record_ref = current.record_ref();
    aphel_kernel::records::lock_version(tx, "tasks", current.id, Some(current.updated_at)).await?;
    if events.contains(&ACTIVITY_COMPLETED) {
        ctx.run_hook(
            ActivityCompleting,
            tx,
            KEY,
            &CompletingActivity {
                id: current.id,
                before: old.clone(),
                data: data.clone(),
                type_key: type_key.to_owned(),
            },
        )
        .await?;
    }
    let (data, custom_fields) =
        saving(ctx, tx, &record_ref, WriteMode::Update, data, custom_fields).await?;
    let data = &data;
    let values = activity_values(data);
    let mut system_changes = diff_changes(
        &ActivitySnapshot::from(&old),
        &ActivitySnapshot::from(data),
        &DiffOptions::default(),
    )?;
    system_changes.extend(rule_change);
    let data_changed = old != *data;
    if let Some(series) = &save {
        RecurrenceStore::of(ctx).save(tx, series, now).await?;
    }
    if data_changed {
        ActivityStore::of(ctx)
            .update(tx, current.id, data, now)
            .await?;
    }
    let custom_changes = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(
            tx,
            ctx.scope,
            &record_ref,
            &custom_fields,
            WriteMode::Update,
        )
        .await?;
    let mut changes = system_changes;
    changes.extend(custom_changes);
    if changes.is_empty() && next.is_none() && save.is_none() {
        return Ok(None);
    }
    let write = RecordWrite {
        record: &record_ref,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(tx, ctx.scope, &write).await?;
    pipeline.saved(tx, ctx.scope, &write).await?;
    if !changes.is_empty() {
        publish(
            ctx,
            tx,
            current.id,
            data,
            type_key,
            events,
            changes.clone(),
            ip.clone(),
        )
        .await?;
    }
    if discard_next && let Some(occurrence) = old.occurrence {
        discard_untouched_occurrence(ctx, tx, occurrence, now, ip.clone()).await?;
    }
    if let Some(next) = next {
        insert_occurrence(ctx, tx, next, now, ip).await?;
    }
    Ok(Some(PendingCommit {
        record: record_ref,
        mode: WriteMode::Update,
        changes,
        values,
    }))
}

async fn discard_untouched_occurrence(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    occurrence: OccurrenceRef,
    now: DateTime<Utc>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let store = ActivityStore::of(ctx);
    let Some(next_id) = store
        .untouched_occurrence(tx, occurrence.series_id, occurrence.index + 1)
        .await?
    else {
        return Ok(());
    };
    let Some(next) = store.find(next_id).await? else {
        return Ok(());
    };
    let data = next.data();
    let changes = diff_changes(
        &ActivitySnapshot::from(&data),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let record = next.record_ref();
    let pipeline = ctx.pipeline();
    pipeline.deleting(tx, ctx.scope, &record).await?;
    if !store.discard_occurrence(tx, next.id, now).await? {
        return Ok(());
    }
    pipeline.deleted(tx, ctx.scope, &record).await?;
    publish(
        ctx,
        tx,
        next.id,
        &data,
        &next.activity_type.key,
        &[ACTIVITY_DELETED],
        changes,
        ip,
    )
    .await
}

async fn insert_occurrence(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    next: NextOccurrence,
    now: DateTime<Utc>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let record_ref = RecordRef::new(RecordType::ACTIVITY, next.id);
    let (data, custom_fields) = saving(
        ctx,
        tx,
        &record_ref,
        WriteMode::Create,
        &next.data,
        &next.custom_fields,
    )
    .await?;
    let next = NextOccurrence { data, ..next };
    ActivityStore::of(ctx)
        .insert(tx, ctx.scope, next.id, &next.data, now)
        .await?;
    let values = activity_values(&next.data);
    let custom_changes = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(
            tx,
            ctx.scope,
            &record_ref,
            &custom_fields,
            WriteMode::Create,
        )
        .await?;
    let mut changes = diff_changes(
        &Value::Null,
        &ActivitySnapshot::from(&next.data),
        &DiffOptions::default(),
    )?;
    changes.extend(custom_changes);
    let write = RecordWrite {
        record: &record_ref,
        mode: WriteMode::Create,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(tx, ctx.scope, &write).await?;
    pipeline.saved(tx, ctx.scope, &write).await?;
    publish(
        ctx,
        tx,
        next.id,
        &next.data,
        &next.type_key,
        &[ACTIVITY_CREATED],
        changes,
        ip,
    )
    .await
}

async fn prepare_next(
    ctx: &Ctx<'_>,
    current_id: Uuid,
    data: &ActivityData,
    series: &RecurrenceSeries,
) -> Result<Option<NextOccurrence>, AppError> {
    let (Some(occurrence), Some(due_at)) = (data.occurrence, data.due_at) else {
        return Ok(None);
    };
    if occurrence.series_id != series.id {
        return Ok(None);
    }
    let next_index = occurrence.index + 1;
    if RecurrenceStore::of(ctx)
        .occurrence_exists(series.id, next_index)
        .await?
    {
        return Ok(None);
    }
    let timezone = timezone(ctx).await?;
    let Some(next_due) = series.next_due(timezone, due_at, next_index, ctx.now()) else {
        return Ok(None);
    };
    let mut next = series.template.occurrence(
        next_due,
        OccurrenceRef {
            series_id: series.id,
            index: next_index,
        },
    );
    let type_key = match ActivityTypeStore::of(ctx)
        .find(next.activity_type_id)
        .await?
        .filter(|activity_type| activity_type.is_active)
    {
        Some(activity_type) => activity_type.key,
        None => {
            next.activity_type_id = data.activity_type_id;
            ActivityTypeStore::of(ctx)
                .find(data.activity_type_id)
                .await?
                .map(|activity_type| activity_type.key)
                .unwrap_or_default()
        }
    };
    if let Some(owner) = next.owner_id.clone() {
        let members = &ctx
            .fields()
            .references()
            .existing_users(std::slice::from_ref(&owner))
            .await?;
        if !members.contains(&owner) {
            next.owner_id = None;
        }
    }
    let query = ctx.query();
    let writable = FieldSet::for_scope(
        query.definitions(),
        query.registry().require(RecordType::ACTIVITY.as_str())?,
        ctx.scope,
    )
    .await?;
    let custom_fields: Map<String, Value> = CustomFieldReader::new(&ctx.fields())
        .load(ctx.scope, &RecordType::ACTIVITY, &[current_id])
        .await?
        .remove(&current_id)
        .unwrap_or_default()
        .into_iter()
        .filter(|(key, value)| {
            !value.is_null()
                && writable.find_active(key).is_some_and(|field| {
                    !field.is_system() && field.is_editable() && !field.definition.is_formula()
                })
        })
        .collect();
    Ok(Some(NextOccurrence {
        id: Uuid::now_v7(),
        data: next,
        type_key,
        custom_fields,
    }))
}

async fn current_series(
    ctx: &Ctx<'_>,
    current: &ActivityView,
) -> Result<Option<RecurrenceSeries>, AppError> {
    match &current.recurrence {
        Some(recurrence) => RecurrenceStore::of(ctx).find(recurrence.series_id).await,
        None => Ok(None),
    }
}

pub(crate) async fn timezone(ctx: &Ctx<'_>) -> Result<Tz, AppError> {
    ctx.query().executor().timezone().await
}

#[allow(clippy::too_many_arguments)]
async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    id: Uuid,
    data: &ActivityData,
    type_key: &str,
    events: &[&'static str],
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let Some(action) = events.first() else {
        return Ok(());
    };
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(RecordType::ACTIVITY, id);
    let with_changes = !events.contains(&ACTIVITY_CREATED) && !events.contains(&ACTIVITY_DELETED);
    for event in events {
        let payload = activity_payload(
            data,
            type_key,
            now,
            with_changes.then_some(changes.as_slice()),
        );
        ctx.events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), *event, now)
                    .with_record(record.clone())
                    .with_payload(payload),
            )
            .await?;
    }
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, *action, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

async fn readable(ctx: &Ctx<'_>, id: &str) -> Result<ActivityView, AppError> {
    let id = Uuid::parse_str(id).map_err(|_| activity_not_found())?;
    let record = RecordRef::new(RecordType::ACTIVITY, id);
    if !record_access(ctx).can_read(ctx.scope, &record).await? {
        return Err(activity_not_found());
    }
    ActivityStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(activity_not_found)
}

async fn updatable(ctx: &Ctx<'_>, id: &str) -> Result<ActivityView, AppError> {
    let current = readable(ctx, id).await?;
    if !record_access(ctx)
        .can_update(ctx.scope, &current.record_ref())
        .await?
    {
        return Err(activity_denied("update"));
    }
    Ok(current)
}

async fn resolve_default_type(
    ctx: &Ctx<'_>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<ActivityTypeView>, AppError> {
    let types = ActivityTypeStore::of(ctx).list(false).await?;
    let index = types
        .iter()
        .position(|activity_type| activity_type.key == DEFAULT_TASK_TYPE)
        .unwrap_or(0);
    match types.into_iter().nth(index) {
        Some(activity_type) => Ok(Some(activity_type)),
        None => {
            violations.push(FieldViolation::new(
                "activity_type_id",
                "there is no active activity type",
            ));
            Ok(None)
        }
    }
}

async fn resolve_type(
    ctx: &Ctx<'_>,
    type_id: &str,
    current: Option<Uuid>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<ActivityTypeView>, AppError> {
    let Ok(type_id) = Uuid::parse_str(type_id.trim()) else {
        violations.push(FieldViolation::new(
            "activity_type_id",
            "activity type was not found",
        ));
        return Ok(None);
    };
    let Some(activity_type) = ActivityTypeStore::of(ctx).find(type_id).await? else {
        violations.push(FieldViolation::new(
            "activity_type_id",
            "activity type was not found",
        ));
        return Ok(None);
    };
    if !activity_type.is_active && current != Some(activity_type.id) {
        violations.push(FieldViolation::new(
            "activity_type_id",
            "activity type is inactive",
        ));
        return Ok(None);
    }
    Ok(Some(activity_type))
}

async fn resolve_owner(
    ctx: &Ctx<'_>,
    owner: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<UserId>, AppError> {
    let owner = owner.trim();
    if owner.is_empty() {
        return Ok(None);
    }
    let Ok(owner_id) = UserId::new(owner) else {
        violations.push(FieldViolation::new(
            "owner_id",
            "user is not a member of the organization",
        ));
        return Ok(None);
    };
    let members = &ctx
        .fields()
        .references()
        .existing_users(std::slice::from_ref(&owner_id))
        .await?;
    if !members.contains(&owner_id) {
        violations.push(FieldViolation::new(
            "owner_id",
            "user is not a member of the organization",
        ));
        return Ok(None);
    }
    Ok(Some(owner_id))
}

async fn resolve_record(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
    record_id: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<RecordRef>, AppError> {
    let record_type = record_type.map(str::trim).filter(|value| !value.is_empty());
    let record_id = record_id.map(str::trim).filter(|value| !value.is_empty());
    match (record_type, record_id) {
        (None, None) => Ok(None),
        (Some(_), None) => {
            violations.push(FieldViolation::new(
                "record_id",
                "must be sent together with record_type",
            ));
            Ok(None)
        }
        (None, Some(_)) => {
            violations.push(FieldViolation::new(
                "record_type",
                "must be sent together with record_id",
            ));
            Ok(None)
        }
        (Some(record_type), Some(record_id)) => {
            readable_target(ctx, record_type, record_id, "record", violations).await
        }
    }
}

async fn resolve_links(
    ctx: &Ctx<'_>,
    links: &[RecordLinkInput],
    primary: Option<&RecordRef>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Vec<RecordRef>, AppError> {
    if links.len() > MAX_ACTIVITY_LINKS {
        violations.push(FieldViolation::new(
            "links",
            "an activity can have at most 20 links",
        ));
        return Ok(Vec::new());
    }
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::new();
    for (index, link) in links.iter().enumerate() {
        let path = format!("links.{index}");
        let Some(record) = readable_target(
            ctx,
            link.record_type.trim(),
            link.record_id.trim(),
            &path,
            violations,
        )
        .await?
        else {
            continue;
        };
        if Some(&record) == primary || !seen.insert((record.record_type.clone(), record.record_id))
        {
            continue;
        }
        resolved.push(record);
    }
    Ok(resolved)
}

async fn readable_target(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<RecordRef>, AppError> {
    let type_field = if path == "record" {
        "record_type".to_owned()
    } else {
        format!("{path}.record_type")
    };
    let id_field = if path == "record" {
        "record_id".to_owned()
    } else {
        format!("{path}.record_id")
    };
    let Ok(record) = RecordRef::parse(record_type, record_id) else {
        violations.push(FieldViolation::new(id_field, "record was not found"));
        return Ok(None);
    };
    if UNLINKABLE_RECORD_TYPES.contains(&record.record_type) {
        violations.push(FieldViolation::new(
            type_field,
            "tasks cannot be linked to this record type",
        ));
        return Ok(None);
    }
    match record_access(ctx).can_read(ctx.scope, &record).await {
        Ok(true) => Ok(Some(record)),
        Ok(false) => {
            violations.push(FieldViolation::new(id_field, "record was not found"));
            Ok(None)
        }
        Err(AppError::Coded { .. }) => {
            violations.push(FieldViolation::new(
                type_field,
                "tasks cannot be linked to this record type",
            ));
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

async fn require_record(ctx: &Ctx<'_>, id: Uuid) -> Result<ActivityRecord, AppError> {
    load_records(ctx, &[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(activity_not_found)
}

pub(crate) async fn load_records(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
) -> Result<Vec<ActivityRecord>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut views: HashMap<Uuid, ActivityView> = ActivityStore::of(ctx)
        .find_many(ids)
        .await?
        .into_iter()
        .map(|view| (view.id, view))
        .collect();
    let mut custom_fields = CustomFieldReader::new(&ctx.fields())
        .load(ctx.scope, &RecordType::ACTIVITY, ids)
        .await?;
    Ok(ids
        .iter()
        .filter_map(|id| {
            views.remove(id).map(|activity| ActivityRecord {
                activity,
                custom_fields: custom_fields.remove(id).unwrap_or_default(),
            })
        })
        .collect())
}

fn activity_values(data: &ActivityData) -> Value {
    serde_json::to_value(ActivitySnapshot::from(data)).unwrap_or(Value::Null)
}

fn text_filter(field: &str, operator: FilterOperator, value: Option<&str>) -> Option<Filter> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Filter::condition(field, operator, json!(value)))
}

fn priority_rank(priority: ActivityPriority) -> u8 {
    match priority {
        ActivityPriority::Urgent => 3,
        ActivityPriority::High => 2,
        ActivityPriority::Normal => 1,
        ActivityPriority::Low => 0,
    }
}

fn due_order(left: &InboxItem, right: &InboxItem) -> Ordering {
    match (left.record.activity.due_at, right.record.activity.due_at) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => right
            .record
            .activity
            .created_at
            .cmp(&left.record.activity.created_at),
    }
}

fn item_order(sort: InboxSort, left: &InboxItem, right: &InboxItem) -> Ordering {
    match sort {
        InboxSort::Due => due_order(left, right),
        InboxSort::Priority => priority_rank(right.record.activity.priority)
            .cmp(&priority_rank(left.record.activity.priority))
            .then_with(|| due_order(left, right)),
        InboxSort::Value => {
            let amount = |item: &InboxItem| item.deal.as_ref().and_then(|deal| deal.amount);
            match (amount(left), amount(right)) {
                (Some(left_amount), Some(right_amount)) => right_amount.cmp(&left_amount),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            }
            .then_with(|| due_order(left, right))
        }
    }
}

pub(crate) async fn calendar(
    ctx: &Ctx<'_>,
    query: CalendarQuery,
) -> Result<CalendarListing, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let mut violations = Vec::new();
    let from = parse_instant(&query.from, "from", &mut violations);
    let to = parse_instant(&query.to, "to", &mut violations);
    if let Some(status) = query.status.as_deref()
        && ActivityStatus::parse(status.trim()).is_none()
    {
        violations.push(FieldViolation::new(
            "status",
            "must be open, done or canceled",
        ));
    }
    if let (Some(from), Some(to)) = (from, to) {
        if to <= from {
            violations.push(FieldViolation::new("to", "must be after from"));
        } else if to - from > Duration::days(MAX_CALENDAR_RANGE_DAYS) {
            violations.push(FieldViolation::new(
                "to",
                "the range must have at most 62 days",
            ));
        }
    }
    let (Some(from), Some(to), true) = (from, to, violations.is_empty()) else {
        return Err(AppError::invalid_fields(
            "invalid calendar range",
            violations,
        ));
    };
    let mut conditions = vec![
        Filter::condition(
            "due_at",
            FilterOperator::Gte,
            json!(from.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        ),
        Filter::condition(
            "due_at",
            FilterOperator::Lt,
            json!(to.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        ),
    ];
    conditions.extend(text_filter(
        "owner_id",
        FilterOperator::Eq,
        query.owner_id.as_deref(),
    ));
    conditions.extend(text_filter(
        "activity_type_id",
        FilterOperator::Eq,
        query.activity_type_id.as_deref(),
    ));
    conditions.extend(text_filter(
        "status",
        FilterOperator::Eq,
        query.status.as_deref(),
    ));
    if let Some(team_id) = query
        .team_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        conditions.push(Filter::condition(
            "owner_team",
            FilterOperator::In,
            json!([team_id]),
        ));
    }
    let record_query = RecordQuery {
        filter: Some(Filter::and(conditions)),
        sort: vec![SortSpec::asc("due_at"), SortSpec::asc("created_at")],
        window: ListWindow {
            offset: 0,
            limit: MAX_CALENDAR_ITEMS,
        },
        ..RecordQuery::default()
    };
    let listing = query_activity_ids(&ctx.query(), ctx.scope, record_query, Vec::new()).await?;
    Ok(CalendarListing {
        items: load_records(ctx, &listing.items).await?,
        total: listing.total,
        truncated: listing.total > MAX_CALENDAR_ITEMS,
    })
}

pub(crate) async fn inbox(ctx: &Ctx<'_>, query: InboxQuery) -> Result<InboxListing, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let mut violations = Vec::new();
    let sort = match query.sort.as_deref().map(str::trim) {
        None | Some("") => InboxSort::Due,
        Some(value) => InboxSort::parse(value).unwrap_or_else(|| {
            violations.push(FieldViolation::new(
                "sort",
                "must be due, priority or value",
            ));
            InboxSort::Due
        }),
    };
    let mut conditions = vec![Filter::condition(
        "status",
        FilterOperator::Eq,
        json!(ActivityStatus::Open.as_str()),
    )];
    match query.owner.as_deref().map(str::trim) {
        None | Some("") | Some("me") => conditions.push(Filter::condition(
            "owner_id",
            FilterOperator::Eq,
            json!(ctx.scope.user_id().as_str()),
        )),
        Some("team") => {
            let teams: Vec<Value> = match query
                .team_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(team_id) => vec![json!(team_id)],
                None => ctx
                    .scope
                    .team_ids()
                    .iter()
                    .map(|team| json!(team.as_str()))
                    .collect(),
            };
            if teams.is_empty() {
                conditions.push(Filter::condition(
                    "owner_id",
                    FilterOperator::Eq,
                    json!(ctx.scope.user_id().as_str()),
                ));
            } else {
                conditions.push(Filter::condition(
                    "owner_team",
                    FilterOperator::In,
                    Value::Array(teams),
                ));
            }
        }
        Some("all") => {}
        Some(_) => violations.push(FieldViolation::new("owner", "must be me, team or all")),
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid inbox query", violations));
    }
    let record_query = RecordQuery {
        filter: Some(Filter::and(conditions)),
        sort: vec![SortSpec::asc("due_at"), SortSpec::desc("created_at")],
        window: ListWindow {
            offset: 0,
            limit: MAX_INBOX_ITEMS,
        },
        ..RecordQuery::default()
    };
    let listing = query_activity_ids(&ctx.query(), ctx.scope, record_query, Vec::new()).await?;
    let records = load_records(ctx, &listing.items).await?;
    let (mut deals, deal_amounts_visible) = deal_values(ctx, &listing.items).await?;

    let timezone = timezone(ctx).await?;
    let now = ctx.now();
    let (_, _, end_of_today) = local_day(timezone, now);
    let mut buckets: HashMap<&'static str, Vec<InboxItem>> = HashMap::new();
    for record in records {
        let key = match record.activity.due_at {
            None => InboxGroupKey::NoDue,
            Some(due) if due < now => InboxGroupKey::Overdue,
            Some(due) if due < end_of_today => InboxGroupKey::Today,
            Some(_) => InboxGroupKey::Upcoming,
        };
        let deal = deals.remove(&record.activity.id);
        buckets
            .entry(key.as_str())
            .or_default()
            .push(InboxItem { record, deal });
    }
    let groups = InboxGroupKey::ALL
        .into_iter()
        .map(|key| {
            let mut items = buckets.remove(key.as_str()).unwrap_or_default();
            items.sort_by(|left, right| item_order(sort, left, right));
            InboxGroup { key, items }
        })
        .collect();
    Ok(InboxListing {
        timezone: timezone.name().to_owned(),
        groups,
        total: listing.total,
        truncated: listing.total > MAX_INBOX_ITEMS,
        deal_amounts_visible,
    })
}

async fn deal_values(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
) -> Result<(HashMap<Uuid, LinkedDealValue>, bool), AppError> {
    let Ok(_) = ctx.scope.record_filter(&Permission::DEALS_READ) else {
        return Ok((HashMap::new(), false));
    };
    if ids.is_empty() {
        return Ok((HashMap::new(), true));
    }
    if !aphel_sdk::records::enabled(ctx, &RecordType::DEAL) {
        return Ok((HashMap::new(), false));
    }
    let definition = ctx.registry().require(RecordType::DEAL.as_str())?;
    let fields = FieldSet::for_scope(ctx.fields().definitions(), definition, ctx.scope).await?;
    let amount_visible = !fields.is_hidden(DEAL_AMOUNT_FIELD);
    let mut values = ActivityStore::of(ctx).linked_deal_values(ids).await?;
    if !amount_visible {
        for value in values.values_mut() {
            value.amount = None;
        }
    }
    Ok((values, amount_visible))
}

pub(crate) fn column_equals(column: &str, value: impl Into<String>) -> PlanNode {
    PlanNode::Condition(PlanCondition {
        target: FieldTarget::System {
            column: column.to_owned(),
        },
        field_type: FieldType::Text,
        predicate: Predicate::Compare(CompareOp::Eq, PlanValue::Text(value.into())),
        negated: false,
    })
}

pub(crate) fn column_in(column: &str, values: impl IntoIterator<Item = String>) -> PlanNode {
    PlanNode::Condition(PlanCondition {
        target: FieldTarget::System {
            column: column.to_owned(),
        },
        field_type: FieldType::Text,
        predicate: Predicate::In(values.into_iter().map(PlanValue::Text).collect()),
        negated: false,
    })
}

pub(crate) fn record_restriction(record: &RecordRef, linked_ids: &[Uuid]) -> PlanNode {
    let primary = PlanNode::And(vec![
        column_equals("record_type", record.record_type.as_str()),
        column_equals("record_id", record.record_id.to_string()),
    ]);
    if linked_ids.is_empty() {
        return primary;
    }
    PlanNode::Or(vec![
        primary,
        column_in("id", linked_ids.iter().map(Uuid::to_string)),
    ])
}

pub(crate) async fn query_activity_ids<Q>(
    ports: &Q,
    scope: &AccessScope,
    query: RecordQuery,
    restrictions: Vec<PlanNode>,
) -> Result<Listing<Uuid>, AppError>
where
    Q: RecordQueryPorts,
{
    let definition = ports.registry().require(RecordType::ACTIVITY.as_str())?;
    let visibility = scope.record_filter(&definition.read_permission)?;
    let fields = FieldSet::for_scope(ports.definitions(), definition, scope).await?;
    let context = ResolveContext {
        now: chrono::Utc::now(),
        timezone: ports.executor().timezone().await?,
        user_id: scope.user_id().clone(),
    };
    let mut plan = resolve_query(&fields, &context, visibility, query)?;
    if !restrictions.is_empty() {
        let mut nodes = restrictions;
        if let Some(filter) = plan.filter.take() {
            nodes.push(filter);
        }
        plan.filter = Some(PlanNode::And(nodes));
    }
    ports.executor().execute(scope, &plan).await
}
