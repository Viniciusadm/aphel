use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEFAULT_DAILY_LIMIT, EnrollmentSource, EnrollmentStatus, ExitReason, MAX_DAILY_LIMIT,
    MAX_STEPS, MAX_WAIT_DAYS, SendWindow, StepKind, format_clock, normalize_sequence_name,
    parse_clock,
};
use crate::store::{DirectoryStore, EnrollmentStore, SequenceStore};
use crate::{
    EnrollCommand, EnrollmentFilter, EnrollmentView, MAX_BODY_BYTES, MAX_DESCRIPTION_LENGTH,
    MAX_SUBJECT_LENGTH, MAX_TITLE_LENGTH, NO_MAILBOX_AVAILABLE, NewEnrollment,
    SEQUENCE_ALREADY_ENROLLED, SEQUENCE_CONTACT_OBJECTED, SEQUENCE_CONTACT_WITHOUT_EMAIL,
    SEQUENCE_CREATED, SEQUENCE_DELETED, SEQUENCE_EMPTY, SEQUENCE_ENROLLED,
    SEQUENCE_ENROLLMENT_FINISHED, SEQUENCE_ENROLLMENT_NOT_FOUND, SEQUENCE_ENROLLMENT_PAUSED,
    SEQUENCE_ENROLLMENT_RECORD_TYPE, SEQUENCE_ENROLLMENT_RESUMED, SEQUENCE_EXITED,
    SEQUENCE_INACTIVE, SEQUENCE_KEY_TAKEN, SEQUENCE_NOT_FOUND, SEQUENCE_RECORD_TYPE,
    SEQUENCE_UPDATED, SaveSequenceCommand, SequenceEnrollmentRequest, SequenceListQuery,
    SequenceMetrics, SequenceStep, SequenceSummary, SequenceView, StepInput, StepMetrics,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::configuration::{is_config_key, key_from_name};
use aphel_sdk::email::address_hash;
use aphel_sdk::email::html::sanitize_html;
use aphel_sdk::platform::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_sdk::platform::application::audit::{DiffOptions, diff_values};
use aphel_sdk::platform::application::files::RecordAccessChecker;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::records::MySqlRecordAccessChecker;

use aphel_sdk::platform::application::shared::parse_uuid as parse_id;

pub(crate) fn sequence_not_found() -> AppError {
    AppError::not_found_code(SEQUENCE_NOT_FOUND, "sequence was not found")
}

pub(crate) fn enrollment_not_found() -> AppError {
    AppError::not_found_code(
        SEQUENCE_ENROLLMENT_NOT_FOUND,
        "sequence enrollment was not found",
    )
}

fn contact_not_found() -> AppError {
    AppError::not_found_code("contact_not_found", "contact was not found")
}

pub(crate) fn require_sequence_reader(scope: &AccessScope) -> Result<RecordScopeFilter, AppError> {
    if scope.require(&Permission::SEQUENCES_MANAGE).is_ok() {
        return Ok(RecordScopeFilter::Organization);
    }
    scope.record_filter(&Permission::SEQUENCES_READ)
}

pub(crate) fn contact_ref(contact_id: Uuid) -> RecordRef {
    RecordRef::new(RecordType::CONTACT, contact_id)
}

pub(crate) fn enrollment_ref(enrollment_id: Uuid) -> RecordRef {
    RecordRef::new(SEQUENCE_ENROLLMENT_RECORD_TYPE, enrollment_id)
}

pub(crate) fn enrollment_payload(enrollment: &EnrollmentView) -> Value {
    json!({
        "sequence_id": enrollment.sequence_id,
        "sequence_key": enrollment.sequence_key,
        "sequence_name": enrollment.sequence_name,
        "enrollment_id": enrollment.id,
        "contact_id": enrollment.contact_id,
        "enrolled_by": enrollment.enrolled_by.as_str(),
        "source": enrollment.source.as_str(),
        "status": enrollment.status.as_str(),
        "position": enrollment.current_position,
        "exit_reason": enrollment.exit_reason.map(|reason| reason.as_str()),
        "records": [{ "record_type": aphel_contract_crm::keys::CONTACT, "record_id": enrollment.contact_id }],
    })
}

fn snapshot(sequence: &SequenceView) -> Value {
    json!({
        "key": sequence.key,
        "name": sequence.name,
        "description": sequence.description,
        "is_active": sequence.is_active,
        "window_start": format_clock(sequence.window.start_minute),
        "window_end": format_clock(sequence.window.end_minute),
        "daily_limit": sequence.daily_limit,
        "steps": sequence
            .steps
            .iter()
            .map(|step| json!({
                "kind": step.kind.as_str(),
                "wait_days": step.wait_days,
                "template_id": step.template_id,
                "subject": step.subject,
                "has_body": step.body_html.as_deref().is_some_and(|body| !body.is_empty()),
                "title": step.title,
            }))
            .collect::<Vec<_>>(),
    })
}

fn clean_line(value: &str, max: usize) -> String {
    value
        .replace(['\r', '\n'], " ")
        .trim()
        .chars()
        .take(max)
        .collect()
}

pub fn validate_steps(
    inputs: &[StepInput],
    current: &[SequenceStep],
    templates: &HashMap<Uuid, String>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<SequenceStep> {
    if inputs.len() > MAX_STEPS {
        violations.push(FieldViolation::new(
            "steps",
            format!("must have at most {MAX_STEPS} steps"),
        ));
    }
    let mut steps = Vec::new();
    let mut used = HashSet::new();
    let mut threaded = false;
    for (index, input) in inputs.iter().enumerate().take(MAX_STEPS) {
        let path = |field: &str| format!("steps.{index}.{field}");
        let Some(kind) = StepKind::parse(input.kind.trim()) else {
            violations.push(FieldViolation::new(
                path("kind"),
                "must be auto_email, manual_email, call, linkedin, task or wait",
            ));
            continue;
        };
        let id = input
            .id
            .as_deref()
            .and_then(parse_id)
            .filter(|id| current.iter().any(|step| step.id == *id) && used.insert(*id))
            .unwrap_or_else(Uuid::now_v7);
        let wait_days = if kind == StepKind::Wait {
            match input.wait_days {
                Some(days) if (1..=MAX_WAIT_DAYS).contains(&days) => days,
                _ => {
                    violations.push(FieldViolation::new(
                        path("wait_days"),
                        format!("must be between 1 and {MAX_WAIT_DAYS} business days"),
                    ));
                    0
                }
            }
        } else {
            0
        };
        let template_id = if kind.is_email() {
            match input.template_id.as_deref().map(str::trim) {
                None | Some("") => None,
                Some(value) => match parse_id(value).filter(|id| templates.contains_key(id)) {
                    Some(id) => Some(id),
                    None => {
                        violations.push(FieldViolation::new(
                            path("template_id"),
                            "must be an e-mail template of the organization",
                        ));
                        None
                    }
                },
            }
        } else {
            None
        };
        let subject = if kind.is_email() {
            clean_line(input.subject.as_deref().unwrap_or_default(), usize::MAX)
        } else {
            String::new()
        };
        if subject.chars().count() > MAX_SUBJECT_LENGTH {
            violations.push(FieldViolation::new(
                path("subject"),
                format!("must have at most {MAX_SUBJECT_LENGTH} characters"),
            ));
        }
        let body_html = if kind.is_email() {
            input
                .body_html
                .as_deref()
                .map(str::trim)
                .filter(|body| !body.is_empty())
                .map(sanitize_html)
        } else {
            None
        };
        if body_html
            .as_ref()
            .is_some_and(|body| body.len() > MAX_BODY_BYTES)
        {
            violations.push(FieldViolation::new(
                path("body_html"),
                format!("must have at most {MAX_BODY_BYTES} bytes"),
            ));
        }
        if kind.is_email() && template_id.is_none() {
            if body_html.is_none() {
                violations.push(FieldViolation::new(
                    path("body_html"),
                    "requires template_id or body_html",
                ));
            }
            if subject.is_empty() && !(kind == StepKind::AutoEmail && threaded) {
                violations.push(FieldViolation::new(
                    path("subject"),
                    "requires template_id or a subject (follow-up e-mails may reuse the conversation subject)",
                ));
            }
        }
        if kind == StepKind::AutoEmail {
            threaded = true;
        }
        let title = if kind == StepKind::Wait || kind == StepKind::AutoEmail {
            String::new()
        } else {
            clean_line(input.title.as_deref().unwrap_or_default(), usize::MAX)
        };
        if title.chars().count() > MAX_TITLE_LENGTH {
            violations.push(FieldViolation::new(
                path("title"),
                format!("must have at most {MAX_TITLE_LENGTH} characters"),
            ));
        }
        if matches!(kind, StepKind::Call | StepKind::Linkedin | StepKind::Task) && title.is_empty()
        {
            violations.push(FieldViolation::new(path("title"), "must not be empty"));
        }
        steps.push(SequenceStep {
            id,
            position: u32::try_from(index).unwrap_or(u32::MAX),
            kind,
            wait_days,
            template_id,
            subject,
            body_html,
            title,
        });
    }
    steps
}

pub fn parse_window(
    start: Option<&str>,
    end: Option<&str>,
    current: SendWindow,
    violations: &mut Vec<FieldViolation>,
) -> SendWindow {
    let mut minute = |field: &str, value: Option<&str>, fallback: u16| match value {
        None => Some(fallback),
        Some(value) => match parse_clock(value) {
            Some(minute) => Some(minute),
            None => {
                violations.push(FieldViolation::new(field, "must be a time as HH:MM"));
                None
            }
        },
    };
    let start = minute("window_start", start, current.start_minute);
    let end = minute("window_end", end, current.end_minute);
    match (start, end) {
        (Some(start), Some(end)) => match SendWindow::new(start, end) {
            Ok(window) => window,
            Err(message) => {
                violations.push(FieldViolation::new("window_end", message));
                current
            }
        },
        _ => current,
    }
}

fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: SequenceListQuery,
    window: ListWindow,
) -> Result<Listing<SequenceSummary>, AppError> {
    let visibility = require_sequence_reader(ctx.scope)?;
    let listing = SequenceStore::of(ctx).list(&query, window).await?;
    let ids: Vec<Uuid> = listing.items.iter().map(|sequence| sequence.id).collect();
    let mut counts = EnrollmentStore::of(ctx).counts(&ids, &visibility).await?;
    Ok(Listing {
        items: listing
            .items
            .into_iter()
            .map(|sequence| SequenceSummary {
                counts: counts.remove(&sequence.id).unwrap_or_default(),
                sequence,
            })
            .collect(),
        total: listing.total,
    })
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<SequenceSummary, AppError> {
    let visibility = require_sequence_reader(ctx.scope)?;
    let sequence = find(ctx, id).await?;
    let counts = EnrollmentStore::of(ctx)
        .counts(&[sequence.id], &visibility)
        .await?
        .remove(&sequence.id)
        .unwrap_or_default();
    Ok(SequenceSummary { sequence, counts })
}

pub(crate) async fn metrics(ctx: &Ctx<'_>, id: &str) -> Result<SequenceMetrics, AppError> {
    let visibility = require_sequence_reader(ctx.scope)?;
    let sequence = find(ctx, id).await?;
    let enrollments = EnrollmentStore::of(ctx);
    let counts = enrollments
        .counts(&[sequence.id], &visibility)
        .await?
        .remove(&sequence.id)
        .unwrap_or_default();
    let mut measured: HashMap<u32, StepMetrics> = enrollments
        .metrics(sequence.id, &visibility)
        .await?
        .into_iter()
        .map(|metrics| (metrics.position, metrics))
        .collect();
    let steps = sequence
        .steps
        .iter()
        .map(|step| {
            let metrics = measured.remove(&step.position).unwrap_or_default();
            StepMetrics {
                position: step.position,
                step_id: Some(step.id),
                kind: Some(step.kind),
                ..metrics
            }
        })
        .collect();
    Ok(SequenceMetrics { counts, steps })
}

pub(crate) async fn find(ctx: &Ctx<'_>, id: &str) -> Result<SequenceView, AppError> {
    let id = parse_id(id).ok_or_else(sequence_not_found)?;
    SequenceStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(sequence_not_found)
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: SaveSequenceCommand,
) -> Result<SequenceSummary, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SEQUENCES_MANAGE)?;
    let now = ctx.now();
    let empty = SequenceView {
        id: Uuid::now_v7(),
        key: String::new(),
        name: String::new(),
        description: None,
        is_active: true,
        window: SendWindow::default(),
        daily_limit: DEFAULT_DAILY_LIMIT,
        steps: Vec::new(),
        created_by: Some(scope.user_id().clone()),
        updated_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let mut violations = Vec::new();
    if command
        .name
        .as_deref()
        .is_none_or(|name| name.trim().is_empty())
    {
        violations.push(FieldViolation::new("name", "must not be empty"));
    }
    let sequences = SequenceStore::of(ctx);
    let sequence = apply(&sequences, &empty, command.clone(), &mut violations).await?;
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid sequence", violations));
    }
    ensure_key_free(&sequences, &sequence.key, None).await?;
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(SEQUENCE_RECORD_TYPE, sequence.id);
    let mut tx = ctx.begin().await?;
    sequences.insert(&mut tx, &sequence).await?;
    let changes = diff_values(&json!({}), &snapshot(&sequence), &DiffOptions::default())?;
    publish(
        ctx,
        &mut tx,
        SequenceMutation {
            actor: &actor,
            event_type: SEQUENCE_CREATED,
            record: &record,
            payload: json!({ "sequence_id": sequence.id, "key": sequence.key, "name": sequence.name }),
            changes,
            ip: command.ip,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(SequenceSummary {
        sequence,
        counts: Default::default(),
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: SaveSequenceCommand,
) -> Result<SequenceSummary, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SEQUENCES_MANAGE)?;
    let current = find(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let sequences = SequenceStore::of(ctx);
    let mut violations = Vec::new();
    let mut sequence = apply(&sequences, &current, command.clone(), &mut violations).await?;
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid sequence", violations));
    }
    if sequence.key != current.key {
        ensure_key_free(&sequences, &sequence.key, Some(current.id)).await?;
    }
    let changes = diff_values(
        &snapshot(&current),
        &snapshot(&sequence),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return get(ctx, &current.id.to_string()).await;
    }
    let now = ctx.now();
    sequence.updated_at = now;
    sequence.updated_by = Some(scope.user_id().clone());
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(SEQUENCE_RECORD_TYPE, sequence.id);
    let mut tx = ctx.begin().await?;
    sequences.update(&mut tx, &sequence).await?;
    publish(
        ctx,
        &mut tx,
        SequenceMutation {
            actor: &actor,
            event_type: SEQUENCE_UPDATED,
            record: &record,
            payload: json!({
                "sequence_id": sequence.id,
                "key": sequence.key,
                "changes": changes.iter().map(|change| json!({
                    "field": change.field,
                    "old": change.old,
                    "new": change.new,
                })).collect::<Vec<_>>(),
            }),
            changes,
            ip: command.ip,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    get(ctx, &sequence.id.to_string()).await
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SEQUENCES_MANAGE)?;
    let current = find(ctx, id).await?;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(SEQUENCE_RECORD_TYPE, current.id);
    let mut tx = ctx.begin().await?;
    if !SequenceStore::of(ctx).delete(&mut tx, current.id).await? {
        return Err(sequence_not_found());
    }
    let changes = diff_values(&snapshot(&current), &json!({}), &DiffOptions::default())?;
    publish(
        ctx,
        &mut tx,
        SequenceMutation {
            actor: &actor,
            event_type: SEQUENCE_DELETED,
            record: &record,
            payload: json!({ "sequence_id": current.id, "key": current.key, "name": current.name }),
            changes,
            ip,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn ensure_key_free(
    sequences: &SequenceStore,
    key: &str,
    except: Option<Uuid>,
) -> Result<(), AppError> {
    if sequences.key_taken(key, except).await? {
        return Err(AppError::conflict_code(
            SEQUENCE_KEY_TAKEN,
            "another sequence already uses this key",
        ));
    }
    Ok(())
}

async fn apply(
    sequences: &SequenceStore,
    current: &SequenceView,
    command: SaveSequenceCommand,
    violations: &mut Vec<FieldViolation>,
) -> Result<SequenceView, AppError> {
    let mut sequence = current.clone();
    if let Some(name) = command.name.as_deref() {
        match normalize_sequence_name(name) {
            Ok(name) => sequence.name = name,
            Err(message) => violations.push(FieldViolation::new("name", message)),
        }
    }
    match command.key.as_deref().map(str::trim) {
        Some(key) if !key.is_empty() => {
            if is_config_key(key) {
                sequence.key = key.to_owned();
            } else {
                violations.push(FieldViolation::new(
                    "key",
                    "must be snake_case: lowercase letters, digits and underscores, starting with a letter (max 64)",
                ));
            }
        }
        _ if sequence.key.is_empty() => {
            sequence.key = free_key(sequences, &sequence.name).await?;
        }
        _ => {}
    }
    if let Some(description) = command.description {
        let description = description
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if description
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_DESCRIPTION_LENGTH)
        {
            violations.push(FieldViolation::new(
                "description",
                format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
            ));
        }
        sequence.description = description;
    }
    if let Some(is_active) = command.is_active {
        sequence.is_active = is_active;
    }
    sequence.window = parse_window(
        command.window_start.as_deref(),
        command.window_end.as_deref(),
        sequence.window,
        violations,
    );
    if let Some(limit) = command.daily_limit {
        if (1..=MAX_DAILY_LIMIT).contains(&limit) {
            sequence.daily_limit = limit;
        } else {
            violations.push(FieldViolation::new(
                "daily_limit",
                format!("must be between 1 and {MAX_DAILY_LIMIT}"),
            ));
        }
    }
    if let Some(steps) = command.steps {
        let templates = sequences.template_keys().await?;
        sequence.steps = validate_steps(&steps, &current.steps, &templates, violations);
    }
    Ok(sequence)
}

async fn free_key(sequences: &SequenceStore, name: &str) -> Result<String, AppError> {
    let base: String = key_from_name(name).chars().take(56).collect();
    let base = base.trim_end_matches('_').to_owned();
    let mut candidate = base.clone();
    let mut suffix = 2;
    while sequences.key_taken(&candidate, None).await? {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

struct SequenceMutation<'a> {
    actor: &'a Actor,
    event_type: &'static str,
    record: &'a RecordRef,
    payload: Value,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    mutation: SequenceMutation<'_>,
) -> Result<(), AppError> {
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(mutation.actor.clone(), mutation.event_type, mutation.now)
                .with_payload(mutation.payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(mutation.actor.clone(), mutation.event_type, mutation.now)
                .with_record(mutation.record.clone())
                .with_changes(mutation.changes)
                .with_ip(mutation.ip),
        )
        .await
}

pub async fn enroll(ctx: &Ctx<'_>, command: EnrollCommand) -> Result<EnrollmentView, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SEQUENCES_ENROLL)?;
    let sequence_id = parse_id(&command.sequence_id).ok_or_else(sequence_not_found)?;
    let sequence = SequenceStore::of(ctx)
        .find(sequence_id)
        .await?
        .ok_or_else(sequence_not_found)?;
    if !sequence.is_active {
        return Err(AppError::validation_code(
            SEQUENCE_INACTIVE,
            "the sequence is inactive",
        ));
    }
    if !sequence
        .steps
        .iter()
        .any(|step| step.kind != StepKind::Wait)
    {
        return Err(AppError::validation_code(
            SEQUENCE_EMPTY,
            "the sequence has no steps to execute",
        ));
    }
    let contact_id = parse_id(&command.contact_id).ok_or_else(contact_not_found)?;
    if !record_access(ctx)
        .can_read(scope, &contact_ref(contact_id))
        .await?
    {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "missing permission to access this contact",
        ));
    }
    let directory = DirectoryStore::of(ctx);
    let contact = directory
        .contact(ctx, contact_id)
        .await?
        .filter(|contact| !contact.deleted)
        .ok_or_else(contact_not_found)?;
    let enroller = command
        .enroll_as
        .clone()
        .unwrap_or_else(|| scope.user_id().clone());
    if &enroller != scope.user_id() && !directory.is_active_member(&enroller).await? {
        return Err(AppError::validation_code(
            NO_MAILBOX_AVAILABLE,
            "the user that would send the sequence is not an active member",
        ));
    }
    let requested_mailbox = match command
        .mailbox_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Some(parse_id(id).ok_or_else(|| {
            AppError::not_found_code("mailbox_not_found", "mailbox was not found")
        })?),
        None => None,
    };
    let mailbox_id = directory
        .sendable_mailbox(&enroller, requested_mailbox)
        .await?;
    if requested_mailbox.is_some() && mailbox_id.is_none() {
        return Err(AppError::not_found_code(
            "mailbox_not_found",
            "mailbox was not found",
        ));
    }
    if sequence.has_auto_email() {
        let Some(email) = contact.email.as_deref() else {
            return Err(AppError::validation_code(
                SEQUENCE_CONTACT_WITHOUT_EMAIL,
                "the contact has no e-mail address",
            ));
        };
        if directory.has_objection(&address_hash(email)).await? {
            return Err(AppError::validation_code(
                SEQUENCE_CONTACT_OBJECTED,
                "the contact objected to receiving tracked e-mails",
            ));
        }
        if mailbox_id.is_none() {
            return Err(AppError::validation_code(
                NO_MAILBOX_AVAILABLE,
                "connect a mailbox before enrolling people in a sequence that sends e-mails",
            ));
        }
    }
    let now = ctx.now();
    let enrollment = NewEnrollment {
        id: Uuid::now_v7(),
        sequence_id: sequence.id,
        contact_id: contact.id,
        enrolled_by: enroller,
        mailbox_id,
        source: command.source,
        next_run_at: now,
    };
    let actor = Actor::from_scope(scope);
    let enrollments = EnrollmentStore::of(ctx);
    let mut tx = ctx.begin().await?;
    if !enrollments.insert(&mut tx, &enrollment).await? {
        return Err(AppError::conflict_code(
            SEQUENCE_ALREADY_ENROLLED,
            "the contact is already in this sequence",
        ));
    }
    let view = EnrollmentView {
        id: enrollment.id,
        sequence_id: sequence.id,
        sequence_key: sequence.key.clone(),
        sequence_name: sequence.name.clone(),
        contact_id: contact.id,
        contact_name: contact.name.clone(),
        contact_email: contact.email.clone(),
        enrolled_by: enrollment.enrolled_by.clone(),
        enrolled_by_name: None,
        mailbox_id,
        source: command.source,
        status: EnrollmentStatus::Active,
        current_position: 0,
        next_run_at: Some(now),
        last_email_id: None,
        exit_reason: None,
        exit_position: None,
        paused_at: None,
        completed_at: None,
        exited_at: None,
        created_at: now,
        updated_at: now,
    };
    publish_enrollment_event(
        ctx,
        &mut tx,
        EnrollmentMutation {
            actor: &actor,
            event_type: SEQUENCE_ENROLLED,
            enrollment: &view,
            changes: vec![
                FieldChange::new("sequence_id", Value::Null, json!(sequence.id)),
                FieldChange::new("contact_id", Value::Null, json!(contact.id)),
                FieldChange::new("enrolled_by", Value::Null, json!(view.enrolled_by.as_str())),
                FieldChange::new("source", Value::Null, json!(command.source.as_str())),
            ],
            ip: command.ip,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    let mut result = enrollments
        .find(enrollment.id)
        .await?
        .ok_or_else(enrollment_not_found)?;
    crate::contacts::hydrate(ctx, std::slice::from_mut(&mut result)).await?;
    Ok(result)
}

pub(crate) async fn enroll_for_automation(
    ctx: &Ctx<'_>,
    request: SequenceEnrollmentRequest,
) -> Result<Uuid, AppError> {
    let enroll_as = request
        .enroll_as
        .map(UserId::new)
        .transpose()
        .map_err(|_| AppError::Validation("the record has no valid owner".to_owned()))?;
    Ok(enroll(
        ctx,
        EnrollCommand {
            sequence_id: request.sequence_id,
            contact_id: request.contact_id.to_string(),
            mailbox_id: None,
            source: EnrollmentSource::Automation,
            enroll_as,
            ip: None,
        },
    )
    .await?
    .id)
}

pub(crate) async fn list_enrollments(
    ctx: &Ctx<'_>,
    filter: EnrollmentFilter,
    window: ListWindow,
) -> Result<Listing<EnrollmentView>, AppError> {
    let visibility = require_sequence_reader(ctx.scope)?;
    if let Some(contact_id) = filter.contact_id
        && !record_access(ctx)
            .can_read(ctx.scope, &contact_ref(contact_id))
            .await?
    {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "missing permission to access this contact",
        ));
    }
    let mut listing = EnrollmentStore::of(ctx)
        .list(&filter, &visibility, window)
        .await?;
    crate::contacts::hydrate(ctx, &mut listing.items).await?;
    Ok(listing)
}

pub(crate) async fn pause(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<EnrollmentView, AppError> {
    transition(ctx, id, ip, |enrollment, now| {
        if enrollment.status != EnrollmentStatus::Active {
            return Err(not_live());
        }
        enrollment.status = EnrollmentStatus::Paused;
        enrollment.paused_at = Some(now);
        enrollment.next_run_at = None;
        Ok(SEQUENCE_ENROLLMENT_PAUSED)
    })
    .await
}

pub(crate) async fn resume(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<EnrollmentView, AppError> {
    transition(ctx, id, ip, |enrollment, now| {
        if enrollment.status != EnrollmentStatus::Paused {
            return Err(not_live());
        }
        enrollment.status = EnrollmentStatus::Active;
        enrollment.paused_at = None;
        enrollment.next_run_at = Some(now);
        Ok(SEQUENCE_ENROLLMENT_RESUMED)
    })
    .await
}

pub(crate) async fn remove(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<EnrollmentView, AppError> {
    transition(ctx, id, ip, |enrollment, now| {
        if !enrollment.status.is_live() {
            return Err(not_live());
        }
        mark_exited(enrollment, ExitReason::Removed, now);
        Ok(SEQUENCE_EXITED)
    })
    .await
}

async fn transition(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
    change: impl FnOnce(&mut EnrollmentView, DateTime<Utc>) -> Result<&'static str, AppError>,
) -> Result<EnrollmentView, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::SEQUENCES_ENROLL)?;
    let id = parse_id(id).ok_or_else(enrollment_not_found)?;
    let enrollments = EnrollmentStore::of(ctx);
    let visible = enrollments
        .list(
            &EnrollmentFilter {
                id: Some(id),
                ..EnrollmentFilter::default()
            },
            &visibility,
            ListWindow {
                offset: 0,
                limit: 1,
            },
        )
        .await?;
    if visible.items.is_empty() {
        return Err(enrollment_not_found());
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    let mut enrollment = enrollments
        .lock(&mut tx, id)
        .await?
        .ok_or_else(enrollment_not_found)?;
    let before = enrollment.clone();
    let event_type = change(&mut enrollment, now)?;
    enrollment.updated_at = now;
    enrollments.save(&mut tx, &enrollment).await?;
    publish_enrollment_event(
        ctx,
        &mut tx,
        EnrollmentMutation {
            actor: &actor,
            event_type,
            enrollment: &enrollment,
            changes: status_changes(&before, &enrollment),
            ip,
            now,
        },
    )
    .await?;
    tx.commit().await?;
    let mut result = enrollments
        .find(id)
        .await?
        .ok_or_else(enrollment_not_found)?;
    crate::contacts::hydrate(ctx, std::slice::from_mut(&mut result)).await?;
    Ok(result)
}

fn not_live() -> AppError {
    AppError::conflict_code(
        SEQUENCE_ENROLLMENT_FINISHED,
        "the enrollment is not in a state that allows this change",
    )
}

pub(crate) fn mark_exited(enrollment: &mut EnrollmentView, reason: ExitReason, now: DateTime<Utc>) {
    enrollment.status = EnrollmentStatus::Exited;
    enrollment.exit_reason = Some(reason);
    enrollment.exit_position = enrollment.current_position.checked_sub(1);
    enrollment.exited_at = Some(now);
    enrollment.next_run_at = None;
    enrollment.paused_at = None;
}

pub(crate) fn status_changes(before: &EnrollmentView, after: &EnrollmentView) -> Vec<FieldChange> {
    let mut changes = vec![FieldChange::new(
        "status",
        json!(before.status.as_str()),
        json!(after.status.as_str()),
    )];
    if before.exit_reason != after.exit_reason {
        changes.push(FieldChange::new(
            "exit_reason",
            json!(before.exit_reason.map(|reason| reason.as_str())),
            json!(after.exit_reason.map(|reason| reason.as_str())),
        ));
    }
    if before.current_position != after.current_position {
        changes.push(FieldChange::new(
            "position",
            json!(before.current_position),
            json!(after.current_position),
        ));
    }
    changes
}

pub(crate) struct EnrollmentMutation<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) event_type: &'a str,
    pub(crate) enrollment: &'a EnrollmentView,
    pub(crate) changes: Vec<FieldChange>,
    pub(crate) ip: Option<String>,
    pub(crate) now: DateTime<Utc>,
}

pub(crate) async fn publish_enrollment_event(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    mutation: EnrollmentMutation<'_>,
) -> Result<(), AppError> {
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(mutation.actor.clone(), mutation.event_type, mutation.now)
                .with_record(contact_ref(mutation.enrollment.contact_id))
                .with_payload(enrollment_payload(mutation.enrollment)),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(mutation.actor.clone(), mutation.event_type, mutation.now)
                .with_record(enrollment_ref(mutation.enrollment.id))
                .with_changes(mutation.changes)
                .with_ip(mutation.ip),
        )
        .await
}
