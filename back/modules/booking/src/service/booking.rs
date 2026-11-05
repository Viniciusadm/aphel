use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_sdk::calendar::BusyInterval;
use aphel_sdk::calendar::CalendarAvailability;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::service::crm;
use crate::service::distribution::{self, HostPick, WeightedHost};
use crate::slots::{HostCalendar, SlotRules, free_slots, merge_slots};
use crate::store::BookingStore;
use crate::token::{ManageToken, SignedManageToken, decode_key, generate_key};
use crate::validation::{parse_instant, parse_timezone};
use crate::{
    ACTIVITY_TYPE_KEY, Availability, BOOKING_EMAIL_JOB, BOOKING_NOT_CHANGEABLE, BOOKING_NOT_FOUND,
    BOOKING_RECORD, BOOKING_REJECTED, BOOKING_SLOT_UNAVAILABLE, BookCommand, Booking,
    BookingStatus, MAX_ANSWER_CHARS, MAX_CANCEL_REASON_CHARS, MAX_GUEST_NAME_CHARS,
    MAX_WINDOW_DAYS, MEETING_BOOKED, MEETING_CANCELED, MEETING_RESCHEDULED, MEETING_TYPE_NOT_FOUND,
    MeetingEventPayload, MeetingType, PublicBooking, PublicMeetingType, Slot, SlotWindow,
    TOKEN_KEY_NAME, TOKEN_KEY_OWNER, email_job_payload,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::{Secret, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::PendingCommit;
use aphel_sdk::email::normalize_address;
use aphel_sdk::people::PersonCreation;
use aphel_sdk::platform::application::secrets::{SecretKey, SecretVault};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, Job, RecordRef, RecordType,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, PermissionSet, RoleKey};
use aphel_sdk::platform::domain::organization_profile::ORGANIZATION_RECORD_ID;
use aphel_sdk::tasks::{MeetingActivity, MeetingActivityMove};

const BUSY_MARGIN_DAYS: i64 = 1;

pub(crate) fn meeting_type_not_found() -> AppError {
    AppError::not_found_code(MEETING_TYPE_NOT_FOUND, "meeting type was not found")
}

pub(crate) fn booking_not_found() -> AppError {
    AppError::not_found_code(BOOKING_NOT_FOUND, "booking was not found")
}

fn slot_unavailable() -> AppError {
    AppError::conflict_code(
        BOOKING_SLOT_UNAVAILABLE,
        "this time is no longer available; pick another one",
    )
}

fn not_changeable() -> AppError {
    AppError::conflict_code(
        BOOKING_NOT_CHANGEABLE,
        "this booking was canceled or already happened",
    )
}

pub(crate) fn booking_scope(host: &AccessScope) -> AccessScope {
    AccessScope::new(
        host.user_id().clone(),
        RoleKey::system(),
        host.team_ids().to_vec(),
        PermissionSet::everything(),
    )
    .with_enabled_modules(host.enabled_modules().to_vec())
}

fn instant_text(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn validate_window(window: &SlotWindow) -> Result<(), AppError> {
    if window.to <= window.from || window.to - window.from > Duration::days(MAX_WINDOW_DAYS) {
        return Err(AppError::invalid_fields(
            "invalid window",
            vec![FieldViolation::new(
                "to",
                format!("must be after from and at most {MAX_WINDOW_DAYS} days later"),
            )],
        ));
    }
    Ok(())
}

pub(crate) type AwayWindow = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

pub(crate) struct Host {
    pub(crate) user_id: UserId,
    pub(crate) name: String,
    pub(crate) scope: AccessScope,
    pub(crate) availability: Availability,
    pub(crate) away: Vec<AwayWindow>,
    pub(crate) weight: u32,
}

impl Host {
    fn away_at(&self, at: DateTime<Utc>) -> bool {
        self.away.iter().any(|(from, until)| {
            from.is_none_or(|from| from <= at) && until.is_none_or(|until| at < until)
        })
    }
}

struct HostBusy {
    busy: Vec<BusyInterval>,
    booked: Vec<BusyInterval>,
}

fn key_reference() -> SecretKey {
    SecretKey::new(TOKEN_KEY_OWNER, ORGANIZATION_RECORD_ID, TOKEN_KEY_NAME)
}

pub(crate) async fn existing_token_key(ctx: &Ctx<'_>) -> Result<Option<Vec<u8>>, AppError> {
    Ok(ctx
        .secrets()?
        .reveal(ctx.scope, &key_reference())
        .await?
        .and_then(|secret| decode_key(secret.expose())))
}

pub(crate) async fn token_key(ctx: &Ctx<'_>) -> Result<Vec<u8>, AppError> {
    if let Some(key) = existing_token_key(ctx).await? {
        return Ok(key);
    }
    ctx.secrets()?
        .store_organization_if_absent(ctx.scope, &key_reference(), &Secret::new(generate_key()?))
        .await?;
    existing_token_key(ctx)
        .await?
        .ok_or_else(|| AppError::Internal("booking token key was not stored".to_owned()))
}

async fn resolve(ctx: &Ctx<'_>, slug: &str) -> Result<MeetingType, AppError> {
    let slug = slug.trim().to_lowercase();
    if slug.is_empty() || slug.len() > 80 {
        return Err(meeting_type_not_found());
    }
    let meeting_type = BookingStore::of(ctx)
        .meeting_type_by_slug(&slug)
        .await?
        .filter(|meeting_type| meeting_type.is_active)
        .ok_or_else(meeting_type_not_found)?;
    Ok(meeting_type)
}

async fn host_availability(ctx: &Ctx<'_>, user_id: &UserId) -> Result<Availability, AppError> {
    if let Some(availability) = BookingStore::of(ctx).availability(user_id).await? {
        return Ok(availability);
    }
    let organization = BookingStore::of(ctx).organization().await?;
    let timezone = BookingStore::of(ctx)
        .host(user_id)
        .await?
        .and_then(|host| host.timezone)
        .and_then(|timezone| parse_timezone(timezone.as_str()))
        .unwrap_or(organization.timezone);
    Ok(Availability::defaults(user_id.clone(), timezone))
}

async fn load_host(ctx: &Ctx<'_>, user_id: &UserId) -> Result<Option<Host>, AppError> {
    let Some(host_scope) = crm::host_scope(ctx, user_id)
        .await?
        .filter(|host_scope| host_scope.scope_for(&Permission::BOOKING_MANAGE).is_some())
    else {
        return Ok(None);
    };
    let name = BookingStore::of(ctx)
        .host(user_id)
        .await?
        .map(|profile| profile.name)
        .unwrap_or_default();
    Ok(Some(Host {
        user_id: user_id.clone(),
        name,
        scope: host_scope,
        availability: host_availability(ctx, user_id).await?,
        away: Vec::new(),
        weight: 1,
    }))
}

pub(crate) async fn meeting_hosts(
    ctx: &Ctx<'_>,
    meeting_type: &MeetingType,
) -> Result<Vec<Host>, AppError> {
    let Some(rule_id) = meeting_type.distribution_rule_id else {
        return Ok(load_host(ctx, &meeting_type.owner_id)
            .await?
            .into_iter()
            .collect());
    };
    let Some(rule) = distribution::find_rule(ctx, rule_id)
        .await?
        .filter(|rule| rule.is_active)
    else {
        return Ok(Vec::new());
    };
    let pool = distribution::rule_pool(ctx, &rule, ctx.now()).await?;
    let Some(rule) = pool.rule else {
        return Ok(Vec::new());
    };
    let mut hosts = Vec::new();
    for member in pool.members {
        if let Some(mut host) = load_host(ctx, &member.user_id).await? {
            host.weight = member.weight;
            host.away = rule
                .members
                .iter()
                .filter(|candidate| candidate.user_id == member.user_id && candidate.away)
                .map(|candidate| (candidate.away_from, candidate.away_until))
                .collect();
            hosts.push(host);
        }
    }
    Ok(hosts)
}

async fn host_busy(
    ctx: &Ctx<'_>,
    host: &UserId,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    exclude: Option<&Booking>,
) -> Result<Option<HostBusy>, AppError> {
    let from = from - Duration::days(BUSY_MARGIN_DAYS);
    let to = to + Duration::days(BUSY_MARGIN_DAYS);
    let repository = BookingStore::of(ctx);
    let mut busy = aphel_sdk::tasks::meetings()?
        .busy(
            ctx,
            host,
            from,
            to,
            exclude.and_then(|booking| booking.activity_id),
        )
        .await?;
    for calendar in ctx.app().contributions::<dyn CalendarAvailability>() {
        match calendar.busy(ctx, host, from, to).await {
            Ok(Some(intervals)) => busy.extend(intervals),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    calendar = calendar.key(),
                    %error,
                    "external calendar unavailable; hiding the host's slots"
                );
                return Ok(None);
            }
        }
    }
    let booked = repository
        .booked(host, from, to, exclude.map(|booking| booking.id))
        .await?;
    Ok(Some(HostBusy { busy, booked }))
}

