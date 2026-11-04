use aphel_contract_crm::CrmPermissions;
use aphel_kernel::application::shared::parse_uuid as parse_id;

pub(crate) mod booking;
pub(crate) mod crm;
pub(crate) mod distribution;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::service::booking::{booking_not_found, meeting_type_not_found};
use crate::store::BookingStore;
use crate::validation::{
    apply_availability, description, duration, invalid, location_value, name, parse_timezone,
    questions, random_suffix, slot_interval, slug, slugify, valid_slug,
};
use crate::{
    AVAILABILITY_RECORD, AVAILABILITY_UPDATED, Availability, AvailabilitySnapshot, Booking,
    BookingFilter, BookingListQuery, ContactFieldOption, DEFAULT_SLOT_INTERVAL_MINUTES,
    LocationKind, MEETING_TYPE_CREATED, MEETING_TYPE_DELETED, MEETING_TYPE_RECORD,
    MEETING_TYPE_SLUG_TAKEN, MEETING_TYPE_UPDATED, MeetingType, MeetingTypeFilter,
    MeetingTypeSnapshot, SaveAvailabilityCommand, SaveMeetingTypeCommand,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, RecordRef, ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{Permission, RecordScope};
use aphel_kernel::errors::{AppError, FieldViolation};

fn slug_taken() -> AppError {
    AppError::conflict_code(
        MEETING_TYPE_SLUG_TAKEN,
        "another booking link already uses this address",
    )
}

fn user_record(user_id: &UserId) -> Result<RecordRef, AppError> {
    let id = Uuid::parse_str(user_id.as_str())
        .map_err(|_| AppError::Internal("user id is not a UUID".to_owned()))?;
    Ok(RecordRef::new(AVAILABILITY_RECORD, id))
}

async fn target_user(ctx: &Ctx<'_>, user_id: Option<&str>) -> Result<UserId, AppError> {
    let filter = ctx.scope.record_filter(&Permission::BOOKING_MANAGE)?;
    let Some(raw) = user_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ctx.scope.user_id().clone());
    };
    let target = UserId::new(raw).map_err(|_| denied_user())?;
    if &target == ctx.scope.user_id() {
        return Ok(target);
    }
    if BookingStore::of(ctx).host(&target).await?.is_none()
        || !BookingStore::of(ctx).user_within(&target, &filter).await?
    {
        return Err(denied_user());
    }
    Ok(target)
}

async fn default_availability(ctx: &Ctx<'_>, user_id: &UserId) -> Result<Availability, AppError> {
    let organization = BookingStore::of(ctx).organization().await?;
    let timezone = BookingStore::of(ctx)
        .host(user_id)
        .await?
        .and_then(|host| host.timezone)
        .and_then(|timezone| parse_timezone(timezone.as_str()))
        .unwrap_or(organization.timezone);
    Ok(Availability::defaults(user_id.clone(), timezone))
}

async fn current_availability(ctx: &Ctx<'_>, user_id: &UserId) -> Result<Availability, AppError> {
    match BookingStore::of(ctx).availability(user_id).await? {
        Some(availability) => Ok(availability),
        None => default_availability(ctx, user_id).await,
    }
}

pub(crate) async fn availability(
    ctx: &Ctx<'_>,
    user_id: Option<&str>,
) -> Result<Availability, AppError> {
    let target = target_user(ctx, user_id).await?;
    current_availability(ctx, &target).await
}

pub(crate) async fn save_availability(
    ctx: &Ctx<'_>,
    user_id: Option<&str>,
    command: SaveAvailabilityCommand,
) -> Result<Availability, AppError> {
    let target = target_user(ctx, user_id).await?;
    let current = current_availability(ctx, &target).await?;
    if let (Some(expected), Some(updated_at)) = (command.expected_updated_at, current.updated_at) {
        ensure_version(Some(expected), updated_at)?;
    }
    let mut updated = apply_availability(current.clone(), &command)?;
    updated.configured = true;
    let changes = diff_changes(
        &if current.configured {
            serde_json::to_value(AvailabilitySnapshot::from(&current))
                .map_err(|error| AppError::Internal(error.to_string()))?
        } else {
            Value::Null
        },
        &AvailabilitySnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    BookingStore::of(ctx)
        .save_availability(&mut tx, &updated, now)
        .await?;
    publish_availability(ctx, &mut tx, &actor, &target, changes, command.ip, now).await?;
    tx.commit().await?;
    current_availability(ctx, &target).await
}

async fn publish_availability(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    actor: &Actor,
    target: &UserId,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let record = user_record(target)?;
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), AVAILABILITY_UPDATED, now)
                .with_record(record.clone())
                .with_payload(serde_json::json!({ "user_id": target.as_str() })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor.clone(), AVAILABILITY_UPDATED, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) async fn contact_fields(ctx: &Ctx<'_>) -> Result<Vec<ContactFieldOption>, AppError> {
    ctx.scope.require(&Permission::BOOKING_MANAGE)?;
    crm::contact_fields(ctx).await
}

