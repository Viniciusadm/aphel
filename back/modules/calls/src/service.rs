use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::*;
use crate::store::{CallOutcomeStore, CallSessionStore, CallStore};
use crate::telephony::{TelephonyRecordingRequest, TelephonyRouter, TelephonyStartRequest};
use ::config_packages::application::{is_config_key, key_from_name};
use ::saved_views::application::{SavedViewRepository, is_visible};
use ::saved_views::infrastructure::MySqlSavedViewRepository;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    ArchivedFilter, RecordQuery, RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts,
};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use tasks::domain::{ActivityRecord, ActivityScheduleInput, CreateActivityCommand, instant};
use tasks::service::{self as activity_service, PreparedActivity, record_access};
use tasks::types::require_activity_type_reader;

pub(crate) fn telephony(ctx: &Ctx<'_>) -> Result<Arc<dyn TelephonyRouter>, AppError> {
    ctx.services().require::<dyn TelephonyRouter>()
}

pub(crate) const CALL_NOT_FOUND: &str = "call_not_found";
pub(crate) const CALL_ACTIVITY_TYPE_MISSING: &str = "call_activity_type_missing";
pub(crate) const TELEPHONY_PROVIDER_NOT_FOUND: &str = "telephony_provider_not_found";
pub(crate) const CALL_RECORDING_NOT_FOUND: &str = "call_recording_not_found";
pub(crate) const MAX_REPORT_DAYS: i64 = 366;
const DEFAULT_REPORT_DAYS: i64 = 30;

pub(crate) struct PreparedCall {
    pub(crate) activity: PreparedActivity,
    pub(crate) details: CallDetails,
    pub(crate) outcome: CallOutcome,
    pub(crate) record: Option<RecordRef>,
}

#[derive(Debug, Serialize)]
struct CallSnapshot<'a> {
    direction: &'static str,
    outcome_key: &'a str,
    duration_seconds: Option<u32>,
    phone_number: Option<&'a str>,
    recording_url: Option<&'a str>,
    provider: Option<&'a str>,
    provider_call_id: Option<&'a str>,
}

impl<'a> From<&'a CallDetails> for CallSnapshot<'a> {
    fn from(details: &'a CallDetails) -> Self {
        Self {
            direction: details.direction.as_str(),
            outcome_key: &details.outcome_key,
            duration_seconds: details.duration_seconds,
            phone_number: details.phone_number.as_deref(),
            recording_url: details.recording_url.as_deref(),
            provider: details.provider.as_deref(),
            provider_call_id: details.provider_call_id.as_deref(),
        }
    }
}

pub(crate) fn call_not_found() -> AppError {
    AppError::not_found_code(CALL_NOT_FOUND, "call was not found")
}

fn provider_not_found() -> AppError {
    AppError::not_found_code(
        TELEPHONY_PROVIDER_NOT_FOUND,
        "telephony provider is not available",
    )
}

fn invalid(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid call", violations)
}

pub(crate) fn optional_text(
    value: Option<&str>,
    field: &str,
    max: usize,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    if value.chars().count() > max {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {max} characters"),
        ));
        return None;
    }
    Some(value.to_owned())
}

fn parse_duration(value: Option<i64>, violations: &mut Vec<FieldViolation>) -> Option<u32> {
    let value = value?;
    if !(0..=MAX_CALL_DURATION_SECONDS).contains(&value) {
        violations.push(FieldViolation::new(
            "duration_seconds",
            format!("must be between 0 and {MAX_CALL_DURATION_SECONDS}"),
        ));
        return None;
    }
    Some(value as u32)
}

fn parse_recording_url(
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let url = optional_text(value, "recording_url", MAX_RECORDING_URL_LENGTH, violations)?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        violations.push(FieldViolation::new(
            "recording_url",
            "must be an http or https address",
        ));
        return None;
    }
    Some(url)
}

fn parse_instant(value: &str, field: &str) -> Result<DateTime<Utc>, FieldViolation> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|instant| instant.with_timezone(&Utc))
        .map_err(|_| FieldViolation::new(field, "must be an RFC 3339 instant"))
}

fn duration_minutes(seconds: Option<u32>) -> Option<i64> {
    seconds
        .filter(|seconds| *seconds > 0)
        .map(|seconds| i64::from(seconds.div_ceil(60)))
}

fn user_uuid(scope: &AccessScope) -> Result<Uuid, AppError> {
    Uuid::parse_str(scope.user_id().as_str())
        .map_err(|_| AppError::Internal("user id is not a uuid".to_owned()))
}