async fn host_slots(
    ctx: &Ctx<'_>,
    host: &Host,
    meeting_type: &MeetingType,
    window: &SlotWindow,
    exclude: Option<&Booking>,
) -> Result<Vec<Slot>, AppError> {
    let Some(calendar) = host_busy(ctx, &host.user_id, window.from, window.to, exclude).await?
    else {
        return Ok(Vec::new());
    };
    let slots = free_slots(
        &HostCalendar {
            availability: &host.availability,
            busy: &calendar.busy,
            booked: &calendar.booked,
        },
        rules(meeting_type),
        window.from,
        window.to,
        ctx.now(),
    );
    Ok(slots
        .into_iter()
        .filter(|slot| !host.away_at(slot.starts_at))
        .collect())
}

pub(crate) async fn public_meeting_type(
    ctx: &Ctx<'_>,
    slug: &str,
) -> Result<PublicMeetingType, AppError> {
    let meeting_type = resolve(ctx, slug).await?;
    let organization = BookingStore::of(ctx).organization().await?;
    let (host_name, timezone) = if meeting_type.is_team() {
        (None, organization.timezone)
    } else {
        let availability = host_availability(ctx, &meeting_type.owner_id).await?;
        (Some(meeting_type.owner_name.clone()), availability.timezone)
    };
    Ok(PublicMeetingType {
        meeting_type,
        host_name,
        organization_name: organization.name,
        timezone,
    })
}