pub(crate) async fn distribution_rules(ctx: &Ctx<'_>) -> Result<Vec<(Uuid, String)>, AppError> {
    if ctx.scope.require(&Permission::BOOKING_MANAGE)? < RecordScope::Team {
        return Err(team_links_denied());
    }
    Ok(distribution::active_rules(ctx)
        .await?
        .into_iter()
        .map(|rule| (rule.id, rule.name))
        .collect())
}

pub(crate) async fn list_meeting_types(
    ctx: &Ctx<'_>,
    window: ListWindow,
) -> Result<Listing<MeetingType>, AppError> {
    let owners = ctx.scope.record_filter(&Permission::BOOKING_MANAGE)?;
    BookingStore::of(ctx)
        .list_meeting_types(&MeetingTypeFilter { owners, window })
        .await
}

async fn visible_meeting_type(ctx: &Ctx<'_>, id: &str) -> Result<MeetingType, AppError> {
    let filter = ctx.scope.record_filter(&Permission::BOOKING_MANAGE)?;
    let id = parse_id(id).ok_or_else(meeting_type_not_found)?;
    let meeting_type = BookingStore::of(ctx)
        .meeting_type(id)
        .await?
        .ok_or_else(meeting_type_not_found)?;
    if !BookingStore::of(ctx)
        .user_within(&meeting_type.owner_id, &filter)
        .await?
    {
        return Err(meeting_type_not_found());
    }
    Ok(meeting_type)
}

pub(crate) async fn get_meeting_type(ctx: &Ctx<'_>, id: &str) -> Result<MeetingType, AppError> {
    visible_meeting_type(ctx, id).await
}

async fn contact_field_options(
    ctx: &Ctx<'_>,
    command: &SaveMeetingTypeCommand,
) -> Result<Vec<ContactFieldOption>, AppError> {
    let mapped = command.questions.as_ref().is_some_and(|questions| {
        questions
            .iter()
            .any(|question| question.contact_field.is_some())
    });
    if mapped {
        crm::contact_fields(ctx).await
    } else {
        Ok(Vec::new())
    }
}

