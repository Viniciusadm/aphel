use chrono::SecondsFormat;
use serde_json::json;
use uuid::Uuid;

use crate::domain::custom::{
    DeliveryOutcome, DeliveryRequest, DeliveryStatus, MAX_DUE_SCHEDULES_PER_RUN,
    MAX_FAILURE_REASON_LENGTH, REPORT_DELIVERY_JOB, REPORT_SCHEDULE_DELIVERED,
    REPORT_SCHEDULE_DELIVERY_FAILED, REPORT_SCHEDULE_RECORD_TYPE, ReportDelivery, RunOverrides,
    is_report_visible, with_overrides,
};
use crate::domain::timing::next_run_after;
use crate::email::{AttachmentNote, MAX_ATTACHMENT_BYTES, report_email, report_link};
use crate::service::reports::{can_read_type, encode_report};
use crate::service::schedules::recipient_scope;
use crate::store::{ReportStore, ScheduleStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::mail::{EmailAttachment, EmailMessage, Mailer};
use aphel_kernel::application::query::AggregateEngine;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, Job, JobQueue, RecordRef,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::exports::ExportFormat;

fn is_access_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::Validation(_)
            | AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Coded { .. }
    )
}

fn truncated_reason(reason: &str) -> String {
    reason.chars().take(MAX_FAILURE_REASON_LENGTH).collect()
}

struct ReadyDelivery {
    message: EmailMessage,
    attachments: Vec<EmailAttachment>,
    row_count: u32,
    requested_by: Option<String>,
}

enum Prepared {
    Ready(Box<ReadyDelivery>),
    Failed {
        reason: String,
        requested_by: Option<String>,
    },
    Skipped,
}

pub(crate) async fn dispatch_due(ctx: &Ctx<'_>) -> Result<u32, AppError> {
    let now = ctx.now();
    let store = ScheduleStore::of(ctx);
    let due = store.due(now, MAX_DUE_SCHEDULES_PER_RUN).await?;
    let mut enqueued = 0;
    for item in due {
        let mut tx = ctx.begin().await?;
        let Some(schedule) = store.lock_due(&mut tx, item.schedule_id, now).await? else {
            tx.commit().await?;
            continue;
        };
        for user_id in &schedule.recipients {
            let request = DeliveryRequest {
                schedule_id: schedule.id,
                report_id: schedule.report_id,
                user_id: user_id.clone(),
                scheduled_for: schedule.next_run_at,
            };
            let job = Job::new(REPORT_DELIVERY_JOB, request.to_payload())
                .with_unique_key(request.unique_key())
                .with_max_attempts(3);
            ctx.jobs().enqueue(&mut tx, job).await?;
            enqueued += 1;
        }
        let next = next_run_after(&schedule.timing, item.timezone, now);
        store.advance(&mut tx, schedule.id, next, now).await?;
        tx.commit().await?;
    }
    Ok(enqueued)
}

pub(crate) async fn deliver(
    ctx: &Ctx<'_>,
    request: &DeliveryRequest,
    mailer: &dyn Mailer,
    app_base_url: &str,
    final_attempt: bool,
) -> Result<DeliveryOutcome, AppError> {
    if let Some(existing) = ScheduleStore::of(ctx)
        .find_delivery(request.schedule_id, &request.user_id, request.scheduled_for)
        .await?
        && existing.status != DeliveryStatus::Failed
    {
        return Ok(DeliveryOutcome::AlreadySent);
    }
    let prepared = prepare(ctx, request, app_base_url).await?;
    match prepared {
        Prepared::Skipped => Ok(DeliveryOutcome::Skipped),
        Prepared::Failed {
            reason,
            requested_by,
        } => {
            finish(
                ctx,
                request,
                DeliveryStatus::Failed,
                None,
                Some(reason),
                requested_by,
            )
            .await?;
            Ok(DeliveryOutcome::Failed)
        }
        Prepared::Ready(ready) => {
            let ReadyDelivery {
                message,
                attachments,
                row_count,
                requested_by,
            } = *ready;
            if !claim(ctx, request).await? {
                return Ok(DeliveryOutcome::AlreadySent);
            }
            match mailer.send_with_attachments(message, attachments).await {
                Ok(()) => {
                    finish(
                        ctx,
                        request,
                        DeliveryStatus::Sent,
                        Some(row_count),
                        None,
                        requested_by,
                    )
                    .await?;
                    Ok(DeliveryOutcome::Sent)
                }
                Err(error) if final_attempt => {
                    finish(
                        ctx,
                        request,
                        DeliveryStatus::Failed,
                        None,
                        Some(truncated_reason(&format!("e-mail not sent: {error}"))),
                        requested_by,
                    )
                    .await?;
                    Err(error)
                }
                Err(error) => {
                    ScheduleStore::of(ctx)
                        .release_delivery(
                            request.schedule_id,
                            &request.user_id,
                            request.scheduled_for,
                        )
                        .await?;
                    Err(error)
                }
            }
        }
    }
}