pub(crate) async fn public_slots(
    ctx: &Ctx<'_>,
    slug: &str,
    window: SlotWindow,
) -> Result<Vec<Slot>, AppError> {
    validate_window(&window)?;
    let meeting_type = resolve(ctx, slug).await?;
    let mut groups = Vec::new();
    for host in meeting_hosts(ctx, &meeting_type).await? {
        groups.push(host_slots(ctx, &host, &meeting_type, &window, None).await?);
    }
    Ok(merge_slots(groups))
}

async fn free_at(
    ctx: &Ctx<'_>,
    host: &Host,
    meeting_type: &MeetingType,
    starts_at: DateTime<Utc>,
    exclude: Option<&Booking>,
) -> Result<bool, AppError> {
    let window = SlotWindow {
        from: starts_at,
        to: starts_at + Duration::minutes(1),
    };
    Ok(host_slots(ctx, host, meeting_type, &window, exclude)
        .await?
        .iter()
        .any(|slot| slot.starts_at == starts_at))
}

fn parse_guest(command: &BookCommand, meeting_type: &MeetingType) -> Result<Guest, AppError> {
    let mut violations = Vec::new();
    if meeting_type.spam_protection
        && command
            .honeypot
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(AppError::validation_code(
            BOOKING_REJECTED,
            "the booking could not be accepted",
        ));
    }
    let starts_at = parse_instant(&command.starts_at);
    if starts_at.is_none() {
        violations.push(FieldViolation::new(
            "starts_at",
            "must be an RFC 3339 instant",
        ));
    }
    let name = command.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_GUEST_NAME_CHARS {
        violations.push(FieldViolation::new(
            "name",
            format!("must have 1 to {MAX_GUEST_NAME_CHARS} characters"),
        ));
    }
    let email = match normalize_address(&command.email) {
        Ok(email) => Some(email),
        Err(message) => {
            violations.push(FieldViolation::new("email", message));
            None
        }
    };
    let timezone = command
        .timezone
        .as_deref()
        .and_then(parse_timezone)
        .map(|timezone| timezone.name().to_owned());
    let locale = command
        .locale
        .as_deref()
        .map(str::trim)
        .filter(|locale| !locale.is_empty() && locale.len() <= 10)
        .map(str::to_owned);
    let mut answers = Map::new();
    for question in &meeting_type.questions {
        let value = command
            .answers
            .get(&question.key)
            .map(|value| match value {
                Value::String(text) => text.trim().to_owned(),
                Value::Null => String::new(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let field = format!("answers.{}", question.key);
        if value.is_empty() {
            if question.required {
                violations.push(FieldViolation::new(field, "is required"));
            }
            continue;
        }
        if value.chars().count() > MAX_ANSWER_CHARS {
            violations.push(FieldViolation::new(
                field,
                format!("must have at most {MAX_ANSWER_CHARS} characters"),
            ));
            continue;
        }
        answers.insert(question.key.clone(), Value::String(value));
    }
    match (starts_at, email, violations.is_empty()) {
        (Some(starts_at), Some(email), true) => Ok(Guest {
            starts_at,
            name,
            email,
            timezone,
            locale,
            answers,
        }),
        _ => Err(AppError::invalid_fields("invalid booking", violations)),
    }
}

async fn pick_host<'a>(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    meeting_type: &MeetingType,
    candidates: &'a [&'a Host],
) -> Result<&'a Host, AppError> {
    let Some(rule_id) = meeting_type.distribution_rule_id else {
        return candidates.first().copied().ok_or_else(slot_unavailable);
    };
    let weighted: Vec<WeightedHost> = candidates
        .iter()
        .map(|host| WeightedHost {
            user_id: host.user_id.clone(),
            weight: host.weight,
        })
        .collect();
    let chosen = match distribution::pick_user(ctx, tx, rule_id, &weighted).await? {
        HostPick::Untracked => {
            return candidates.first().copied().ok_or_else(slot_unavailable);
        }
        HostPick::Unavailable => return Err(slot_unavailable()),
        HostPick::Picked(chosen) => chosen,
    };
    candidates
        .iter()
        .copied()
        .find(|host| host.user_id == chosen)
        .ok_or_else(slot_unavailable)
}

