use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{EnrollmentStatus, ExitReason, RunStatus, StepKind};
use crate::hooks::{RunningStep, StepRunning};
use crate::service::{
    EnrollmentMutation, contact_ref, enrollment_ref, mark_exited, publish_enrollment_event,
    status_changes,
};
use crate::store::{DirectoryStore, EnrollmentStore, SequenceStore};
use crate::{
    EnrollmentView, KEY, MAX_FAILURE_REASON_LENGTH, MAX_TITLE_LENGTH, RenderedDraft,
    SEQUENCE_COMPLETED, SEQUENCE_EXITED, SEQUENCE_STEP_EXECUTED, SentSequenceEmail, SequenceEmail,
    SequenceStep, SequenceTask, SequenceView, StepRun, truncate,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::email::EmailAddress;
use aphel_sdk::email::EmailPreviewCommand as PreviewCommand;
use aphel_sdk::email::address_hash;
use aphel_sdk::email::sender;
use aphel_sdk::email::{RecordInput, SendEmailCommand};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EnqueueOutcome, EventPublisher, FieldChange, Job,
    JobQueue,
};
use aphel_sdk::platform::domain::access::{AccessScope, PermissionSet, RoleKey};
use aphel_sdk::tasks::CreateActivityCommand;

pub(crate) const SEQUENCES_TICK_JOB: &str = "sequences.tick";
pub(crate) const SEQUENCES_TICK_CRON: &str = "0 * * * * *";
pub(crate) const SEQUENCE_STEP_JOB: &str = "sequence.step";
pub(crate) const SEQUENCE_DISPATCH_LIMIT: u32 = 500;

pub(crate) fn sequence_system_scope() -> AccessScope {
    AccessScope::new(
        UserId::system(),
        RoleKey::system(),
        Vec::new(),
        PermissionSet::default(),
    )
}

pub(crate) fn sequence_step_job(enrollment_id: Uuid) -> Job {
    Job::new(
        SEQUENCE_STEP_JOB,
        json!({
            "enrollment_id": enrollment_id,
        }),
    )
    .with_max_attempts(3)
    .with_unique_key(format!("{SEQUENCE_STEP_JOB}:{enrollment_id}"))
}

pub(crate) fn is_definitive(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidFields { .. }
            | AppError::CodedFields { .. }
            | AppError::Validation(_)
            | AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::Conflict(_)
    ) || matches!(
        error,
        AppError::Coded { code, .. } | AppError::CodedDetails { code, .. }
            if *code != "mail_provider_unavailable"
    )
}

fn local_now(tz: Tz, now: DateTime<Utc>) -> NaiveDateTime {
    now.with_timezone(&tz).naive_local()
}

fn to_utc(tz: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    tz.from_local_datetime(&local)
        .earliest()
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&local))
}

fn day_start(tz: Tz, now: DateTime<Utc>) -> DateTime<Utc> {
    let local = local_now(tz, now);
    to_utc(tz, local.date().and_time(chrono::NaiveTime::MIN))
}

enum Outcome {
    Advance(StepRun),
    Retry(AppError),
    Exit(ExitReason, Option<StepRun>),
}