async fn distribution_rule(
    ctx: &Ctx<'_>,
    raw: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<Option<(Uuid, String)>, AppError> {
    if ctx.scope.require(&Permission::BOOKING_MANAGE)? < RecordScope::Team {
        return Err(team_links_denied());
    }
    let Some(rule_id) = parse_id(raw) else {
        violations.push(FieldViolation::new(
            "distribution_rule_id",
            "distribution rule was not found",
        ));
        return Ok(None);
    };
    match distribution::find_rule(ctx, rule_id).await? {
        Some(rule) if rule.is_active => Ok(Some((rule.id, rule.name))),
        Some(_) => {
            violations.push(FieldViolation::new(
                "distribution_rule_id",
                "distribution rule is inactive",
            ));
            Ok(None)
        }
        None => {
            violations.push(FieldViolation::new(
                "distribution_rule_id",
                "distribution rule was not found",
            ));
            Ok(None)
        }
    }
}

async fn unique_slug(ctx: &Ctx<'_>, base: &str, except: Option<Uuid>) -> Result<String, AppError> {
    let base = if base.len() >= 3 {
        base.to_owned()
    } else {
        "agenda".to_owned()
    };
    if valid_slug(&base) && !BookingStore::of(ctx).slug_taken(&base, except).await? {
        return Ok(base);
    }
    for _ in 0..5 {
        let candidate = format!("{base}-{}", random_suffix()?);
        if !BookingStore::of(ctx).slug_taken(&candidate, except).await? {
            return Ok(candidate);
        }
    }
    Err(slug_taken())
}

pub(crate) async fn create_meeting_type(
    ctx: &Ctx<'_>,
    command: SaveMeetingTypeCommand,
) -> Result<MeetingType, AppError> {
    ctx.scope.require(&Permission::BOOKING_MANAGE)?;
    let mut violations = Vec::new();
    let meeting_name = match &command.name {
        Some(value) => name(value, &mut violations),
        None => {
            violations.push(FieldViolation::new("name", "is required"));
            None
        }
    };
    let requested_slug = command
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| slug(value, &mut violations));
    let fields = contact_field_options(ctx, &command).await?;
    let meeting_questions = questions(
        command.questions.as_deref().unwrap_or_default(),
        &fields,
        &mut violations,
    );
    let rule = match command
        .distribution_rule_id
        .as_ref()
        .and_then(Option::as_deref)
    {
        Some(raw) => distribution_rule(ctx, raw, &mut violations).await?,
        None => None,
    };
    let now = ctx.now();
    let location = command.location_kind.unwrap_or(LocationKind::None);
    let mut meeting_type = MeetingType {
        id: Uuid::now_v7(),
        owner_id: ctx.scope.user_id().clone(),
        owner_name: String::new(),
        slug: String::new(),
        name: meeting_name.clone().unwrap_or_default(),
        description: description(
            command.description.as_ref().and_then(Option::as_deref),
            &mut violations,
        ),
        duration_minutes: duration(command.duration_minutes.unwrap_or(30), &mut violations),
        slot_interval_minutes: slot_interval(
            command
                .slot_interval_minutes
                .unwrap_or(DEFAULT_SLOT_INTERVAL_MINUTES),
            &mut violations,
        ),
        location_kind: location,
        location_value: location_value(
            command.location_value.as_ref().and_then(Option::as_deref),
            &mut violations,
        ),
        questions: meeting_questions,
        spam_protection: command.spam_protection.unwrap_or(true),
        distribution_rule_id: rule.as_ref().map(|(id, _)| *id),
        distribution_rule_name: rule.map(|(_, name)| name),
        is_active: command.is_active.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    meeting_type.slug = match requested_slug {
        Some(requested) => {
            if BookingStore::of(ctx).slug_taken(&requested, None).await? {
                return Err(slug_taken());
            }
            requested
        }
        None => {
            unique_slug(
                ctx,
                &slugify(meeting_name.as_deref().unwrap_or_default()),
                None,
            )
            .await?
        }
    };
    let record = RecordRef::new(MEETING_TYPE_RECORD, meeting_type.id);
    let changes = diff_changes(
        &Value::Null,
        &MeetingTypeSnapshot::from(&meeting_type),
        &DiffOptions::default(),
    )?;
    let availability = match BookingStore::of(ctx)
        .availability(ctx.scope.user_id())
        .await?
    {
        Some(_) => None,
        None => Some(default_availability(ctx, ctx.scope.user_id()).await?),
    };
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    if let Some(mut availability) = availability {
        availability.configured = true;
        BookingStore::of(ctx)
            .save_availability(&mut tx, &availability, now)
            .await?;
        let availability_changes = diff_changes(
            &Value::Null,
            &AvailabilitySnapshot::from(&availability),
            &DiffOptions::default(),
        )?;
        publish_availability(
            ctx,
            &mut tx,
            &actor,
            ctx.scope.user_id(),
            availability_changes,
            command.ip.clone(),
            now,
        )
        .await?;
    }
    BookingStore::of(ctx)
        .insert_meeting_type(&mut tx, &meeting_type)
        .await?;
    publish(
        ctx,
        &mut tx,
        &actor,
        MEETING_TYPE_CREATED,
        &record,
        &meeting_type,
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    reload(ctx, meeting_type.id).await
}

#[allow(clippy::too_many_arguments)]
async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    actor: &Actor,
    event_type: &'static str,
    record: &RecordRef,
    meeting_type: &MeetingType,
    changes: Vec<FieldChange>,
    ip: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(serde_json::json!({
                    "slug": meeting_type.slug,
                    "name": meeting_type.name,
                    "owner_id": meeting_type.owner_id.as_str(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<MeetingType, AppError> {
    BookingStore::of(ctx)
        .meeting_type(id)
        .await?
        .ok_or_else(meeting_type_not_found)
}

pub(crate) async fn update_meeting_type(
    ctx: &Ctx<'_>,
    id: &str,
    command: SaveMeetingTypeCommand,
) -> Result<MeetingType, AppError> {
    let current = visible_meeting_type(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut updated = current.clone();
    let mut violations = Vec::new();
    if let Some(value) = &command.name
        && let Some(value) = name(value, &mut violations)
    {
        updated.name = value;
    }
    if let Some(value) = command.slug.as_deref()
        && let Some(value) = slug(value, &mut violations)
    {
        updated.slug = value;
    }
    if let Some(value) = &command.description {
        updated.description = description(value.as_deref(), &mut violations);
    }
    if let Some(value) = command.duration_minutes {
        updated.duration_minutes = duration(value, &mut violations);
    }
    if let Some(value) = command.slot_interval_minutes {
        updated.slot_interval_minutes = slot_interval(value, &mut violations);
    }
    if let Some(value) = command.location_kind {
        updated.location_kind = value;
    }
    if let Some(value) = &command.location_value {
        updated.location_value = location_value(value.as_deref(), &mut violations);
    }
    if let Some(inputs) = &command.questions {
        let fields = contact_field_options(ctx, &command).await?;
        updated.questions = questions(inputs, &fields, &mut violations);
    }
    if let Some(value) = command.spam_protection {
        updated.spam_protection = value;
    }
    if let Some(value) = command.is_active {
        updated.is_active = value;
    }
    if let Some(rule) = &command.distribution_rule_id {
        match rule.as_deref() {
            Some(raw) => {
                if let Some((rule_id, rule_name)) =
                    distribution_rule(ctx, raw, &mut violations).await?
                {
                    updated.distribution_rule_id = Some(rule_id);
                    updated.distribution_rule_name = Some(rule_name);
                }
            }
            None => {
                updated.distribution_rule_id = None;
                updated.distribution_rule_name = None;
            }
        }
    }
    if !violations.is_empty() {
        return Err(invalid(violations));
    }
    if updated.slug != current.slug
        && BookingStore::of(ctx)
            .slug_taken(&updated.slug, Some(current.id))
            .await?
    {
        return Err(slug_taken());
    }
    let changes = diff_changes(
        &MeetingTypeSnapshot::from(&current),
        &MeetingTypeSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    let now = ctx.now();
    updated.updated_at = now;
    let record = RecordRef::new(MEETING_TYPE_RECORD, current.id);
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    BookingStore::of(ctx)
        .update_meeting_type(&mut tx, &updated)
        .await?;
    publish(
        ctx,
        &mut tx,
        &actor,
        MEETING_TYPE_UPDATED,
        &record,
        &updated,
        changes,
        command.ip,
        now,
    )
    .await?;
    tx.commit().await?;
    reload(ctx, current.id).await
}

pub(crate) async fn delete_meeting_type(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let current = visible_meeting_type(ctx, id).await?;
    let changes = diff_changes(
        &MeetingTypeSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let now = ctx.now();
    let record = RecordRef::new(MEETING_TYPE_RECORD, current.id);
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    BookingStore::of(ctx)
        .delete_meeting_type(&mut tx, current.id)
        .await?;
    publish(
        ctx,
        &mut tx,
        &actor,
        MEETING_TYPE_DELETED,
        &record,
        &current,
        changes,
        ip,
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn list_bookings(
    ctx: &Ctx<'_>,
    query: BookingListQuery,
    window: ListWindow,
) -> Result<Listing<Booking>, AppError> {
    let hosts = ctx.scope.record_filter(&Permission::BOOKING_MANAGE)?;
    let mut violations = Vec::new();
    let status = query.status;
    let meeting_type_id = match query.meeting_type_id.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => match parse_id(value) {
            Some(id) => Some(id),
            None => {
                violations.push(FieldViolation::new("meeting_type_id", "must be a UUID"));
                None
            }
        },
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid filter", violations));
    }
    BookingStore::of(ctx)
        .list_bookings(&BookingFilter {
            hosts,
            status,
            meeting_type_id,
            after: query.upcoming.then(|| ctx.now()),
            window,
        })
        .await
}

pub(crate) async fn cancel_booking(
    ctx: &Ctx<'_>,
    id: &str,
    reason: Option<String>,
    ip: Option<String>,
) -> Result<Booking, AppError> {
    let filter = ctx.scope.record_filter(&Permission::BOOKING_MANAGE)?;
    let id = parse_id(id).ok_or_else(booking_not_found)?;
    let booking = BookingStore::of(ctx)
        .booking(id)
        .await?
        .ok_or_else(booking_not_found)?;
    if !BookingStore::of(ctx)
        .user_within(&booking.host_user_id, &filter)
        .await?
    {
        return Err(booking_not_found());
    }
    booking::cancel(ctx, booking, Actor::from_scope(ctx.scope), reason, ip).await
}

fn team_links_denied() -> AppError {
    AppError::forbidden_code(
        "permission_denied",
        "team booking links need booking.manage for the team or the organization",
    )
}

fn denied_user() -> AppError {
    AppError::forbidden_code(
        "permission_denied",
        "you cannot manage the booking settings of this user",
    )
}