async fn ensure_host_row(ctx: &Ctx<'_>, host: &Host) -> Result<(), AppError> {
    if host.availability.configured {
        return Ok(());
    }
    BookingStore::of(ctx)
        .insert_availability_if_absent(&host.availability, ctx.now())
        .await
}

async fn lock_host(ctx: &Ctx<'_>, tx: &mut MySqlTx, host: &Host) -> Result<(), AppError> {
    if BookingStore::of(ctx)
        .lock_availability(tx, &host.user_id)
        .await?
    {
        Ok(())
    } else {
        Err(slot_unavailable())
    }
}

async fn still_free(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    host: &Host,
    meeting_type: &MeetingType,
    starts_at: DateTime<Utc>,
    exclude: Option<Uuid>,
) -> Result<bool, AppError> {
    let booked = BookingStore::of(ctx)
        .booked_locked(
            tx,
            &host.user_id,
            starts_at - Duration::days(BUSY_MARGIN_DAYS),
            starts_at + Duration::days(BUSY_MARGIN_DAYS),
            exclude,
        )
        .await?;
    let window_end = starts_at + Duration::minutes(1);
    Ok(free_slots(
        &HostCalendar {
            availability: &host.availability,
            busy: &[],
            booked: &booked,
        },
        rules(meeting_type),
        starts_at,
        window_end,
        ctx.now(),
    )
    .iter()
    .any(|slot| slot.starts_at == starts_at))
}

async fn enqueue_emails(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    booking: &Booking,
    kind: &str,
) -> Result<(), AppError> {
    for recipient in ["guest", "host"] {
        ctx.jobs()
            .enqueue(
                tx,
                Job::new(
                    BOOKING_EMAIL_JOB,
                    email_job_payload(booking.id, kind, recipient),
                )
                .with_unique_key(format!(
                    "booking:{}:{}:{kind}:{recipient}",
                    booking.id, booking.revision
                )),
            )
            .await?;
    }
    Ok(())
}

async fn publish_meeting(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    origin: Origin,
    event_type: &'static str,
    booking: &Booking,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    let Origin { actor, ip } = origin;
    let (Some(contact_id), Some(activity_id)) = (booking.contact_id, booking.activity_id) else {
        return Err(AppError::Internal(
            "a confirmed booking must have a contact and an activity".to_owned(),
        ));
    };
    let now = ctx.now();
    let payload = MeetingEventPayload {
        booking_id: booking.id,
        meeting_type_id: booking.meeting_type_id,
        activity_id,
        contact_id,
        host_user_id: booking.host_user_id.clone(),
        starts_at: booking.starts_at,
        ends_at: booking.ends_at,
    };
    let event = DomainEvent::new(actor.clone(), event_type, now)
        .with_record(RecordRef::new(RecordType::CONTACT, contact_id))
        .with_payload(payload.to_value());
    let entry = AuditEntry::new(actor, event_type, now)
        .with_record(RecordRef::new(BOOKING_RECORD, booking.id))
        .with_changes(changes)
        .with_ip(ip);
    ctx.events().publish(tx, event).await?;
    ctx.audit().record(tx, entry).await?;
    Ok(())
}

