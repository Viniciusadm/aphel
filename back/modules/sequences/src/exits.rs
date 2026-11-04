use aphel_contract_crm::CrmRecordTypes;
use std::collections::BTreeSet;

use serde_json::Value;
use uuid::Uuid;

use crate::domain::ExitReason;
use crate::runner::exit_enrollment;
use crate::store::{DirectoryStore, EnrollmentStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::errors::AppError;
use aphel_sdk::booking::MEETING_BOOKED;
use aphel_sdk::deals::{DEAL_LOST, DEAL_WON};
use aphel_sdk::email::{EMAIL_BOUNCED, EMAIL_OPENED, EMAIL_RECEIVED};
use email::tracking::{EMAIL_TRACKING_OBJECTION_ADDED, address_hash};

pub(crate) const SEQUENCE_EXITS_CONSUMER: &str = "sequences.exits";
pub(crate) const CONTACT_DELETED: &str = "contact.deleted";

pub(crate) const EXIT_EVENT_TYPES: [&str; 8] = [
    EMAIL_RECEIVED,
    EMAIL_OPENED,
    EMAIL_BOUNCED,
    MEETING_BOOKED,
    DEAL_WON,
    DEAL_LOST,
    CONTACT_DELETED,
    EMAIL_TRACKING_OBJECTION_ADDED,
];

fn uuid_field(payload: &Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn text_field<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(Value::as_str)
}

fn event_record(event: &DomainEvent, record_type: &RecordType) -> Option<Uuid> {
    event
        .record
        .as_ref()
        .filter(|record| &record.record_type == record_type)
        .map(|record| record.record_id)
}

pub(crate) async fn handle(ctx: &Ctx<'_>, event: &DomainEvent) -> Result<(), AppError> {
    let now = ctx.now();
    let enrollments = EnrollmentStore::of(ctx);
    let payload = &event.payload;
    match event.event_type.as_str() {
        EMAIL_RECEIVED => {
            if text_field(payload, "direction") != Some("inbound")
                || text_field(payload, "kind") != Some("message")
            {
                return Ok(());
            }
            let mut live_enrollments = BTreeSet::new();
            if let Some(thread_id) = uuid_field(payload, "thread_id") {
                live_enrollments.extend(
                    enrollments
                        .mark_replied(thread_id, event.occurred_at)
                        .await?,
                );
            }
            let people: Vec<Uuid> = event_record(event, &RecordType::CONTACT)
                .into_iter()
                .collect();
            live_enrollments.extend(live(&enrollments, &people).await?);
            exit_all(ctx, live_enrollments, ExitReason::Replied, now).await
        }
        EMAIL_OPENED => {
            if let Some(email_id) = uuid_field(payload, "email_id") {
                enrollments.mark_opened(email_id, event.occurred_at).await?;
            }
            Ok(())
        }
        EMAIL_BOUNCED => {
            let mut live_enrollments = BTreeSet::new();
            if let Some(email_id) = uuid_field(payload, "email_id") {
                live_enrollments.extend(
                    enrollments
                        .mark_bounced(email_id, event.occurred_at)
                        .await?,
                );
            }
            let people: Vec<Uuid> = event_record(event, &RecordType::CONTACT)
                .into_iter()
                .collect();
            live_enrollments.extend(live(&enrollments, &people).await?);
            exit_all(ctx, live_enrollments, ExitReason::Bounced, now).await
        }
        MEETING_BOOKED => {
            let people: Vec<Uuid> = uuid_field(payload, "contact_id")
                .or_else(|| event_record(event, &RecordType::CONTACT))
                .into_iter()
                .collect();
            let live_enrollments = live(&enrollments, &people).await?;
            exit_all(ctx, live_enrollments, ExitReason::MeetingBooked, now).await
        }
        DEAL_WON | DEAL_LOST => {
            let Some(deal_id) = event_record(event, &RecordType::DEAL) else {
                return Ok(());
            };
            let people = DirectoryStore::of(ctx).deal_contacts(deal_id).await?;
            let live_enrollments = live(&enrollments, &people).await?;
            let reason = if event.event_type == DEAL_WON {
                ExitReason::DealWon
            } else {
                ExitReason::DealLost
            };
            exit_all(ctx, live_enrollments, reason, now).await
        }
        CONTACT_DELETED => {
            let people: Vec<Uuid> = event_record(event, &RecordType::CONTACT)
                .into_iter()
                .collect();
            let live_enrollments = live(&enrollments, &people).await?;
            exit_all(ctx, live_enrollments, ExitReason::ContactDeleted, now).await
        }
        EMAIL_TRACKING_OBJECTION_ADDED => {
            let Some(objection_id) = uuid_field(payload, "objection_id") else {
                return Ok(());
            };
            let Some(hash) = DirectoryStore::of(ctx).objection_hash(objection_id).await? else {
                return Ok(());
            };
            let people: Vec<Uuid> = enrollments
                .live_contact_emails()
                .await?
                .into_iter()
                .filter(|(_, email)| address_hash(email) == hash)
                .map(|(contact_id, _)| contact_id)
                .collect();
            let live_enrollments = live(&enrollments, &people).await?;
            exit_all(ctx, live_enrollments, ExitReason::Objection, now).await
        }
        _ => Ok(()),
    }
}

async fn live(enrollments: &EnrollmentStore, people: &[Uuid]) -> Result<BTreeSet<Uuid>, AppError> {
    if people.is_empty() {
        return Ok(BTreeSet::new());
    }
    Ok(enrollments
        .live_for_contacts(people)
        .await?
        .into_iter()
        .map(|enrollment| enrollment.id)
        .collect())
}

async fn exit_all(
    ctx: &Ctx<'_>,
    enrollments: BTreeSet<Uuid>,
    reason: ExitReason,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    for enrollment_id in enrollments {
        exit_enrollment(ctx, enrollment_id, reason, now).await?;
    }
    Ok(())
}
