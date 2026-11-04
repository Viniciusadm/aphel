use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CreateNoteCommand, ListNotesQuery, NOTE_CREATED, NOTE_DELETED, NOTE_PINNED, NOTE_UNPINNED,
    NOTE_UPDATED, NoteData, NoteView, NoteVisibility, UpdateNoteCommand, normalize_note_body,
    note_not_found,
};
use crate::store::NoteStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;
use aphel_sdk::messaging::mentions::extract_mentions;
const MAX_MENTIONS_PER_NOTE: usize = 50;

fn record_access(ctx: &Ctx<'_>) -> MySqlRecordAccessChecker {
    MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
}

const BODY_FIELD: [&str; 1] = ["body"];

#[derive(Debug, Serialize)]
struct NoteSnapshot<'a> {
    body: &'a str,
    visibility: &'static str,
    pinned: bool,
}

impl<'a> From<&'a NoteData> for NoteSnapshot<'a> {
    fn from(data: &'a NoteData) -> Self {
        Self {
            body: &data.body,
            visibility: data.visibility.as_str(),
            pinned: data.pinned,
        }
    }
}

fn note_denied(action: &str) -> AppError {
    AppError::forbidden_code(
        PERMISSION_DENIED,
        format!("missing permission to {action} this note"),
    )
}

fn parse_visibility(value: &str, violations: &mut Vec<FieldViolation>) -> Option<NoteVisibility> {
    let visibility = NoteVisibility::parse(value.trim());
    if visibility.is_none() {
        violations.push(FieldViolation::new("visibility", "must be team or private"));
    }
    visibility
}

fn mentioned_users(body: &str) -> Vec<UserId> {
    extract_mentions(body)
        .into_iter()
        .filter_map(|id| UserId::new(id).ok())
        .collect()
}

fn announced_mentions(data: &NoteData, previous: Option<&NoteData>) -> Vec<UserId> {
    if data.visibility.is_private() {
        return Vec::new();
    }
    let already: Vec<UserId> = previous
        .filter(|previous| !previous.visibility.is_private())
        .map(|previous| mentioned_users(&previous.body))
        .unwrap_or_default();
    mentioned_users(&data.body)
        .into_iter()
        .filter(|user| !already.contains(user))
        .collect()
}

fn diff_options(private: bool) -> DiffOptions<'static> {
    if private {
        DiffOptions::default().redacting(&BODY_FIELD)
    } else {
        DiffOptions::default()
    }
}