pub(crate) async fn book(
    ctx: &Ctx<'_>,
    slug: &str,
    command: BookCommand,
) -> Result<PublicBooking, AppError> {
    let meeting_type = resolve(ctx, slug).await?;
    let guest = parse_guest(&command, &meeting_type)?;
    let hosts = meeting_hosts(ctx, &meeting_type).await?;
    let mut free = Vec::new();
    for host in &hosts {
        if free_at(ctx, host, &meeting_type, guest.starts_at, None).await? {
            free.push(host);
        }
    }
    if free.is_empty() {
        return Err(slot_unavailable());
    }
    let manage_key = token_key(ctx).await?;
    for host in &free {
        ensure_host_row(ctx, host).await?;
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let host = pick_host(ctx, &mut tx, &meeting_type, &free).await?;
    lock_host(ctx, &mut tx, host).await?;
    if !still_free(ctx, &mut tx, host, &meeting_type, guest.starts_at, None).await? {
        return Err(slot_unavailable());
    }
    let mut booking = Booking {
        id: Uuid::now_v7(),
        meeting_type_id: meeting_type.id,
        meeting_type_name: meeting_type.name.clone(),
        host_user_id: host.user_id.clone(),
        host_name: host.name.clone(),
        contact_id: None,
        activity_id: None,
        status: BookingStatus::Confirmed,
        starts_at: guest.starts_at,
        ends_at: guest.starts_at + Duration::minutes(i64::from(meeting_type.duration_minutes)),
        guest_name: guest.name.clone(),
        guest_email: guest.email.clone(),
        guest_timezone: guest.timezone.clone(),
        guest_locale: guest.locale.clone(),
        answers: guest.answers.clone(),
        revision: 0,
        cancel_reason: None,
        canceled_at: None,
        created_at: now,
        updated_at: now,
    };
    let acting = booking_scope(&host.scope);
    let acting_ctx = ctx.with_scope(&acting);
    let records = create_records(
        &acting_ctx,
        &mut tx,
        host,
        &meeting_type,
        &booking,
        command.ip.clone(),
    )
    .await?;
    booking.contact_id = Some(records.contact_id);
    booking.activity_id = Some(records.activity_id);
    BookingStore::of(ctx)
        .insert_booking(&mut tx, &booking)
        .await?;
    confirm(ctx, &mut tx, &booking, records.contact_id, command.ip).await?;
    tx.commit().await?;
    if let Some(contact) = &records.created_contact {
        contact.committed(&acting_ctx).await;
    }

    let organization = BookingStore::of(ctx).organization().await?;
    Ok(PublicBooking {
        manage_token: ManageToken {
            booking_id: booking.id,
        }
        .encode(&manage_key)?,
        booking,
        meeting_type,
        organization_name: organization.name,
        host_timezone: host.availability.timezone,
    })
}

async fn confirm(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    booking: &Booking,
    contact_id: Uuid,
    ip: Option<String>,
) -> Result<(), AppError> {
    let changes = vec![
        FieldChange::new(
            "status",
            Value::Null,
            json!(BookingStatus::Confirmed.as_str()),
        ),
        FieldChange::new(
            "starts_at",
            Value::Null,
            json!(instant_text(booking.starts_at)),
        ),
        FieldChange::new(
            "host_user_id",
            Value::Null,
            json!(booking.host_user_id.as_str()),
        ),
        FieldChange::new("contact_id", Value::Null, json!(contact_id)),
    ];
    publish_meeting(
        ctx,
        tx,
        Origin {
            actor: Actor::System,
            ip,
        },
        MEETING_BOOKED,
        booking,
        changes,
    )
    .await?;
    enqueue_emails(ctx, tx, booking, "booked").await
}

struct BookingRecords {
    contact_id: Uuid,
    activity_id: Uuid,
    created_contact: Option<PendingCommit>,
}

const CONTACT_SAVEPOINT: &str = "booking_contact";
const ACTIVITY_SAVEPOINT: &str = "booking_activity";

async fn savepoint(tx: &mut MySqlTx, name: &str) -> Result<(), AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!("SAVEPOINT {name}")))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn rollback_to_savepoint(tx: &mut MySqlTx, name: &str) -> Result<(), AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!("ROLLBACK TO SAVEPOINT {name}")))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn create_contact_attempt(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    command: PersonCreation,
) -> Result<PendingCommit, AppError> {
    savepoint(tx, CONTACT_SAVEPOINT).await?;
    let created = aphel_sdk::people::writer()?
        .create_in_tx(ctx, tx, command)
        .await;
    if created.is_err() {
        rollback_to_savepoint(tx, CONTACT_SAVEPOINT).await?;
    }
    created
}

struct UpdatedActivity {
    scope: AccessScope,
    pending: PendingCommit,
}

impl UpdatedActivity {
    async fn committed(self, ctx: &Ctx<'_>) {
        self.pending.committed(&ctx.with_scope(&self.scope)).await;
    }
}

async fn cancel_activity(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    booking: &Booking,
    ip: Option<String>,
) -> Result<Option<UpdatedActivity>, AppError> {
    let Some(activity_id) = booking.activity_id else {
        return Ok(None);
    };
    let Some(host_scope) = crm::host_scope(ctx, &booking.host_user_id).await? else {
        return Ok(None);
    };
    let acting = booking_scope(&host_scope);
    let updated =
        update_activity_attempt(ctx, tx, acting, activity_id, ActivityChange::Cancel { ip })
            .await?;
    match updated {
        Ok(updated) => Ok(updated),
        Err(error) => {
            tracing::warn!(%error, booking_id = %booking.id, "failed to cancel the meeting activity");
            Ok(None)
        }
    }
}

