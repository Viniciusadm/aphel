use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::custom::{
    CreateScheduleCommand, DeliveryEntry, MAX_SCHEDULE_RECIPIENTS, MAX_SCHEDULES_PER_REPORT,
    REPORT_SCHEDULE_CREATED, REPORT_SCHEDULE_DELETED, REPORT_SCHEDULE_RECORD_TYPE,
    REPORT_SCHEDULE_UPDATED, ReportSchedule, SavedReport, ScheduleEntry, ScheduleFrequency,
    ScheduleTiming, UpdateScheduleCommand, authorize_edit, is_report_visible, schedule_not_found,
};
use crate::domain::timing::next_run_after;
use crate::service::reports::{can_read_type, require_report};
use crate::store::ScheduleStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::fields::permissions::query_error;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{RecordQueryExecutor, RecordQueryPorts};
use aphel_kernel::application::shared::{FieldChange, RecordRef, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::dashboard::PeriodPreset;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::records::{RecordMutation, lock_version, publish_mutation};

#[derive(Serialize)]
struct ScheduleSnapshot<'a> {
    frequency: &'static str,
    weekday: Option<u8>,
    day_of_month: Option<u8>,
    hour: u8,
    minute: u8,
    period_preset: Option<&'a str>,
    recipients: Vec<&'a str>,
    is_active: bool,
}

impl<'a> From<&'a ReportSchedule> for ScheduleSnapshot<'a> {
    fn from(schedule: &'a ReportSchedule) -> Self {
        Self {
            frequency: schedule.timing.frequency.as_str(),
            weekday: schedule.timing.weekday,
            day_of_month: schedule.timing.day_of_month,
            hour: schedule.timing.hour,
            minute: schedule.timing.minute,
            period_preset: schedule.period_preset.as_deref(),
            recipients: schedule.recipients.iter().map(UserId::as_str).collect(),
            is_active: schedule.is_active,
        }
    }
}

pub(crate) async fn list_schedules(
    ctx: &Ctx<'_>,
    report_id: &str,
) -> Result<Vec<ScheduleEntry>, AppError> {
    let report = editable_report(ctx, report_id).await?;
    let schedules = ScheduleStore::of(ctx).list_for_report(report.id).await?;
    let mut entries = Vec::with_capacity(schedules.len());
    for schedule in schedules {
        entries.push(schedule_entry(ctx, schedule).await?);
    }
    Ok(entries)
}