pub(crate) async fn process(
    ctx: &Ctx<'_>,
    enrollment_id: Uuid,
    can_retry: bool,
) -> Result<(), AppError> {
    let now = ctx.now();
    let enrollments = EnrollmentStore::of(ctx);
    let directory = DirectoryStore::of(ctx);
    let Some(enrollment) = enrollments.find(enrollment_id).await? else {
        return Ok(());
    };
    if enrollment.status != EnrollmentStatus::Active
        || enrollment.next_run_at.is_none_or(|at| at > now)
    {
        return Ok(());
    }
    let Some(sequence) = SequenceStore::of(ctx).find(enrollment.sequence_id).await? else {
        return Ok(());
    };
    if !sequence.is_active {
        return Ok(());
    }
    let position = enrollment.current_position;
    let Some(step) = sequence.steps.get(position as usize).cloned() else {
        return complete(ctx, &enrollment, now).await;
    };
    let tz = directory.timezone().await?;
    if step.kind == StepKind::Wait {
        let next = to_utc(
            tz,
            crate::domain::add_business_days(local_now(tz, now), step.wait_days),
        );
        return advance(ctx, &enrollment, &sequence, None, next, now).await;
    }
    if let Some(run) = enrollments.find_run(enrollment.id, position).await? {
        return recover(ctx, &enrollment, &sequence, run, now).await;
    }
    let Some(member) = directory.member_scope(&enrollment.enrolled_by).await? else {
        return exit(ctx, &enrollment, ExitReason::EnrollerUnavailable, now).await;
    };
    let member_ctx = ctx.with_scope(&member);
    let Some(contact) = directory
        .contact(&member_ctx, enrollment.contact_id)
        .await?
        .filter(|contact| !contact.deleted)
    else {
        return exit(ctx, &enrollment, ExitReason::ContactUnavailable, now).await;
    };
    if let Err(error) = step_running(
        &member_ctx,
        RunningStep {
            enrollment_id: enrollment.id,
            sequence_id: sequence.id,
            contact_id: contact.id,
            enrolled_by: enrollment.enrolled_by.clone(),
            position,
            kind: step.kind,
            template_id: step.template_id,
        },
    )
    .await
    {
        if !is_definitive(&error) {
            return Err(error);
        }
        tracing::info!(error = %error, enrollment_id = %enrollment.id, "sequence step vetoed");
        return exit(ctx, &enrollment, ExitReason::Removed, now).await;
    }
    let email = SequenceEmail {
        contact_id: contact.id,
        to: contact.email.clone().unwrap_or_default(),
        mailbox_id: enrollment.mailbox_id,
        reply_to_email_id: enrollment.last_email_id,
        template_id: step.template_id,
        subject: step.subject.clone(),
        body_html: step.body_html.clone(),
    };
    if step.kind == StepKind::AutoEmail {
        if email.to.is_empty() {
            return exit(ctx, &enrollment, ExitReason::ContactUnavailable, now).await;
        }
        if directory.has_objection(&address_hash(&email.to)).await? {
            return exit(ctx, &enrollment, ExitReason::Objection, now).await;
        }
        let local = local_now(tz, now);
        let opening = sequence.window.next_opening(local);
        if opening > local {
            return reschedule(ctx, &enrollment, to_utc(tz, opening)).await;
        }
        let today = day_start(tz, now);
        if enrollments
            .emails_since(Some(contact.id), None, today)
            .await?
            > 0
        {
            return reschedule(
                ctx,
                &enrollment,
                to_utc(tz, sequence.window.next_day_opening(local)),
            )
            .await;
        }
        let Some(mailbox_id) = directory
            .sendable_mailbox(&enrollment.enrolled_by, enrollment.mailbox_id)
            .await?
        else {
            return exit(ctx, &enrollment, ExitReason::SendFailed, now).await;
        };
        if enrollments
            .emails_since(None, Some(mailbox_id), today)
            .await?
            >= u64::from(sequence.daily_limit)
        {
            return reschedule(
                ctx,
                &enrollment,
                to_utc(tz, sequence.window.next_day_opening(local)),
            )
            .await;
        }
        let run = new_run(&enrollment, &step, Some(mailbox_id), now);
        if !claim(ctx, &run).await? {
            return Ok(());
        }
        let outcome = match send_email(
            &member_ctx,
            SequenceEmail {
                mailbox_id: Some(mailbox_id),
                ..email
            },
        )
        .await
        {
            Ok(sent) => Outcome::Advance(StepRun {
                status: RunStatus::Sent,
                mailbox_id: Some(sent.mailbox_id),
                email_id: Some(sent.email_id),
                thread_id: Some(sent.thread_id),
                ..run
            }),
            Err(error) => failure(run, error, can_retry),
        };
        return conclude(ctx, &enrollment, &sequence, outcome, now).await;
    }
    let run = new_run(&enrollment, &step, None, now);
    if !claim(ctx, &run).await? {
        return Ok(());
    }
    let outcome = match create_task(&member_ctx, &step, &email, &enrollment, now).await {
        Ok(activity_id) => Outcome::Advance(StepRun {
            status: RunStatus::Created,
            activity_id: Some(activity_id),
            ..run
        }),
        Err(error) => failure(run, error, can_retry),
    };
    conclude(ctx, &enrollment, &sequence, outcome, now).await
}