async fn move_activity(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    booking: &Booking,
    host: &Host,
    duration_minutes: u32,
    ip: Option<String>,
) -> Result<Option<UpdatedActivity>, AppError> {
    let Some(activity_id) = booking.activity_id else {
        return Ok(None);
    };
    let updated = update_activity_attempt(
        ctx,
        tx,
        booking_scope(&host.scope),
        activity_id,
        ActivityChange::Move {
            starts_at: booking.starts_at,
            duration_minutes,
            ip,
        },
    )
    .await?;
    match updated {
        Ok(updated) => Ok(updated),
        Err(error) => {
            tracing::warn!(%error, booking_id = %booking.id, "failed to move the meeting activity");
            Ok(None)
        }
    }
}

enum ActivityChange {
    Move {
        starts_at: DateTime<Utc>,
        duration_minutes: u32,
        ip: Option<String>,
    },
    Cancel {
        ip: Option<String>,
    },
}

async fn update_activity_attempt(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    acting: AccessScope,
    activity_id: Uuid,
    change: ActivityChange,
) -> Result<Result<Option<UpdatedActivity>, AppError>, AppError> {
    let record_states = tx.record_states.clone();
    savepoint(tx, ACTIVITY_SAVEPOINT).await?;
    let updated = match change {
        ActivityChange::Move {
            starts_at,
            duration_minutes,
            ip,
        } => {
            crm::move_meeting(
                ctx,
                tx,
                &acting,
                activity_id,
                MeetingActivityMove {
                    starts_at,
                    duration_minutes,
                    ip,
                },
            )
            .await
        }
        ActivityChange::Cancel { ip } => {
            crm::cancel_meeting(ctx, tx, &acting, activity_id, ip).await
        }
    };
    match updated {
        Ok(pending) => Ok(Ok(pending.map(|pending| UpdatedActivity {
            scope: acting,
            pending,
        }))),
        Err(error) => {
            rollback_to_savepoint(tx, ACTIVITY_SAVEPOINT).await?;
            tx.record_states = record_states;
            Ok(Err(error))
        }
    }
}

async fn find_or_create_contact(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    host: &Host,
    meeting_type: &MeetingType,
    booking: &Booking,
    ip: Option<String>,
) -> Result<(Uuid, Option<PendingCommit>), AppError> {
    if let Some(contact_id) = crm::find_contact_by_email(ctx, &booking.guest_email).await? {
        return Ok((contact_id, None));
    }
    let (first_name, last_name) = split_name(&booking.guest_name);
    let mut input = Map::from_iter([
        ("first_name".to_owned(), Value::String(first_name)),
        (
            "email".to_owned(),
            Value::String(booking.guest_email.clone()),
        ),
        (
            "owner_id".to_owned(),
            Value::String(host.user_id.as_str().to_owned()),
        ),
    ]);
    if let Some(last_name) = last_name {
        input.insert("last_name".to_owned(), Value::String(last_name));
    }
    let base = input.clone();
    let mut custom_fields = Map::new();
    for question in &meeting_type.questions {
        let (Some(target), Some(Value::String(answer))) =
            (&question.contact_field, booking.answers.get(&question.key))
        else {
            continue;
        };
        let value = Value::String(answer.clone());
        match target.as_str() {
            "phone" | "mobile" | "job_title" | "notes" | "address_city" | "address_state"
            | "address_country" => {
                input.insert(target.clone(), value);
            }
            custom => {
                custom_fields.insert(custom.to_owned(), Value::String(answer.clone()));
            }
        }
    }
    let mapped = PersonCreation {
        values: input,
        custom_fields,
        ip: ip.clone(),
    };
    let created = match create_contact_attempt(ctx, tx, mapped).await {
        Ok(created) => created,
        Err(AppError::InvalidFields { .. } | AppError::CodedFields { .. }) => {
            aphel_sdk::people::writer()?
                .create_in_tx(
                    ctx,
                    tx,
                    PersonCreation {
                        values: base,
                        custom_fields: Map::new(),
                        ip,
                    },
                )
                .await?
        }
        Err(error) => return Err(error),
    };
    Ok((created.record.record_id, Some(created)))
}