pub(crate) async fn log(
    ctx: &Ctx<'_>,
    command: LogCallCommand,
) -> Result<ActivityRecord, AppError> {
    let prepared = prepare_call(ctx, command, None, false).await?;
    let mut tx = ctx.begin().await?;
    insert_call(ctx, &mut tx, &prepared).await?;
    tx.commit().await?;
    activity_service::find_record(ctx, prepared.activity.id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    activity_id: &str,
    command: UpdateCallCommand,
) -> Result<ActivityRecord, AppError> {
    ctx.scope.require(&Permission::TASKS_UPDATE)?;
    let record = activity_service::get(ctx, activity_id).await?;
    let id = record.activity.id;
    if !record_access(ctx)
        .can_update(ctx.scope, &RecordRef::new(RecordType::ACTIVITY, id))
        .await?
    {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "missing permission to update this activity",
        ));
    }
    let current = CallStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(call_not_found)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut details = current.details();
    if let Some(direction) = command.direction {
        details.direction = direction;
    }
    let mut outcome = None;
    if let Some(key) = &command.outcome_key {
        outcome = resolve_outcome(ctx, Some(key), None, false, &mut violations).await?;
        if let Some(outcome) = &outcome {
            details.outcome_key = outcome.key.clone();
        }
    }
    if let Some(duration) = command.duration_seconds {
        details.duration_seconds = parse_duration(duration, &mut violations);
    }
    if let Some(phone) = &command.phone_number {
        details.phone_number = optional_text(
            phone.as_deref(),
            "phone_number",
            MAX_PHONE_NUMBER_LENGTH,
            &mut violations,
        );
    }
    if let Some(url) = &command.recording_url {
        details.recording_url = parse_recording_url(url.as_deref(), &mut violations);
    }
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    save_update(
        ctx,
        id,
        &current.details(),
        details,
        outcome,
        record.activity.record.as_ref().map(|linked| &linked.record),
        command.ip,
    )
    .await?;
    activity_service::find_record(ctx, id).await
}

pub(crate) async fn record_event(
    ctx: &Ctx<'_>,
    command: CallEventCommand,
) -> Result<ActivityRecord, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let mut violations = Vec::new();
    let provider = optional_text(
        Some(&command.provider),
        "provider",
        MAX_PROVIDER_LENGTH,
        &mut violations,
    );
    let call_id = optional_text(
        Some(&command.provider_call_id),
        "provider_call_id",
        MAX_PROVIDER_CALL_ID_LENGTH,
        &mut violations,
    );
    if provider.is_none() && !violations.iter().any(|v| v.field == "provider") {
        violations.push(FieldViolation::new("provider", "is required"));
    }
    if call_id.is_none() && !violations.iter().any(|v| v.field == "provider_call_id") {
        violations.push(FieldViolation::new("provider_call_id", "is required"));
    }
    let (Some(provider), Some(call_id), true) = (provider, call_id, violations.is_empty()) else {
        return Err(invalid(violations));
    };
    if telephony(ctx)?.provider(ctx.scope, &provider).is_none() {
        return Err(provider_not_found());
    }
    let existing = CallStore::of(ctx)
        .find_by_provider(&provider, &call_id)
        .await?;
    match existing {
        Some(activity_id) => {
            let changes = UpdateCallCommand {
                direction: command.direction,
                outcome_key: command.outcome_key,
                duration_seconds: command.duration_seconds.map(Some),
                phone_number: command.phone_number.map(Some),
                recording_url: command.recording_url.map(Some),
                expected_updated_at: None,
                ip: command.ip,
            };
            update(ctx, &activity_id.to_string(), changes).await
        }
        None => {
            let log = LogCallCommand {
                record_type: command.record_type,
                record_id: command.record_id,
                direction: command.direction,
                outcome_key: command.outcome_key,
                duration_seconds: command.duration_seconds,
                phone_number: command.phone_number,
                recording_url: command.recording_url,
                occurred_at: command.occurred_at,
                provider: Some(provider),
                provider_call_id: Some(call_id),
                ip: command.ip,
                ..LogCallCommand::default()
            };
            let prepared = prepare_call(ctx, log, None, true).await?;
            let mut tx = ctx.begin().await?;
            insert_call(ctx, &mut tx, &prepared).await?;
            tx.commit().await?;
            activity_service::find_record(ctx, prepared.activity.id).await
        }
    }
}

pub(crate) async fn providers(ctx: &Ctx<'_>) -> Result<Vec<(String, String)>, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    Ok(telephony(ctx)?
        .providers(ctx.scope)
        .iter()
        .map(|provider| (provider.key().to_owned(), provider.label().to_owned()))
        .collect())
}

pub(crate) async fn start(
    ctx: &Ctx<'_>,
    command: StartCallCommand,
) -> Result<StartedCallView, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let provider = telephony(ctx)?
        .provider(ctx.scope, command.provider.trim())
        .ok_or_else(provider_not_found)?;
    let mut violations = Vec::new();
    let phone = optional_text(
        Some(&command.phone_number),
        "phone_number",
        MAX_PHONE_NUMBER_LENGTH,
        &mut violations,
    );
    let record = match (command.record_type.as_deref(), command.record_id.as_deref()) {
        (Some(record_type), Some(record_id)) => match RecordRef::parse(record_type, record_id) {
            Ok(record) if record_access(ctx).can_read(ctx.scope, &record).await? => Some(record),
            _ => {
                violations.push(FieldViolation::new("record_id", "record was not found"));
                None
            }
        },
        (None, None) => None,
        _ => {
            violations.push(FieldViolation::new(
                "record_id",
                "record_type and record_id must be sent together",
            ));
            None
        }
    };
    let Some(phone) = phone.filter(|_| violations.is_empty()) else {
        if violations.is_empty() {
            violations.push(FieldViolation::new("phone_number", "is required"));
        }
        return Err(invalid(violations));
    };
    let call = provider
        .start_call(&TelephonyStartRequest {
            user_id: user_uuid(ctx.scope)?,
            to: phone,
            record,
        })
        .await?;
    Ok(StartedCallView {
        provider: provider.key().to_owned(),
        provider_call_id: call.provider_call_id,
    })
}