pub(crate) async fn dispatch(ctx: &Ctx<'_>) -> Result<u32, AppError> {
    let due = EnrollmentStore::of(ctx)
        .due(ctx.now(), SEQUENCE_DISPATCH_LIMIT)
        .await?;
    let mut queued = 0;
    for item in due {
        if matches!(
            ctx.jobs()
                .enqueue_now(sequence_step_job(item.enrollment_id))
                .await?,
            EnqueueOutcome::Enqueued(_)
        ) {
            queued += 1;
        }
    }
    Ok(queued)
}

async fn send_email(ctx: &Ctx<'_>, email: SequenceEmail) -> Result<SentSequenceEmail, AppError> {
    let composed = sender()?
        .send(
            ctx,
            SendEmailCommand {
                mailbox_id: email.mailbox_id.map(|id| id.to_string()),
                to: vec![EmailAddress::new(email.to)],
                subject: Some(email.subject).filter(|subject| !subject.trim().is_empty()),
                body_html: email.body_html,
                template_id: email.template_id.map(|id| id.to_string()),
                record: Some(RecordInput {
                    record_type: "people.person".to_owned(),
                    record_id: email.contact_id.to_string(),
                }),
                reply_to_message_id: email.reply_to_email_id.map(|id| id.to_string()),
                ..SendEmailCommand::default()
            },
        )
        .await?;
    Ok(SentSequenceEmail {
        email_id: composed.email_id,
        thread_id: composed.thread_id,
        mailbox_id: composed.mailbox_id,
    })
}

async fn render_draft(ctx: &Ctx<'_>, email: &SequenceEmail) -> Result<RenderedDraft, AppError> {
    let rendered = sender()?
        .preview(
            ctx,
            PreviewCommand {
                template_id: email.template_id.map(|id| id.to_string()),
                subject: Some(email.subject.clone()).filter(|subject| !subject.is_empty()),
                body_html: email.body_html.clone(),
                record_type: Some("people.person".to_owned()),
                record_id: Some(email.contact_id.to_string()),
            },
        )
        .await?;
    Ok(RenderedDraft {
        subject: rendered.subject,
        body_text: rendered.text,
    })
}

async fn create_activity(ctx: &Ctx<'_>, task: SequenceTask) -> Result<Uuid, AppError> {
    let types = aphel_sdk::tasks::types()?
        .list(
            ctx,
            false,
            ListWindow {
                offset: 0,
                limit: 200,
            },
        )
        .await?
        .items;
    let activity_type_id = types
        .iter()
        .find(|activity_type| activity_type.key == task.activity_type_key)
        .or_else(|| {
            types
                .iter()
                .find(|activity_type| activity_type.key == "task")
        })
        .or_else(|| types.first())
        .map(|activity_type| activity_type.id.to_string())
        .ok_or_else(|| AppError::Validation("there is no active activity type".to_owned()))?;
    let id = aphel_sdk::tasks::writer()?
        .create(
            ctx,
            CreateActivityCommand {
                title: task.title,
                activity_type_id,
                due_at: Some(
                    task.due_at
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                ),
                owner_id: Some(task.owner_id.into_inner()),
                record_type: Some("people.person".to_owned()),
                record_id: Some(task.contact_id.to_string()),
                description: task.description,
                ..CreateActivityCommand::default()
            },
        )
        .await?;
    Ok(id)
}

async fn step_running(ctx: &Ctx<'_>, step: RunningStep) -> Result<(), AppError> {
    if !ctx.app().hooks().has(StepRunning) {
        return Ok(());
    }
    let mut tx = ctx.begin().await?;
    let result = ctx.run_hook(StepRunning, &mut tx, KEY, &step).await;
    tx.rollback().await?;
    result
}

