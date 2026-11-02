use aphel_contract_crm::CrmRecordTypes;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    CreateReminderCommand, ListRemindersQuery, ReminderData, ReminderFilter, ReminderStatus,
    ReminderView, UpdateReminderCommand, normalize_reminder_note, reminder_not_found,
};
use crate::store::ReminderStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::PERMISSION_DENIED;
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;

fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

fn invalid(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid reminder", violations)
}

fn parse_instant(value: &str, violations: &mut Vec<FieldViolation>) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value.trim()) {
        Ok(instant) => Some(instant.with_timezone(&Utc)),
        Err(_) => {
            violations.push(FieldViolation::new(
                "remind_at",
                "must be an RFC 3339 instant",
            ));
            None
        }
    }
}

fn parse_note(value: Option<&str>, violations: &mut Vec<FieldViolation>) -> Option<String> {
    normalize_reminder_note(value)
        .map_err(|message| violations.push(FieldViolation::new("note", message)))
        .ok()
        .flatten()
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: ListRemindersQuery,
) -> Result<Listing<ReminderView>, AppError> {
    let status = match query.status.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => Some(ReminderStatus::parse(value).ok_or_else(|| {
            invalid(vec![FieldViolation::new(
                "status",
                "must be pending, sent or dismissed",
            )])
        })?),
    };
    let record = match (query.record_type.as_deref(), query.record_id.as_deref()) {
        (Some(record_type), Some(record_id)) => Some(RecordRef::parse(record_type, record_id)?),
        (None, None) => None,
        _ => {
            return Err(AppError::Validation(
                "record_type and record_id must be sent together".to_owned(),
            ));
        }
    };
    ReminderStore::of(ctx)
        .list(ctx.scope, &ReminderFilter { status, record }, query.window)
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<ReminderView, AppError> {
    find(ctx, id).await
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateReminderCommand,
) -> Result<ReminderView, AppError> {
    let mut violations = Vec::new();
    let remind_at = parse_instant(&command.remind_at, &mut violations);
    let note = parse_note(command.note.as_deref(), &mut violations);
    let record = resolve_record(
        ctx,
        command.record_type.as_deref(),
        command.record_id.as_deref(),
        &mut violations,
    )
    .await?;
    if record.is_none() && note.is_none() && violations.is_empty() {
        violations.push(FieldViolation::new(
            "notes.note",
            "must be sent when the reminder has no record",
        ));
    }
    let (Some(remind_at), true) = (remind_at, violations.is_empty()) else {
        return Err(invalid(violations));
    };
    let id = Uuid::now_v7();
    let data = ReminderData {
        record,
        remind_at,
        note,
        status: ReminderStatus::Pending,
    };
    let mut tx = ctx.begin().await?;
    ReminderStore::of(ctx)
        .insert(&mut tx, ctx.scope, id, &data, ctx.now())
        .await?;
    tx.commit().await?;
    find(ctx, &id.to_string()).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateReminderCommand,
) -> Result<ReminderView, AppError> {
    let current = find(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut data = ReminderData {
        record: current.record.as_ref().map(|linked| linked.record.clone()),
        remind_at: current.remind_at,
        note: current.note.clone(),
        status: current.status,
    };
    if let Some(remind_at) = &command.remind_at
        && let Some(instant) = parse_instant(remind_at, &mut violations)
        && instant != current.remind_at
    {
        data.remind_at = instant;
        data.status = ReminderStatus::Pending;
    }
    if let Some(note) = &command.note {
        data.note = parse_note(note.as_deref(), &mut violations);
    }
    match command.status {
        Some(ReminderStatus::Sent) => violations.push(FieldViolation::new(
            "status",
            "must be pending or dismissed",
        )),
        Some(status) => data.status = status,
        None => {}
    }
    if command.record_type.is_some() || command.record_id.is_some() {
        let current_type = data
            .record
            .as_ref()
            .map(|record| record.record_type.as_str().to_owned());
        let current_id = data
            .record
            .as_ref()
            .map(|record| record.record_id.to_string());
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
    if data.record.is_none() && data.note.is_none() && violations.is_empty() {
        violations.push(FieldViolation::new(
            "notes.note",
            "must be sent when the reminder has no record",
        ));
    }
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    let mut tx = ctx.begin().await?;
    if !ReminderStore::of(ctx)
        .update(&mut tx, ctx.scope, current.id, &data, ctx.now())
        .await?
    {
        return Err(reminder_not_found());
    }
    tx.commit().await?;
    find(ctx, id).await
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
    let current = find(ctx, id).await?;
    let mut tx = ctx.begin().await?;
    if !ReminderStore::of(ctx)
        .delete(&mut tx, ctx.scope, current.id)
        .await?
    {
        return Err(reminder_not_found());
    }
    tx.commit().await?;
    Ok(())
}

async fn find(ctx: &Ctx<'_>, id: &str) -> Result<ReminderView, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| reminder_not_found())?;
    ReminderStore::of(ctx)
        .find(ctx.scope, id)
        .await?
        .ok_or_else(reminder_not_found)
}

async fn resolve_record(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
    record_id: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<RecordRef>, AppError> {
    let record_type = record_type.map(str::trim).filter(|value| !value.is_empty());
    let record_id = record_id.map(str::trim).filter(|value| !value.is_empty());
    let (record_type, record_id) = match (record_type, record_id) {
        (None, None) => return Ok(None),
        (Some(_), None) => {
            violations.push(FieldViolation::new(
                "record_id",
                "must be sent together with record_type",
            ));
            return Ok(None);
        }
        (None, Some(_)) => {
            violations.push(FieldViolation::new(
                "record_type",
                "must be sent together with record_id",
            ));
            return Ok(None);
        }
        (Some(record_type), Some(record_id)) => (record_type, record_id),
    };
    let Ok(record) = RecordRef::parse(record_type, record_id) else {
        violations.push(FieldViolation::new("record_id", "record was not found"));
        return Ok(None);
    };
    if record.record_type == RecordType::NOTE {
        violations.push(FieldViolation::new(
            "record_type",
            "reminders cannot be linked to this record type",
        ));
        return Ok(None);
    }
    match record_access(ctx).can_read(ctx.scope, &record).await {
        Ok(true) => Ok(Some(record)),
        Ok(false) => {
            violations.push(FieldViolation::new("record_id", "record was not found"));
            Ok(None)
        }
        Err(AppError::Coded { code, .. }) if code == PERMISSION_DENIED => {
            violations.push(FieldViolation::new("record_id", "record was not found"));
            Ok(None)
        }
        Err(AppError::Coded { .. }) => {
            violations.push(FieldViolation::new(
                "record_type",
                "reminders cannot be linked to this record type",
            ));
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