pub(crate) async fn recording(
    ctx: &Ctx<'_>,
    activity_id: &str,
) -> Result<CallRecordingView, AppError> {
    let record = activity_service::get(ctx, activity_id).await?;
    let call = CallStore::of(ctx)
        .find(record.activity.id)
        .await?
        .ok_or_else(call_not_found)?;
    if let (Some(provider), Some(call_id)) = (&call.provider, &call.provider_call_id)
        && let Some(provider) = telephony(ctx)?.provider(ctx.scope, provider)
        && let Some(recording) = provider
            .recording(&TelephonyRecordingRequest {
                user_id: user_uuid(ctx.scope)?,
                provider_call_id: call_id.clone(),
            })
            .await?
    {
        return Ok(CallRecordingView {
            url: recording.url,
            expires_at: recording.expires_at,
        });
    }
    call.recording_url
        .map(|url| CallRecordingView {
            url,
            expires_at: None,
        })
        .ok_or_else(|| {
            AppError::not_found_code(CALL_RECORDING_NOT_FOUND, "the call has no recording")
        })
}

pub(crate) async fn report(ctx: &Ctx<'_>, query: CallReportQuery) -> Result<CallReport, AppError> {
    let visibility = ctx.scope.record_filter(&Permission::TASKS_READ)?;
    let timezone = ctx.query().executor().timezone().await?;
    let now = ctx.now();
    let mut violations = Vec::new();
    let parse_bound = |value: &str, field: &str, end: bool| {
        if let Ok(date) = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d") {
            let date = if end {
                date.succ_opt().unwrap_or(date)
            } else {
                date
            };
            return timezone
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default())
                .earliest()
                .map(|local| local.with_timezone(&Utc))
                .ok_or_else(|| FieldViolation::new(field, "is not a valid date"));
        }
        parse_instant(value, field)
    };
    let to = match query.to.as_deref() {
        Some(value) => parse_bound(value, "to", true)
            .map_err(|violation| violations.push(violation))
            .ok(),
        None => Some(now),
    };
    let from = match query.from.as_deref() {
        Some(value) => parse_bound(value, "from", false)
            .map_err(|violation| violations.push(violation))
            .ok(),
        None => to.map(|to| to - Duration::days(DEFAULT_REPORT_DAYS)),
    };
    let owner_id = match query.owner_id.as_deref().map(str::trim) {
        None | Some("") => None,
        Some("me") => Some(ctx.scope.user_id().clone()),
        Some(value) => UserId::new(value)
            .map_err(|_| violations.push(FieldViolation::new("owner_id", "must be a user id")))
            .ok(),
    };
    if let (Some(from), Some(to)) = (from, to) {
        if from >= to {
            violations.push(FieldViolation::new("from", "must be before to"));
        } else if to - from > Duration::days(MAX_REPORT_DAYS) {
            violations.push(FieldViolation::new(
                "from",
                format!("the period can have at most {MAX_REPORT_DAYS} days"),
            ));
        }
    }
    let (Some(from), Some(to), true) = (from, to, violations.is_empty()) else {
        return Err(AppError::invalid_fields(
            "invalid report period",
            violations,
        ));
    };
    let definition = ctx.registry().require(RecordType::ACTIVITY.as_str())?;
    let rows = CallStore::of(ctx)
        .report(
            ctx.scope,
            definition,
            &visibility,
            &CallReportFilter { from, to, owner_id },
        )
        .await?;
    let outcomes = CallOutcomeStore::of(ctx).list(true).await?;
    let connected_keys: Vec<&str> = outcomes
        .iter()
        .filter(|outcome| outcome.is_connected)
        .map(|outcome| outcome.key.as_str())
        .collect();
    let mut users: BTreeMap<String, CallReportUser> = BTreeMap::new();
    for row in rows {
        let key = row
            .user
            .as_ref()
            .map(|user| user.id.as_str().to_owned())
            .unwrap_or_default();
        let entry = users.entry(key).or_insert_with(|| CallReportUser {
            user: row.user.clone(),
            calls: 0,
            connected: 0,
            duration_seconds: 0,
            outcomes: Vec::new(),
        });
        entry.calls += row.calls;
        entry.duration_seconds += row.duration_seconds;
        if connected_keys.contains(&row.outcome_key.as_str()) {
            entry.connected += row.calls;
        }
        entry.outcomes.push(CallReportOutcomeCount {
            outcome_key: row.outcome_key,
            calls: row.calls,
        });
    }
    let mut users: Vec<CallReportUser> = users.into_values().collect();
    users.sort_by_key(|user| std::cmp::Reverse(user.calls));
    Ok(CallReport {
        from,
        to,
        calls: users.iter().map(|user| user.calls).sum(),
        connected: users.iter().map(|user| user.connected).sum(),
        duration_seconds: users.iter().map(|user| user.duration_seconds).sum(),
        outcomes,
        users,
    })
}