async fn create_records(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    host: &Host,
    meeting_type: &MeetingType,
    booking: &Booking,
    ip: Option<String>,
) -> Result<BookingRecords, AppError> {
    let (contact_id, created_contact) =
        find_or_create_contact(ctx, tx, host, meeting_type, booking, ip.clone()).await?;
    let activity_type_id = crm::meeting_activity_type(ctx, ctx.scope)
        .await?
        .ok_or_else(|| {
            AppError::Internal(format!(
                "the organization has no active `{ACTIVITY_TYPE_KEY}` activity type"
            ))
        })?;
    let activity_id = crm::schedule_meeting(
        ctx,
        tx,
        MeetingActivity {
            activity_type_id,
            title: activity_title(meeting_type, booking),
            owner_id: host.user_id.clone(),
            record: RecordRef::new(RecordType::CONTACT, contact_id),
            starts_at: booking.starts_at,
            duration_minutes: meeting_type.duration_minutes,
            description: Some(activity_description(meeting_type, booking)),
            ip,
        },
    )
    .await?;
    Ok(BookingRecords {
        contact_id,
        activity_id,
        created_contact,
    })
}

async fn resolve_token(ctx: &Ctx<'_>, token: &str) -> Result<(Booking, Vec<u8>), AppError> {
    let signed = SignedManageToken::parse(token).ok_or_else(booking_not_found)?;
    let key = existing_token_key(ctx)
        .await?
        .ok_or_else(booking_not_found)?;
    if !signed.verify(&key) {
        return Err(booking_not_found());
    }
    let booking = BookingStore::of(ctx)
        .booking(signed.token.booking_id)
        .await?
        .ok_or_else(booking_not_found)?;
    Ok((booking, key))
}

async fn public_view(
    ctx: &Ctx<'_>,
    booking: Booking,
    key: &[u8],
) -> Result<PublicBooking, AppError> {
    let meeting_type = BookingStore::of(ctx)
        .meeting_type(booking.meeting_type_id)
        .await?
        .ok_or_else(booking_not_found)?;
    let organization = BookingStore::of(ctx).organization().await?;
    let availability = host_availability(ctx, &booking.host_user_id).await?;
    Ok(PublicBooking {
        manage_token: ManageToken {
            booking_id: booking.id,
        }
        .encode(key)?,
        booking,
        meeting_type,
        organization_name: organization.name,
        host_timezone: availability.timezone,
    })
}

pub(crate) async fn manage(ctx: &Ctx<'_>, token: &str) -> Result<PublicBooking, AppError> {
    let (booking, key) = resolve_token(ctx, token).await?;
    public_view(ctx, booking, &key).await
}

fn changeable(ctx: &Ctx<'_>, booking: &Booking) -> Result<(), AppError> {
    if booking.status != BookingStatus::Confirmed || booking.starts_at <= ctx.now() {
        return Err(not_changeable());
    }
    Ok(())
}

async fn booking_host(ctx: &Ctx<'_>, booking: &Booking) -> Result<(Host, MeetingType), AppError> {
    let meeting_type = BookingStore::of(ctx)
        .meeting_type(booking.meeting_type_id)
        .await?
        .filter(|meeting_type| meeting_type.is_active)
        .ok_or_else(not_changeable)?;
    let host = load_host(ctx, &booking.host_user_id)
        .await?
        .ok_or_else(not_changeable)?;
    Ok((host, meeting_type))
}

pub(crate) async fn manage_slots(
    ctx: &Ctx<'_>,
    token: &str,
    window: SlotWindow,
) -> Result<Vec<Slot>, AppError> {
    validate_window(&window)?;
    let (booking, _) = resolve_token(ctx, token).await?;
    changeable(ctx, &booking)?;
    let (host, meeting_type) = booking_host(ctx, &booking).await?;
    host_slots(ctx, &host, &meeting_type, &window, Some(&booking)).await
}

pub(crate) async fn reschedule(
    ctx: &Ctx<'_>,
    token: &str,
    starts_at: &str,
    ip: Option<String>,
) -> Result<PublicBooking, AppError> {
    let (booking, key) = resolve_token(ctx, token).await?;
    changeable(ctx, &booking)?;
    let starts_at = parse_instant(starts_at).ok_or_else(|| {
        AppError::invalid_fields(
            "invalid booking",
            vec![FieldViolation::new(
                "starts_at",
                "must be an RFC 3339 instant",
            )],
        )
    })?;
    let (host, meeting_type) = booking_host(ctx, &booking).await?;
    if !free_at(ctx, &host, &meeting_type, starts_at, Some(&booking)).await? {
        return Err(slot_unavailable());
    }
    let mut moved = booking.clone();
    moved.starts_at = starts_at;
    moved.ends_at = starts_at + Duration::minutes(i64::from(meeting_type.duration_minutes));
    moved.revision += 1;
    moved.updated_at = ctx.now();
    ensure_host_row(ctx, &host).await?;
    let mut tx = ctx.begin().await?;
    lock_host(ctx, &mut tx, &host).await?;
    if !still_free(
        ctx,
        &mut tx,
        &host,
        &meeting_type,
        starts_at,
        Some(booking.id),
    )
    .await?
    {
        return Err(slot_unavailable());
    }
    BookingStore::of(ctx)
        .move_booking(&mut tx, &moved, moved.updated_at)
        .await?;
    let changes = vec![
        FieldChange::new(
            "starts_at",
            json!(instant_text(booking.starts_at)),
            json!(instant_text(moved.starts_at)),
        ),
        FieldChange::new(
            "ends_at",
            json!(instant_text(booking.ends_at)),
            json!(instant_text(moved.ends_at)),
        ),
    ];
    publish_meeting(
        ctx,
        &mut tx,
        Origin {
            actor: Actor::System,
            ip: ip.clone(),
        },
        MEETING_RESCHEDULED,
        &moved,
        changes,
    )
    .await?;
    enqueue_emails(ctx, &mut tx, &moved, "rescheduled").await?;
    let activity = move_activity(
        ctx,
        &mut tx,
        &moved,
        &host,
        meeting_type.duration_minutes,
        ip,
    )
    .await?;
    tx.commit().await?;
    if let Some(activity) = activity {
        activity.committed(ctx).await;
    }
    public_view(ctx, moved, &key).await
}