pub async fn list(
    ctx: &Ctx<'_>,
    query: ListNotesQuery,
    window: ListWindow,
) -> Result<Listing<NoteView>, AppError> {
    ctx.scope.require(&Permission::NOTES_READ)?;
    let record = RecordRef::parse(&query.record_type, &query.record_id)?;
    require_readable_record(ctx, &record).await?;
    NoteStore::of(ctx)
        .list_visible(ctx.scope, &record, window)
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<NoteView, AppError> {
    ctx.scope.require(&Permission::NOTES_READ)?;
    visible(ctx, id).await
}

pub async fn create(ctx: &Ctx<'_>, command: CreateNoteCommand) -> Result<NoteView, AppError> {
    ctx.scope.require(&Permission::NOTES_CREATE)?;
    let record = RecordRef::parse(&command.record_type, &command.record_id)?;
    if record.record_type == RecordType::NOTE {
        return Err(AppError::invalid_fields(
            "invalid note",
            vec![FieldViolation::new(
                "record_type",
                "notes cannot be attached to notes",
            )],
        ));
    }
    require_readable_record(ctx, &record).await?;
    let mut violations = Vec::new();
    let body = normalize_note_body(&command.body)
        .map_err(|message| violations.push(FieldViolation::new("body", message)))
        .ok();
    let visibility = match command.visibility.as_deref() {
        Some(value) => parse_visibility(value, &mut violations),
        None => Some(NoteVisibility::default()),
    };
    let (Some(body), Some(visibility), true) = (body, visibility, violations.is_empty()) else {
        return Err(AppError::invalid_fields("invalid note", violations));
    };
    let data = NoteData {
        body,
        visibility,
        pinned: command.pinned.unwrap_or(false),
    };
    validate_mentions(ctx, &data.body, &[]).await?;
    let mentions = announced_mentions(&data, None);
    let now = ctx.now();
    let id = Uuid::now_v7();
    let changes = diff_changes(
        &Value::Null,
        &NoteSnapshot::from(&data),
        &diff_options(data.visibility.is_private()),
    )?;
    let mut tx = ctx.begin().await?;
    NoteStore::of(ctx)
        .insert(&mut tx, ctx.scope, id, &record, &data, now)
        .await?;
    publish(
        ctx,
        &mut tx,
        id,
        &record,
        &data,
        &[NOTE_CREATED],
        &mentions,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    require_note(ctx, id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateNoteCommand,
) -> Result<NoteView, AppError> {
    let current = visible(ctx, id).await?;
    require_manage(ctx, &current, &Permission::NOTES_UPDATE, "update").await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut data = current.data();
    if let Some(body) = &command.body {
        match normalize_note_body(body) {
            Ok(body) => data.body = body,
            Err(message) => violations.push(FieldViolation::new("body", message)),
        }
    }
    if violations.is_empty() && data.body != current.body {
        validate_mentions(ctx, &data.body, &mentioned_users(&current.body)).await?;
    }
    if let Some(visibility) = command.visibility {
        data.visibility = visibility;
    }
    if let Some(pinned) = command.pinned {
        data.pinned = pinned;
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid note", violations));
    }
    if data.visibility != current.visibility && current.author_id() != Some(ctx.scope.user_id()) {
        return Err(note_denied("change the visibility of"));
    }
    write_update(ctx, &current, data, command.ip).await
}

pub(crate) async fn set_pinned(
    ctx: &Ctx<'_>,
    id: &str,
    pinned: bool,
    ip: Option<String>,
) -> Result<NoteView, AppError> {
    let current = visible(ctx, id).await?;
    require_manage(ctx, &current, &Permission::NOTES_UPDATE, "update").await?;
    let mut data = current.data();
    data.pinned = pinned;
    write_update(ctx, &current, data, ip).await
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let current = visible(ctx, id).await?;
    require_manage(ctx, &current, &Permission::NOTES_DELETE, "delete").await?;
    let data = current.data();
    let changes = diff_changes(
        &NoteSnapshot::from(&data),
        &Value::Null,
        &diff_options(data.visibility.is_private()),
    )?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !NoteStore::of(ctx)
        .soft_delete(&mut tx, current.id, now)
        .await?
    {
        return Err(note_not_found());
    }
    publish(
        ctx,
        &mut tx,
        current.id,
        &current.record,
        &data,
        &[NOTE_DELETED],
        &[],
        changes,
        ip,
    )
    .await?;
    Ok(tx.commit().await?)
}

async fn write_update(
    ctx: &Ctx<'_>,
    current: &NoteView,
    data: NoteData,
    ip: Option<String>,
) -> Result<NoteView, AppError> {
    let old = current.data();
    let private = old.visibility.is_private() || data.visibility.is_private();
    let changes = diff_changes(
        &NoteSnapshot::from(&old),
        &NoteSnapshot::from(&data),
        &diff_options(private),
    )?;
    if changes.is_empty() {
        return Ok(current.clone());
    }
    let mut events = Vec::new();
    let mut mentions = Vec::new();
    if old.body != data.body || old.visibility != data.visibility {
        events.push(NOTE_UPDATED);
        mentions = announced_mentions(&data, Some(&old));
    }
    if old.pinned != data.pinned {
        events.push(if data.pinned {
            NOTE_PINNED
        } else {
            NOTE_UNPINNED
        });
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    NoteStore::of(ctx)
        .update(&mut tx, current.id, &data, now)
        .await?;
    publish(
        ctx,
        &mut tx,
        current.id,
        &current.record,
        &data,
        &events,
        &mentions,
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    require_note(ctx, current.id).await
}

#[allow(clippy::too_many_arguments)]
async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    id: Uuid,
    record: &RecordRef,
    data: &NoteData,
    events: &[&'static str],
    mentions: &[UserId],
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let Some(action) = events.first() else {
        return Ok(());
    };
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let note = RecordRef::new(RecordType::NOTE, id);
    let payload = json!({
        "record_type": record.record_type.as_str(),
        "record_id": record.record_id.to_string(),
        "visibility": data.visibility.as_str(),
        "pinned": data.pinned,
    });
    for event in events {
        let mut payload = payload.clone();
        if matches!(*event, NOTE_CREATED | NOTE_UPDATED)
            && let Some(object) = payload.as_object_mut()
        {
            object.insert(
                "mentioned_user_ids".to_owned(),
                mentions.iter().map(|user| json!(user.as_str())).collect(),
            );
        }
        ctx.events()
            .publish(
                tx,
                DomainEvent::new(actor.clone(), *event, now)
                    .with_record(note.clone())
                    .with_payload(payload),
            )
            .await?;
    }
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, *action, now)
                .with_record(note)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

async fn validate_mentions(ctx: &Ctx<'_>, body: &str, known: &[UserId]) -> Result<(), AppError> {
    let mentioned = mentioned_users(body);
    if mentioned.len() > MAX_MENTIONS_PER_NOTE {
        return Err(AppError::invalid_fields(
            "invalid note",
            vec![FieldViolation::new(
                "body",
                format!("must mention at most {MAX_MENTIONS_PER_NOTE} users"),
            )],
        ));
    }
    let added: Vec<UserId> = mentioned
        .into_iter()
        .filter(|user| !known.contains(user))
        .collect();
    if added.is_empty() {
        return Ok(());
    }
    let members = NoteStore::of(ctx).active_members(&added).await?;
    if added.iter().all(|user| members.contains(user)) {
        return Ok(());
    }
    Err(AppError::invalid_fields(
        "invalid note",
        vec![FieldViolation::new(
            "body",
            "mentions users who are not active members of the organization",
        )],
    ))
}

async fn require_manage(
    ctx: &Ctx<'_>,
    note: &NoteView,
    permission: &Permission,
    action: &str,
) -> Result<(), AppError> {
    let filter = ctx.scope.record_filter(permission)?;
    let allowed = match (note.author_id(), &filter) {
        (Some(author), _) if author == ctx.scope.user_id() => true,
        _ if note.visibility.is_private() => false,
        (_, RecordScopeFilter::Organization) => true,
        (None, _) => false,
        (Some(author), filter) => NoteStore::of(ctx).author_within(author, filter).await?,
    };
    if allowed {
        Ok(())
    } else {
        Err(note_denied(action))
    }
}

async fn visible(ctx: &Ctx<'_>, id: &str) -> Result<NoteView, AppError> {
    let id = Uuid::parse_str(id).map_err(|_| note_not_found())?;
    let note = NoteStore::of(ctx)
        .find(id)
        .await?
        .filter(|note| note.is_visible_to(ctx.scope.user_id()))
        .ok_or_else(note_not_found)?;
    if !record_access(ctx).can_read(ctx.scope, &note.record).await? {
        return Err(note_not_found());
    }
    Ok(note)
}

async fn require_readable_record(ctx: &Ctx<'_>, record: &RecordRef) -> Result<(), AppError> {
    if record_access(ctx).can_read(ctx.scope, record).await? {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        format!(
            "missing permission to access {} records",
            record.record_type
        ),
    ))
}

async fn require_note(ctx: &Ctx<'_>, id: Uuid) -> Result<NoteView, AppError> {
    NoteStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(note_not_found)
}