pub(crate) async fn resolve_outcome(
    ctx: &Ctx<'_>,
    key: Option<&str>,
    duration: Option<u32>,
    infer: bool,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<CallOutcome>, AppError> {
    match key.map(str::trim).filter(|key| !key.is_empty()) {
        Some(key) => {
            let outcome = CallOutcomeStore::of(ctx)
                .find_by_key(key)
                .await?
                .filter(|outcome| outcome.is_active);
            if outcome.is_none() {
                violations.push(FieldViolation::new(
                    "outcome_key",
                    "call outcome was not found or is inactive",
                ));
            }
            Ok(outcome)
        }
        None if infer => {
            let connected = duration.is_some_and(|seconds| seconds > 0);
            let outcome = CallOutcomeStore::of(ctx)
                .list(false)
                .await?
                .into_iter()
                .find(|outcome| outcome.is_connected == connected);
            if outcome.is_none() {
                violations.push(FieldViolation::new(
                    "outcome_key",
                    "no active call outcome matches the call",
                ));
            }
            Ok(outcome)
        }
        None => {
            violations.push(FieldViolation::new("outcome_key", "is required"));
            Ok(None)
        }
    }
}

pub(crate) async fn prepare_call(
    ctx: &Ctx<'_>,
    command: LogCallCommand,
    session_id: Option<Uuid>,
    infer_outcome: bool,
) -> Result<PreparedCall, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let mut violations = Vec::new();
    let direction = command.direction.unwrap_or_default();
    let duration_seconds = parse_duration(command.duration_seconds, &mut violations);
    let outcome = resolve_outcome(
        ctx,
        command.outcome_key.as_deref(),
        duration_seconds,
        infer_outcome,
        &mut violations,
    )
    .await?;
    let phone_number = optional_text(
        command.phone_number.as_deref(),
        "phone_number",
        MAX_PHONE_NUMBER_LENGTH,
        &mut violations,
    );
    let recording_url = parse_recording_url(command.recording_url.as_deref(), &mut violations);
    let provider = optional_text(
        command.provider.as_deref(),
        "provider",
        MAX_PROVIDER_LENGTH,
        &mut violations,
    );
    let provider_call_id = optional_text(
        command.provider_call_id.as_deref(),
        "provider_call_id",
        MAX_PROVIDER_CALL_ID_LENGTH,
        &mut violations,
    );
    if provider.is_some() != provider_call_id.is_some() {
        violations.push(FieldViolation::new(
            "provider_call_id",
            "provider and provider_call_id must be sent together",
        ));
    }
    if let (Some(provider), Some(call_id)) = (&provider, &provider_call_id)
        && CallStore::of(ctx)
            .find_by_provider(provider, call_id)
            .await?
            .is_some()
    {
        violations.push(FieldViolation::new(
            "provider_call_id",
            "this call was already logged",
        ));
    }
    if let Some(value) = &command.occurred_at
        && let Err(violation) = parse_instant(value, "occurred_at")
    {
        violations.push(violation);
    }
    let (Some(outcome), true) = (outcome, violations.is_empty()) else {
        return Err(invalid(violations));
    };
    let activity_type = CallOutcomeStore::of(ctx)
        .call_activity_type()
        .await?
        .ok_or_else(|| {
            AppError::validation_code(
                CALL_ACTIVITY_TYPE_MISSING,
                "the organization has no call activity type",
            )
        })?;
    let title = command
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} · {}", activity_type.name, outcome.name));
    let activity = activity_service::prepare_create(
        ctx,
        CreateActivityCommand {
            title,
            activity_type_id: activity_type.id.to_string(),
            status: Some("done".to_owned()),
            completed_at: command.occurred_at.clone(),
            record_type: command.record_type.clone(),
            record_id: command.record_id.clone(),
            duration_minutes: duration_minutes(duration_seconds),
            description: command.notes.clone(),
            custom_fields: command.custom_fields.clone(),
            ip: command.ip.clone(),
            ..CreateActivityCommand::default()
        },
        ActivityScheduleInput {
            reminder_minutes_before: Some(None),
            recurrence: None,
        },
    )
    .await?;
    let record = activity.data.record.clone();
    Ok(PreparedCall {
        details: CallDetails {
            direction,
            outcome_key: outcome.key.clone(),
            duration_seconds,
            phone_number,
            recording_url,
            provider,
            provider_call_id,
            session_id,
        },
        outcome,
        record,
        activity,
    })
}

pub(crate) async fn insert_call(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    prepared: &PreparedCall,
) -> Result<(), AppError> {
    let now = ctx.now();
    activity_service::insert_prepared(ctx, tx, &prepared.activity).await?;
    CallStore::of(ctx)
        .insert(tx, prepared.activity.id, &prepared.details, now)
        .await?;
    let changes = diff_changes(
        &Value::Null,
        &CallSnapshot::from(&prepared.details),
        &DiffOptions::default(),
    )?;
    publish_call(
        ctx,
        tx,
        prepared.activity.id,
        &prepared.details,
        &prepared.outcome,
        prepared.record.as_ref(),
        CALL_LOGGED,
        changes,
        prepared.activity_ip(),
    )
    .await
}