pub(crate) async fn create_schedule(
    ctx: &Ctx<'_>,
    report_id: &str,
    command: CreateScheduleCommand,
) -> Result<ScheduleEntry, AppError> {
    let report = editable_report(ctx, report_id).await?;
    let existing = ScheduleStore::of(ctx).list_for_report(report.id).await?;
    if existing.len() >= MAX_SCHEDULES_PER_REPORT {
        return Err(AppError::invalid_fields(
            "invalid schedule",
            vec![FieldViolation::new(
                "report_id",
                format!("a report has at most {MAX_SCHEDULES_PER_REPORT} schedules"),
            )],
        ));
    }
    let mut violations = Vec::new();
    let recipients = parse_recipients(&command.recipients, &mut violations);
    let frequency = command.frequency;
    let now = ctx.now();
    let timing = ScheduleTiming {
        frequency,
        weekday: command.weekday,
        day_of_month: command.day_of_month,
        hour: command.hour,
        minute: command.minute,
    };
    let mut schedule = ReportSchedule {
        id: Uuid::now_v7(),
        report_id: report.id,
        created_by: Some(ctx.scope.user_id().clone()),
        timing: normalized_timing(timing),
        period_preset: normalize_preset(command.period_preset),
        recipients,
        is_active: command.is_active,
        next_run_at: now,
        last_run_at: None,
        created_at: now,
        updated_at: now,
    };
    violations.extend(schedule_violations(ctx, &report, &schedule).await?);
    if !violations.is_empty() {
        return Err(query_error("invalid schedule", violations));
    }
    let timezone = ctx.query().executor().timezone().await?;
    schedule.next_run_at = next_run_after(&schedule.timing, timezone, now);
    let changes = diff_changes(
        &Value::Null,
        &ScheduleSnapshot::from(&schedule),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ScheduleStore::of(ctx).insert(&mut tx, &schedule).await?;
    record_schedule(
        ctx,
        &mut tx,
        REPORT_SCHEDULE_CREATED,
        &schedule,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    schedule_entry(ctx, schedule).await
}

pub(crate) async fn update_schedule(
    ctx: &Ctx<'_>,
    report_id: &str,
    schedule_id: &str,
    command: UpdateScheduleCommand,
) -> Result<ScheduleEntry, AppError> {
    let report = editable_report(ctx, report_id).await?;
    let current = require_schedule(ctx, &report, schedule_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut updated = current.clone();
    let mut violations = Vec::new();
    if let Some(frequency) = command.frequency {
        updated.timing.frequency = frequency;
    }
    if let Some(weekday) = command.weekday {
        updated.timing.weekday = weekday;
    }
    if let Some(day_of_month) = command.day_of_month {
        updated.timing.day_of_month = day_of_month;
    }
    if let Some(hour) = command.hour {
        updated.timing.hour = hour;
    }
    if let Some(minute) = command.minute {
        updated.timing.minute = minute;
    }
    if let Some(period_preset) = command.period_preset {
        updated.period_preset = normalize_preset(period_preset);
    }
    if let Some(recipients) = &command.recipients {
        updated.recipients = parse_recipients(recipients, &mut violations);
    }
    if let Some(is_active) = command.is_active {
        updated.is_active = is_active;
    }
    updated.timing = normalized_timing(updated.timing);
    violations.extend(schedule_violations(ctx, &report, &updated).await?);
    if !violations.is_empty() {
        return Err(query_error("invalid schedule", violations));
    }
    let changes = diff_changes(
        &ScheduleSnapshot::from(&current),
        &ScheduleSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return schedule_entry(ctx, current).await;
    }
    let now = ctx.now();
    if updated.timing != current.timing || (updated.is_active && !current.is_active) {
        let timezone = ctx.query().executor().timezone().await?;
        updated.next_run_at = next_run_after(&updated.timing, timezone, now);
    }
    updated.updated_at = now;
    let mut tx = ctx.begin().await?;
    lock_version(
        &mut tx,
        "report_schedules",
        current.id,
        command.expected_updated_at,
    )
    .await?;
    ScheduleStore::of(ctx).update(&mut tx, &updated).await?;
    record_schedule(
        ctx,
        &mut tx,
        REPORT_SCHEDULE_UPDATED,
        &updated,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    schedule_entry(ctx, updated).await
}

pub(crate) async fn delete_schedule(
    ctx: &Ctx<'_>,
    report_id: &str,
    schedule_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let report = editable_report(ctx, report_id).await?;
    let current = require_schedule(ctx, &report, schedule_id).await?;
    let changes = diff_changes(
        &ScheduleSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    ScheduleStore::of(ctx).delete(&mut tx, current.id).await?;
    record_schedule(ctx, &mut tx, REPORT_SCHEDULE_DELETED, &current, changes, ip).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn list_deliveries(
    ctx: &Ctx<'_>,
    report_id: &str,
    schedule_id: &str,
    window: ListWindow,
) -> Result<Listing<DeliveryEntry>, AppError> {
    let report = editable_report(ctx, report_id).await?;
    let schedule = require_schedule(ctx, &report, schedule_id).await?;
    let listing = ScheduleStore::of(ctx)
        .list_deliveries(schedule.id, window)
        .await?;
    let ids: Vec<UserId> = listing
        .items
        .iter()
        .map(|delivery| delivery.user_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let users = ScheduleStore::of(ctx).members(&ids, false).await?;
    Ok(Listing {
        total: listing.total,
        items: listing
            .items
            .into_iter()
            .map(|delivery| DeliveryEntry {
                recipient: users
                    .iter()
                    .find(|user| user.id == delivery.user_id)
                    .cloned(),
                delivery,
            })
            .collect(),
    })
}

pub(crate) async fn recipient_scope(
    ctx: &Ctx<'_>,
    user_id: &UserId,
) -> Result<Option<AccessScope>, AppError> {
    Ok(MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(user_id)
        .await?
        .map(|access| build_access_scope(user_id.clone(), access)))
}

async fn editable_report(ctx: &Ctx<'_>, report_id: &str) -> Result<SavedReport, AppError> {
    let report = require_report(ctx, report_id).await?;
    authorize_edit(ctx.scope, &report)?;
    Ok(report)
}

async fn require_schedule(
    ctx: &Ctx<'_>,
    report: &SavedReport,
    schedule_id: &str,
) -> Result<ReportSchedule, AppError> {
    let store = ScheduleStore::of(ctx);
    Uuid::parse_str(schedule_id.trim())
        .ok()
        .map(|id| store.find(id))
        .ok_or_else(schedule_not_found)?
        .await?
        .filter(|schedule| schedule.report_id == report.id)
        .ok_or_else(schedule_not_found)
}

async fn schedule_entry(
    ctx: &Ctx<'_>,
    schedule: ReportSchedule,
) -> Result<ScheduleEntry, AppError> {
    let users = ScheduleStore::of(ctx)
        .members(&schedule.recipients, false)
        .await?;
    let recipients = schedule
        .recipients
        .iter()
        .filter_map(|id| users.iter().find(|user| &user.id == id).cloned())
        .collect::<Vec<UserReference>>();
    Ok(ScheduleEntry {
        schedule,
        recipients,
    })
}

async fn schedule_violations(
    ctx: &Ctx<'_>,
    report: &SavedReport,
    schedule: &ReportSchedule,
) -> Result<Vec<FieldViolation>, AppError> {
    let mut violations = Vec::new();
    let timing = &schedule.timing;
    if timing.hour > 23 {
        violations.push(FieldViolation::new("hour", "must be between 0 and 23"));
    }
    if timing.minute > 59 {
        violations.push(FieldViolation::new("minute", "must be between 0 and 59"));
    }
    match timing.frequency {
        ScheduleFrequency::Weekly => {
            if !timing.weekday.is_some_and(|day| (1..=7).contains(&day)) {
                violations.push(FieldViolation::new(
                    "weekday",
                    "must be between 1 (Monday) and 7 (Sunday)",
                ));
            }
        }
        ScheduleFrequency::Monthly => {
            if !timing
                .day_of_month
                .is_some_and(|day| (1..=31).contains(&day))
            {
                violations.push(FieldViolation::new(
                    "day_of_month",
                    "must be between 1 and 31",
                ));
            }
        }
        ScheduleFrequency::Daily => {}
    }
    if let Some(preset) = &schedule.period_preset {
        if report.definition.period.is_none() {
            violations.push(FieldViolation::new(
                "period_preset",
                "the report has no period field",
            ));
        } else if !PeriodPreset::parse(preset).is_some_and(|preset| preset != PeriodPreset::Custom)
        {
            violations.push(FieldViolation::new(
                "period_preset",
                "must be a period preset other than custom",
            ));
        }
    }
    if schedule.recipients.is_empty() {
        violations.push(FieldViolation::new(
            "recipients",
            "must have at least one recipient",
        ));
    }
    if schedule.recipients.len() > MAX_SCHEDULE_RECIPIENTS {
        violations.push(FieldViolation::new(
            "recipients",
            format!("must have at most {MAX_SCHEDULE_RECIPIENTS} recipients"),
        ));
        return Ok(violations);
    }
    let members = ScheduleStore::of(ctx)
        .members(&schedule.recipients, true)
        .await?;
    for (index, user_id) in schedule.recipients.iter().enumerate() {
        let path = format!("recipients.{index}");
        if !members.iter().any(|member| &member.id == user_id) {
            violations.push(FieldViolation::new(
                path,
                "is not an active member of the organization",
            ));
            continue;
        }
        let allowed = match recipient_scope(ctx, user_id).await? {
            Some(recipient) => {
                is_report_visible(&recipient, report)
                    && can_read_type(ctx, &recipient, &report.record_type)
            }
            None => false,
        };
        if !allowed {
            violations.push(FieldViolation::new(path, "cannot see this report"));
        }
    }
    Ok(violations)
}

async fn record_schedule(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    action: &'static str,
    schedule: &ReportSchedule,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events().as_ref(),
        ctx.audit().as_ref(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(REPORT_SCHEDULE_RECORD_TYPE, schedule.id),
            event_type: action,
            payload: json!({
                "report_id": schedule.report_id.to_string(),
                "frequency": schedule.timing.frequency.as_str(),
                "recipients": schedule.recipients.len(),
                "is_active": schedule.is_active,
            }),
            changes,
            ip,
        },
    )
    .await
}

fn normalized_timing(timing: ScheduleTiming) -> ScheduleTiming {
    match timing.frequency {
        ScheduleFrequency::Daily => ScheduleTiming {
            weekday: None,
            day_of_month: None,
            ..timing
        },
        ScheduleFrequency::Weekly => ScheduleTiming {
            day_of_month: None,
            ..timing
        },
        ScheduleFrequency::Monthly => ScheduleTiming {
            weekday: None,
            ..timing
        },
    }
}

fn normalize_preset(preset: Option<String>) -> Option<String> {
    preset
        .map(|preset| preset.trim().to_owned())
        .filter(|preset| !preset.is_empty())
}

fn parse_recipients(values: &[String], violations: &mut Vec<FieldViolation>) -> Vec<UserId> {
    let mut seen = BTreeSet::new();
    let mut recipients = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let value = value.trim();
        match Uuid::parse_str(value)
            .ok()
            .and_then(|id| UserId::new(id.to_string()).ok())
        {
            Some(user_id) => {
                if seen.insert(user_id.as_str().to_owned()) {
                    recipients.push(user_id);
                }
            }
            None => violations.push(FieldViolation::new(
                format!("recipients.{index}"),
                "must be a valid user id",
            )),
        }
    }
    recipients
}