async fn create_task(
    ctx: &Ctx<'_>,
    step: &SequenceStep,
    email: &SequenceEmail,
    enrollment: &EnrollmentView,
    now: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    let (title, description) = if step.kind == StepKind::ManualEmail {
        let draft = render_draft(ctx, email).await?;
        let title = if step.title.is_empty() {
            draft.subject.clone()
        } else {
            step.title.clone()
        };
        let mut description = draft.subject;
        if !draft.body_text.trim().is_empty() {
            description.push_str("\n\n");
            description.push_str(draft.body_text.trim());
        }
        (title, Some(description))
    } else {
        (step.title.clone(), None)
    };
    let title = if title.trim().is_empty() {
        enrollment.sequence_name.clone()
    } else {
        title
    };
    create_activity(
        ctx,
        SequenceTask {
            activity_type_key: step.kind.activity_type_key().unwrap_or("task"),
            title: truncate(&title, MAX_TITLE_LENGTH),
            description,
            owner_id: enrollment.enrolled_by.clone(),
            contact_id: enrollment.contact_id,
            due_at: now,
        },
    )
    .await
}

fn new_run(
    enrollment: &EnrollmentView,
    step: &SequenceStep,
    mailbox_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> StepRun {
    StepRun {
        id: Uuid::now_v7(),
        sequence_id: enrollment.sequence_id,
        enrollment_id: enrollment.id,
        step_id: Some(step.id),
        position: enrollment.current_position,
        kind: step.kind,
        status: RunStatus::Running,
        contact_id: enrollment.contact_id,
        mailbox_id,
        email_id: None,
        thread_id: None,
        activity_id: None,
        failure_reason: None,
        executed_at: now,
    }
}

fn failure(run: StepRun, error: AppError, can_retry: bool) -> Outcome {
    if can_retry && !is_definitive(&error) {
        return Outcome::Retry(error);
    }
    tracing::warn!(error = %error, enrollment_id = %run.enrollment_id, "sequence step failed");
    Outcome::Exit(
        ExitReason::SendFailed,
        Some(StepRun {
            status: RunStatus::Failed,
            failure_reason: Some(truncate(error.client_message(), MAX_FAILURE_REASON_LENGTH)),
            ..run
        }),
    )
}

async fn claim(ctx: &Ctx<'_>, run: &StepRun) -> Result<bool, AppError> {
    let mut tx = ctx.begin().await?;
    let claimed = EnrollmentStore::of(ctx).claim_run(&mut tx, run).await?;
    tx.commit().await?;
    Ok(claimed)
}

async fn recover(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    sequence: &SequenceView,
    run: StepRun,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let outcome = match run.status {
        RunStatus::Sent | RunStatus::Created => Outcome::Advance(run),
        RunStatus::Failed => Outcome::Exit(ExitReason::SendFailed, None),
        RunStatus::Running => Outcome::Advance(StepRun {
            status: RunStatus::Failed,
            failure_reason: Some("the step was interrupted before it finished".to_owned()),
            ..run
        }),
    };
    conclude(ctx, enrollment, sequence, outcome, now).await
}

async fn conclude(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    sequence: &SequenceView,
    outcome: Outcome,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    match outcome {
        Outcome::Advance(run) => advance(ctx, enrollment, sequence, Some(run), now, now).await,
        Outcome::Retry(error) => {
            let mut tx = ctx.begin().await?;
            EnrollmentStore::of(ctx)
                .release_run(&mut tx, enrollment.id, enrollment.current_position)
                .await?;
            tx.commit().await?;
            Err(error)
        }
        Outcome::Exit(reason, run) => {
            if let Some(run) = run {
                let mut tx = ctx.begin().await?;
                EnrollmentStore::of(ctx).save_run(&mut tx, &run).await?;
                tx.commit().await?;
            }
            exit(ctx, enrollment, reason, now).await
        }
    }
}

async fn reschedule(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    let enrollments = EnrollmentStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let Some(mut current) = enrollments.lock(&mut tx, enrollment.id).await? else {
        return Ok(());
    };
    if current.status != EnrollmentStatus::Active
        || current.current_position != enrollment.current_position
    {
        return Ok(());
    }
    current.next_run_at = Some(at);
    enrollments.save(&mut tx, &current).await?;
    tx.commit().await?;
    Ok(())
}

async fn advance(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    sequence: &SequenceView,
    run: Option<StepRun>,
    next_run_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let enrollments = EnrollmentStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let Some(mut current) = enrollments.lock(&mut tx, enrollment.id).await? else {
        return Ok(());
    };
    if current.status != EnrollmentStatus::Active
        || current.current_position != enrollment.current_position
    {
        return Ok(());
    }
    let before = current.clone();
    if let Some(run) = &run {
        enrollments.save_run(&mut tx, run).await?;
        if let Some(email_id) = run.email_id {
            current.last_email_id = Some(email_id);
        }
        if run.mailbox_id.is_some() {
            current.mailbox_id = run.mailbox_id;
        }
    }
    current.current_position += 1;
    current.updated_at = now;
    let finished = current.current_position as usize >= sequence.steps.len();
    if finished {
        current.status = EnrollmentStatus::Completed;
        current.completed_at = Some(now);
        current.next_run_at = None;
    } else {
        current.next_run_at = Some(next_run_at);
    }
    enrollments.save(&mut tx, &current).await?;
    if let Some(run) = &run {
        let payload = json!({
            "sequence_id": current.sequence_id,
            "enrollment_id": current.id,
            "contact_id": current.contact_id,
            "position": run.position,
            "kind": run.kind.as_str(),
            "status": run.status.as_str(),
            "email_id": run.email_id,
            "activity_id": run.activity_id,
            "records": [{ "record_type": "people.person", "record_id": current.contact_id }],
        });
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::System, SEQUENCE_STEP_EXECUTED, now)
                    .with_record(contact_ref(current.contact_id))
                    .with_payload(payload),
            )
            .await?;
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(Actor::System, SEQUENCE_STEP_EXECUTED, now)
                    .with_record(enrollment_ref(current.id))
                    .with_changes(vec![
                        FieldChange::new("position", json!(run.position), json!(run.position + 1)),
                        FieldChange::new("kind", Value::Null, json!(run.kind.as_str())),
                        FieldChange::new("status", Value::Null, json!(run.status.as_str())),
                    ]),
            )
            .await?;
    }
    if finished {
        publish_enrollment_event(
            ctx,
            &mut tx,
            EnrollmentMutation {
                actor: &Actor::System,
                event_type: SEQUENCE_COMPLETED,
                enrollment: &current,
                changes: status_changes(&before, &current),
                ip: None,
                now,
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn complete(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let enrollments = EnrollmentStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let Some(mut current) = enrollments.lock(&mut tx, enrollment.id).await? else {
        return Ok(());
    };
    if current.status != EnrollmentStatus::Active {
        return Ok(());
    }
    let before = current.clone();
    current.status = EnrollmentStatus::Completed;
    current.completed_at = Some(now);
    current.next_run_at = None;
    current.updated_at = now;
    enrollments.save(&mut tx, &current).await?;
    publish_enrollment_event(
        ctx,
        &mut tx,
        EnrollmentMutation {
            actor: &Actor::System,
            event_type: SEQUENCE_COMPLETED,
            enrollment: &current,
            changes: status_changes(&before, &current),
            ip: None,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn exit(
    ctx: &Ctx<'_>,
    enrollment: &EnrollmentView,
    reason: ExitReason,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    exit_enrollment(ctx, enrollment.id, reason, now).await
}

pub(crate) async fn exit_enrollment(
    ctx: &Ctx<'_>,
    enrollment_id: Uuid,
    reason: ExitReason,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let enrollments = EnrollmentStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let Some(mut current) = enrollments.lock(&mut tx, enrollment_id).await? else {
        return Ok(());
    };
    if !current.status.is_live() {
        return Ok(());
    }
    let before = current.clone();
    mark_exited(&mut current, reason, now);
    current.updated_at = now;
    enrollments.save(&mut tx, &current).await?;
    publish_enrollment_event(
        ctx,
        &mut tx,
        EnrollmentMutation {
            actor: &Actor::System,
            event_type: SEQUENCE_EXITED,
            enrollment: &current,
            changes: status_changes(&before, &current),
            ip: None,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