async fn save_update(
    ctx: &Ctx<'_>,
    activity_id: Uuid,
    old: &CallDetails,
    new: CallDetails,
    outcome: Option<CallOutcome>,
    record: Option<&RecordRef>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let changes = diff_changes(
        &CallSnapshot::from(old),
        &CallSnapshot::from(&new),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(());
    }
    let outcome = match outcome {
        Some(outcome) => outcome,
        None => CallOutcomeStore::of(ctx)
            .find_by_key(&new.outcome_key)
            .await?
            .ok_or_else(call_not_found)?,
    };
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    CallStore::of(ctx)
        .update(&mut tx, activity_id, &new, now)
        .await?;
    publish_call(
        ctx,
        &mut tx,
        activity_id,
        &new,
        &outcome,
        record,
        CALL_UPDATED,
        changes,
        ip,
    )
    .await?;
    Ok(tx.commit().await?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn publish_call(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    activity_id: Uuid,
    details: &CallDetails,
    outcome: &CallOutcome,
    record: Option<&RecordRef>,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let activity = RecordRef::new(RecordType::ACTIVITY, activity_id);
    let records: Vec<Value> = record
        .map(|record| {
            json!({
                "record_type": record.record_type.as_str(),
                "record_id": record.record_id.to_string(),
            })
        })
        .into_iter()
        .collect();
    let mut payload = json!({
        "activity_id": activity_id.to_string(),
        "activity_type_key": CALL_ACTIVITY_TYPE_KEY,
        "direction": details.direction.as_str(),
        "outcome_key": outcome.key,
        "outcome_name": outcome.name,
        "is_connected": outcome.is_connected,
        "duration_seconds": details.duration_seconds,
        "has_recording": details.recording_url.is_some(),
        "provider": details.provider,
        "provider_call_id": details.provider_call_id,
        "session_id": details.session_id.map(|id| id.to_string()),
        "user_id": ctx.scope.user_id().as_str(),
        "record_type": record.map(|record| record.record_type.as_str().to_owned()),
        "record_id": record.map(|record| record.record_id.to_string()),
        "records": records,
        "occurred_at": instant(now),
    });
    if event_type == CALL_UPDATED
        && let Some(object) = payload.as_object_mut()
    {
        object.insert(
            "changes".to_owned(),
            changes
                .iter()
                .map(
                    |change| json!({ "field": change.field, "old": change.old, "new": change.new }),
                )
                .collect(),
        );
    }
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(activity.clone())
                .with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(activity)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

impl PreparedCall {
    fn activity_ip(&self) -> Option<String> {
        self.activity.ip()
    }
}

pub(crate) const CALL_OUTCOME_NOT_FOUND: &str = "call_outcome_not_found";
pub(crate) const CALL_OUTCOME_KEY_TAKEN: &str = "call_outcome_key_taken";
pub(crate) const LAST_ACTIVE_CALL_OUTCOME: &str = "last_active_call_outcome";
pub(crate) const CALL_OUTCOME_RECORD_TYPE: RecordType =
    RecordType::from_static("calls.call_outcome");

#[derive(Debug, Serialize)]
struct OutcomeSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    is_connected: bool,
    is_active: bool,
}

impl<'a> From<&'a CallOutcome> for OutcomeSnapshot<'a> {
    fn from(outcome: &'a CallOutcome) -> Self {
        Self {
            key: &outcome.key,
            name: &outcome.name,
            is_connected: outcome.is_connected,
            is_active: outcome.is_active,
        }
    }
}

pub(crate) fn call_outcome_not_found() -> AppError {
    AppError::not_found_code(CALL_OUTCOME_NOT_FOUND, "call outcome was not found")
}

fn invalid_outcome(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid call outcome", violations)
}

pub(crate) async fn list_outcomes(
    ctx: &Ctx<'_>,
    include_inactive: bool,
) -> Result<Vec<CallOutcome>, AppError> {
    require_activity_type_reader(ctx.scope)?;
    CallOutcomeStore::of(ctx).list(include_inactive).await
}

pub(crate) async fn create_outcome(
    ctx: &Ctx<'_>,
    command: CreateCallOutcomeCommand,
) -> Result<CallOutcome, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let name = optional_text(
        Some(&command.name),
        "name",
        MAX_OUTCOME_NAME_LENGTH,
        &mut violations,
    );
    if name.is_none() && violations.is_empty() {
        violations.push(FieldViolation::new("name", "is required"));
    }
    let explicit_key = command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    if let Some(key) = explicit_key
        && !is_config_key(key)
    {
        violations.push(FieldViolation::new(
            "key",
            "must start with a lowercase letter and use only lowercase letters, digits and underscores",
        ));
    }
    let existing = CallOutcomeStore::of(ctx).list(true).await?;
    if existing.len() >= MAX_CALL_OUTCOMES {
        violations.push(FieldViolation::new(
            "name",
            format!("an organization can have at most {MAX_CALL_OUTCOMES} call outcomes"),
        ));
    }
    let (Some(name), true) = (name, violations.is_empty()) else {
        return Err(invalid_outcome(violations));
    };
    let taken = |key: &str| existing.iter().any(|outcome| outcome.key == key);
    let key = match explicit_key {
        Some(key) if taken(key) => {
            return Err(AppError::conflict_code(
                CALL_OUTCOME_KEY_TAKEN,
                "another call outcome already uses this key",
            ));
        }
        Some(key) => key.to_owned(),
        None => {
            let base = key_from_name(&name);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while taken(&candidate) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            candidate
        }
    };
    let now = ctx.now();
    let outcome = CallOutcome {
        id: Uuid::now_v7(),
        key,
        name,
        is_connected: command.is_connected,
        is_system: false,
        is_active: true,
        position: existing
            .iter()
            .map(|outcome| outcome.position)
            .max()
            .map_or(0, |position| position + 1),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &OutcomeSnapshot::from(&outcome),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    CallOutcomeStore::of(ctx).insert(&mut tx, &outcome).await?;
    publish_outcome(
        ctx,
        &mut tx,
        &outcome,
        CALL_OUTCOME_CREATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(outcome)
}

pub(crate) async fn update_outcome(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateCallOutcomeCommand,
) -> Result<CallOutcome, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let id = Uuid::parse_str(id.trim()).map_err(|_| call_outcome_not_found())?;
    let current = CallOutcomeStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(call_outcome_not_found)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut next = current.clone();
    if let Some(name) = &command.name {
        match optional_text(Some(name), "name", MAX_OUTCOME_NAME_LENGTH, &mut violations) {
            Some(name) => next.name = name,
            None if violations.is_empty() => {
                violations.push(FieldViolation::new("name", "is required"))
            }
            None => {}
        }
    }
    if let Some(is_connected) = command.is_connected {
        next.is_connected = is_connected;
    }
    if let Some(is_active) = command.is_active {
        next.is_active = is_active;
    }
    if !violations.is_empty() {
        return Err(invalid_outcome(violations));
    }
    if current.is_active && !next.is_active {
        let active = CallOutcomeStore::of(ctx).list(false).await?;
        if active.iter().all(|outcome| outcome.id == current.id) {
            return Err(AppError::conflict_code(
                LAST_ACTIVE_CALL_OUTCOME,
                "the organization needs at least one active call outcome",
            ));
        }
    }
    let changes = diff_changes(
        &OutcomeSnapshot::from(&current),
        &OutcomeSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    next.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    CallOutcomeStore::of(ctx).update(&mut tx, &next).await?;
    publish_outcome(
        ctx,
        &mut tx,
        &next,
        CALL_OUTCOME_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn reorder_outcomes(
    ctx: &Ctx<'_>,
    ids: &[String],
    ip: Option<String>,
) -> Result<Vec<CallOutcome>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let existing = CallOutcomeStore::of(ctx).list(true).await?;
    let invalid_order = || {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every call outcome of the organization exactly once",
            )],
        )
    };
    let ordered = ids
        .iter()
        .map(|id| Uuid::parse_str(id).map_err(|_| invalid_order()))
        .collect::<Result<Vec<_>, _>>()?;
    let requested: BTreeSet<Uuid> = ordered.iter().copied().collect();
    let known: BTreeSet<Uuid> = existing.iter().map(|outcome| outcome.id).collect();
    if requested.len() != ordered.len() || requested != known {
        return Err(invalid_order());
    }
    let previous: Vec<String> = existing.iter().map(|outcome| outcome.key.clone()).collect();
    let keys: Vec<String> = ordered
        .iter()
        .filter_map(|id| existing.iter().find(|outcome| outcome.id == *id))
        .map(|outcome| outcome.key.clone())
        .collect();
    if previous == keys {
        return Ok(existing);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    CallOutcomeStore::of(ctx)
        .reorder(&mut tx, &ordered, now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), CALL_OUTCOME_REORDERED, now)
                .with_payload(json!({ "keys": keys })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, CALL_OUTCOME_REORDERED, now)
                .with_changes(vec![FieldChange::new(
                    "order",
                    json!(previous),
                    json!(keys),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    CallOutcomeStore::of(ctx).list(true).await
}

async fn publish_outcome(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    outcome: &CallOutcome,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(CALL_OUTCOME_RECORD_TYPE, outcome.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "key": outcome.key,
                    "name": outcome.name,
                    "is_connected": outcome.is_connected,
                    "is_active": outcome.is_active,
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

pub(crate) const CALL_SESSION_NOT_FOUND: &str = "call_session_not_found";
pub(crate) const CALL_SESSION_VIEW_NOT_FOUND: &str = "call_session_view_not_found";
pub(crate) const CALL_SESSION_EMPTY: &str = "call_session_empty";
pub(crate) const CALL_SESSION_COMPLETED_CODE: &str = "call_session_completed";
pub(crate) const CALL_SESSION_ITEM_NOT_FOUND: &str = "call_session_item_not_found";
pub(crate) const CALL_SESSION_RECORD_TYPE: RecordType =
    RecordType::from_static("calls.call_session");

fn session_not_found() -> AppError {
    AppError::not_found_code(CALL_SESSION_NOT_FOUND, "call session was not found")
}

fn session_completed() -> AppError {
    AppError::conflict_code(
        CALL_SESSION_COMPLETED_CODE,
        "the call session is already completed",
    )
}

fn item_not_found() -> AppError {
    AppError::not_found_code(
        CALL_SESSION_ITEM_NOT_FOUND,
        "the call session has no contact at this position",
    )
}

pub(crate) async fn start_session(
    ctx: &Ctx<'_>,
    command: StartCallSessionCommand,
) -> Result<CallSessionView, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let view_not_found = || {
        AppError::not_found_code(
            CALL_SESSION_VIEW_NOT_FOUND,
            "saved view of people was not found",
        )
    };
    let view_id = Uuid::parse_str(command.view_id.trim()).map_err(|_| view_not_found())?;
    let view = MySqlSavedViewRepository::new(ctx.pool().clone())
        .find(view_id)
        .await?
        .filter(|view| view.record_type == RecordType::CONTACT && is_visible(ctx.scope, view))
        .ok_or_else(view_not_found)?;
    let mut violations = Vec::new();
    let name = optional_text(
        command.name.as_deref(),
        "name",
        MAX_SESSION_NAME_LENGTH,
        &mut violations,
    )
    .unwrap_or_else(|| view.name.chars().take(MAX_SESSION_NAME_LENGTH).collect());
    let view_script = view
        .settings
        .get(VIEW_SCRIPT_SETTING)
        .and_then(Value::as_str)
        .map(str::to_owned);
    let script = optional_text(
        command.script.as_deref().or(view_script.as_deref()),
        "script",
        MAX_SESSION_SCRIPT_LENGTH,
        &mut violations,
    );
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid call session", violations));
    }
    let listing = RecordQueryEngine::new(&ctx.query())
        .query(
            ctx.scope,
            RecordType::CONTACT.as_str(),
            RecordQuery {
                filter: view.filter.clone(),
                sort: view.sort.clone(),
                search: None,
                archived: ArchivedFilter::Exclude,
                window: ListWindow {
                    offset: 0,
                    limit: MAX_SESSION_CONTACTS,
                },
            },
        )
        .await?;
    if listing.items.is_empty() {
        return Err(AppError::validation_code(
            CALL_SESSION_EMPTY,
            "the view has no people to call",
        ));
    }
    let now = ctx.now();
    let session = CallSession {
        id: Uuid::now_v7(),
        user_id: ctx.scope.user_id().clone(),
        view_id: Some(view.id),
        name,
        script,
        status: CallSessionStatus::Active,
        position: 0,
        total: listing.items.len() as u32,
        completed_at: None,
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    CallSessionStore::of(ctx)
        .insert(&mut tx, &session, &listing.items)
        .await?;
    publish_session(
        ctx,
        &mut tx,
        &session,
        CALL_SESSION_STARTED,
        json!({ "matched": listing.total }),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    session_view(ctx, session).await
}

pub(crate) async fn list_sessions(
    ctx: &Ctx<'_>,
    active_only: bool,
    window: ListWindow,
) -> Result<Listing<CallSession>, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    CallSessionStore::of(ctx)
        .list(ctx.scope, active_only, window)
        .await
}

pub(crate) async fn get_session(ctx: &Ctx<'_>, id: &str) -> Result<CallSessionView, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let session = owned_session(ctx, id).await?;
    session_view(ctx, session).await
}

pub(crate) async fn session_items(
    ctx: &Ctx<'_>,
    id: &str,
    window: ListWindow,
) -> Result<Listing<CallSessionItem>, AppError> {
    ctx.scope.require(&Permission::TASKS_READ)?;
    let session = owned_session(ctx, id).await?;
    CallSessionStore::of(ctx).items(session.id, window).await
}

pub(crate) async fn log_in_session(
    ctx: &Ctx<'_>,
    id: &str,
    command: SessionCallCommand,
) -> Result<(ActivityRecord, CallSessionView), AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let session = active_session(ctx, id).await?;
    let position = command.position.unwrap_or(session.position);
    let item = CallSessionStore::of(ctx)
        .item(session.id, position)
        .await?
        .ok_or_else(item_not_found)?;
    let prepared = prepare_call(
        ctx,
        LogCallCommand {
            record_type: Some(RecordType::CONTACT.as_str().to_owned()),
            record_id: Some(item.contact_id.to_string()),
            direction: command.direction,
            outcome_key: command.outcome_key,
            duration_seconds: command.duration_seconds,
            phone_number: command.phone_number,
            recording_url: command.recording_url,
            notes: command.notes,
            provider: command.provider,
            provider_call_id: command.provider_call_id,
            custom_fields: command.custom_fields,
            ip: command.ip.clone(),
            ..LogCallCommand::default()
        },
        Some(session.id),
        false,
    )
    .await?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let mut locked = lock_active(ctx, &mut tx, session.id).await?;
    insert_call(ctx, &mut tx, &prepared).await?;
    CallSessionStore::of(ctx)
        .mark_item(
            &mut tx,
            session.id,
            position,
            SessionItemStatus::Logged,
            Some((prepared.activity.id, prepared.outcome.key.as_str())),
            now,
        )
        .await?;
    advance(ctx, &mut tx, &mut locked, position, command.ip).await?;
    tx.commit().await?;
    let activity = activity_service::find_record(ctx, prepared.activity.id).await?;
    Ok((activity, session_view(ctx, locked).await?))
}

pub(crate) async fn skip_in_session(
    ctx: &Ctx<'_>,
    id: &str,
    position: Option<u32>,
    ip: Option<String>,
) -> Result<CallSessionView, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let session = active_session(ctx, id).await?;
    let position = position.unwrap_or(session.position);
    let item = CallSessionStore::of(ctx)
        .item(session.id, position)
        .await?
        .ok_or_else(item_not_found)?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let mut locked = lock_active(ctx, &mut tx, session.id).await?;
    if item.status == SessionItemStatus::Pending {
        CallSessionStore::of(ctx)
            .mark_item(
                &mut tx,
                session.id,
                position,
                SessionItemStatus::Skipped,
                None,
                now,
            )
            .await?;
    }
    advance(ctx, &mut tx, &mut locked, position, ip).await?;
    tx.commit().await?;
    session_view(ctx, locked).await
}

pub(crate) async fn move_session(
    ctx: &Ctx<'_>,
    id: &str,
    position: u32,
) -> Result<CallSessionView, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let session = active_session(ctx, id).await?;
    if position >= session.total {
        return Err(AppError::invalid_fields(
            "invalid position",
            vec![FieldViolation::new(
                "position",
                format!("must be between 0 and {}", session.total.saturating_sub(1)),
            )],
        ));
    }
    let mut tx = ctx.begin().await?;
    let mut locked = lock_active(ctx, &mut tx, session.id).await?;
    if locked.position != position {
        locked.position = position;
        locked.updated_at = ctx.now();
        CallSessionStore::of(ctx).save(&mut tx, &locked).await?;
    }
    tx.commit().await?;
    session_view(ctx, locked).await
}

pub(crate) async fn finish_session(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<CallSessionView, AppError> {
    ctx.scope.require(&Permission::TASKS_CREATE)?;
    let session = active_session(ctx, id).await?;
    let mut tx = ctx.begin().await?;
    let mut locked = lock_active(ctx, &mut tx, session.id).await?;
    complete(ctx, &mut tx, &mut locked, ip).await?;
    tx.commit().await?;
    session_view(ctx, locked).await
}

async fn advance(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    session: &mut CallSession,
    after: u32,
    ip: Option<String>,
) -> Result<(), AppError> {
    let next = CallSessionStore::of(ctx)
        .next_pending(tx, session.id, after)
        .await?;
    match next {
        Some(position) => {
            session.position = position;
            session.updated_at = ctx.now();
            CallSessionStore::of(ctx).save(tx, session).await
        }
        None => complete(ctx, tx, session, ip).await,
    }
}

async fn complete(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    session: &mut CallSession,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    session.status = CallSessionStatus::Completed;
    session.position = session.total;
    session.completed_at = Some(now);
    session.updated_at = now;
    CallSessionStore::of(ctx).save(tx, session).await?;
    publish_session(ctx, tx, session, CALL_SESSION_COMPLETED, json!({}), ip).await
}

async fn owned_session(ctx: &Ctx<'_>, id: &str) -> Result<CallSession, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| session_not_found())?;
    CallSessionStore::of(ctx)
        .find(id)
        .await?
        .filter(|session| &session.user_id == ctx.scope.user_id())
        .ok_or_else(session_not_found)
}

async fn active_session(ctx: &Ctx<'_>, id: &str) -> Result<CallSession, AppError> {
    let session = owned_session(ctx, id).await?;
    if session.status != CallSessionStatus::Active {
        return Err(session_completed());
    }
    Ok(session)
}

async fn lock_active(ctx: &Ctx<'_>, tx: &mut MySqlTx, id: Uuid) -> Result<CallSession, AppError> {
    let session = CallSessionStore::of(ctx)
        .lock(tx, id)
        .await?
        .ok_or_else(session_not_found)?;
    if session.status != CallSessionStatus::Active {
        return Err(session_completed());
    }
    Ok(session)
}

async fn session_view(ctx: &Ctx<'_>, session: CallSession) -> Result<CallSessionView, AppError> {
    let counts = CallSessionStore::of(ctx).counts(session.id).await?;
    let current = if session.position < session.total {
        CallSessionStore::of(ctx)
            .item(session.id, session.position)
            .await?
    } else {
        None
    };
    Ok(CallSessionView {
        session,
        counts,
        current,
    })
}

async fn publish_session(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    session: &CallSession,
    event_type: &'static str,
    extra: Value,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let record = RecordRef::new(CALL_SESSION_RECORD_TYPE, session.id);
    let mut payload = json!({
        "session_id": session.id.to_string(),
        "name": session.name,
        "view_id": session.view_id.map(|id| id.to_string()),
        "user_id": session.user_id.as_str(),
        "status": session.status.as_str(),
        "total": session.total,
    });
    if let (Some(object), Value::Object(extra)) = (payload.as_object_mut(), extra) {
        object.extend(extra);
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
                .with_changes(vec![FieldChange::new(
                    "status",
                    Value::Null,
                    Value::String(session.status.as_str().to_owned()),
                )])
                .with_ip(ip),
        )
        .await
}