pub(crate) async fn cancel_with_token(
    ctx: &Ctx<'_>,
    token: &str,
    reason: Option<String>,
    ip: Option<String>,
) -> Result<PublicBooking, AppError> {
    let (booking, key) = resolve_token(ctx, token).await?;
    let canceled = cancel(ctx, booking, Actor::System, reason, ip).await?;
    public_view(ctx, canceled, &key).await
}

pub(crate) async fn cancel(
    ctx: &Ctx<'_>,
    booking: Booking,
    actor: Actor,
    reason: Option<String>,
    ip: Option<String>,
) -> Result<Booking, AppError> {
    changeable(ctx, &booking)?;
    let reason = reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(|reason| {
            reason
                .chars()
                .take(MAX_CANCEL_REASON_CHARS)
                .collect::<String>()
        });
    let mut canceled = booking.clone();
    canceled.status = BookingStatus::Canceled;
    canceled.cancel_reason = reason;
    canceled.canceled_at = Some(ctx.now());
    canceled.revision += 1;
    canceled.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    BookingStore::of(ctx)
        .cancel_booking(&mut tx, &canceled, canceled.updated_at)
        .await?;
    let changes = vec![FieldChange::new(
        "status",
        json!(booking.status.as_str()),
        json!(BookingStatus::Canceled.as_str()),
    )];
    publish_meeting(
        ctx,
        &mut tx,
        Origin {
            actor,
            ip: ip.clone(),
        },
        MEETING_CANCELED,
        &canceled,
        changes,
    )
    .await?;
    enqueue_emails(ctx, &mut tx, &canceled, "canceled").await?;
    let activity = cancel_activity(ctx, &mut tx, &canceled, ip).await?;
    tx.commit().await?;
    if let Some(activity) = activity {
        activity.committed(ctx).await;
    }
    Ok(canceled)
}

pub(crate) struct Origin {
    pub(crate) actor: Actor,
    pub(crate) ip: Option<String>,
}

struct Guest {
    starts_at: DateTime<Utc>,
    name: String,
    email: String,
    timezone: Option<String>,
    locale: Option<String>,
    answers: Map<String, Value>,
}

fn rules(meeting_type: &MeetingType) -> SlotRules {
    SlotRules {
        duration_minutes: meeting_type.duration_minutes,
        interval_minutes: meeting_type.slot_interval_minutes,
    }
}

fn split_name(name: &str) -> (String, Option<String>) {
    let name = name.trim();
    match name.split_once(char::is_whitespace) {
        Some((first, rest)) if !rest.trim().is_empty() => {
            (first.to_owned(), Some(rest.trim().to_owned()))
        }
        _ => (name.to_owned(), None),
    }
}

fn activity_title(meeting_type: &MeetingType, booking: &Booking) -> String {
    let title = format!("{} — {}", meeting_type.name, booking.guest_name);
    title.chars().take(200).collect()
}

pub(crate) fn activity_description(meeting_type: &MeetingType, booking: &Booking) -> String {
    let mut lines = vec![format!(
        "Agendado pelo link de agendamento por {} <{}>.",
        booking.guest_name, booking.guest_email
    )];
    if let Some(location) = &meeting_type.location_value {
        lines.push(format!("Local: {location}"));
    }
    for question in &meeting_type.questions {
        if let Some(Value::String(answer)) = booking.answers.get(&question.key) {
            lines.push(format!("{}: {answer}", question.label));
        }
    }
    let description = lines.join("\n");
    description.chars().take(5000).collect()
}