async fn prepare(
    ctx: &Ctx<'_>,
    request: &DeliveryRequest,
    app_base_url: &str,
) -> Result<Prepared, AppError> {
    let schedules = ScheduleStore::of(ctx);
    let Some(schedule) = schedules.find(request.schedule_id).await? else {
        return Ok(Prepared::Skipped);
    };
    let requested_by = schedule
        .created_by
        .as_ref()
        .map(|user| user.as_str().to_owned());
    if !schedule.recipients.contains(&request.user_id) || schedule.report_id != request.report_id {
        return Ok(Prepared::Skipped);
    }
    let failed = |reason: &str| Prepared::Failed {
        reason: reason.to_owned(),
        requested_by: requested_by.clone(),
    };
    let Some(scope) = recipient_scope(ctx, &request.user_id).await? else {
        return Ok(failed(
            "the recipient is no longer an active member of the organization",
        ));
    };
    let Some(report) = ReportStore::of(ctx).find(request.report_id).await? else {
        return Ok(Prepared::Skipped);
    };
    if !is_report_visible(&scope, &report) {
        return Ok(failed("the recipient can no longer see this report"));
    }
    if !can_read_type(ctx, &scope, &report.record_type) {
        return Ok(failed("the recipient can no longer read this record type"));
    }
    let Some(recipient) = schedules.recipient(&request.user_id).await? else {
        return Ok(failed("the recipient has no active account"));
    };
    let overrides = RunOverrides {
        preset: schedule.period_preset.clone(),
        ..RunOverrides::default()
    };
    let query = match with_overrides(&report.definition, &overrides) {
        Ok(query) => query,
        Err(error) => return Ok(failed(&truncated_reason(&error.to_string()))),
    };
    let result = match AggregateEngine::new(&ctx.query()).run(&scope, query).await {
        Ok(result) => result,
        Err(error) if is_access_error(&error) => {
            return Ok(failed(&truncated_reason(&format!(
                "the report could not run with the recipient permissions: {error}"
            ))));
        }
        Err(error) => return Err(error),
    };
    let link = report_link(app_base_url, &report);
    let (note, attachments) = match encode_report(&report, &result, ExportFormat::Xlsx) {
        Ok(export) if export.content.len() > MAX_ATTACHMENT_BYTES => {
            (AttachmentNote::TooLarge, Vec::new())
        }
        Ok(export) => (
            AttachmentNote::Attached,
            vec![EmailAttachment {
                filename: export.file_name,
                content_type: export.mime_type.to_owned(),
                bytes: export.content.to_vec(),
            }],
        ),
        Err(error) => {
            tracing::warn!(report_id = %report.id, %error, "scheduled report spreadsheet could not be encoded");
            (AttachmentNote::Unavailable, Vec::new())
        }
    };
    let message = report_email(&recipient, &report, &result, &link, note);
    Ok(Prepared::Ready(Box::new(ReadyDelivery {
        message,
        attachments,
        row_count: u32::try_from(result.rows.len()).unwrap_or(u32::MAX),
        requested_by,
    })))
}

async fn claim(ctx: &Ctx<'_>, request: &DeliveryRequest) -> Result<bool, AppError> {
    let now = ctx.now();
    let delivery = ReportDelivery {
        id: Uuid::now_v7(),
        schedule_id: request.schedule_id,
        report_id: request.report_id,
        user_id: request.user_id.clone(),
        scheduled_for: request.scheduled_for,
        status: DeliveryStatus::Sending,
        row_count: None,
        failure_reason: None,
        created_at: now,
        finished_at: None,
    };
    let mut tx = ctx.begin().await?;
    let claimed = ScheduleStore::of(ctx)
        .claim_delivery(&mut tx, &delivery)
        .await?;
    tx.commit().await?;
    Ok(claimed)
}

async fn finish(
    ctx: &Ctx<'_>,
    request: &DeliveryRequest,
    status: DeliveryStatus,
    row_count: Option<u32>,
    failure_reason: Option<String>,
    requested_by: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let delivery = ReportDelivery {
        id: Uuid::now_v7(),
        schedule_id: request.schedule_id,
        report_id: request.report_id,
        user_id: request.user_id.clone(),
        scheduled_for: request.scheduled_for,
        status,
        row_count,
        failure_reason: failure_reason.clone(),
        created_at: now,
        finished_at: Some(now),
    };
    let action = match status {
        DeliveryStatus::Sent => REPORT_SCHEDULE_DELIVERED,
        DeliveryStatus::Failed | DeliveryStatus::Sending => REPORT_SCHEDULE_DELIVERY_FAILED,
    };
    let record = RecordRef::new(REPORT_SCHEDULE_RECORD_TYPE, request.schedule_id);
    let event = DomainEvent::new(Actor::System, action, now)
        .with_record(record.clone())
        .with_payload(json!({
            "report_id": request.report_id.to_string(),
            "recipient_id": request.user_id.as_str(),
            "requested_by": requested_by,
            "scheduled_for": request.scheduled_for.to_rfc3339_opts(SecondsFormat::Secs, true),
            "row_count": row_count,
            "failure_reason": failure_reason,
        }));
    let entry = AuditEntry::new(Actor::System, action, now).with_record(record);
    let mut tx = ctx.begin().await?;
    ScheduleStore::of(ctx)
        .save_delivery(&mut tx, &delivery)
        .await?;
    ctx.events().publish(&mut tx, event).await?;
    ctx.audit().record(&mut tx, entry).await?;
    tx.commit().await?;
    Ok(())
}
