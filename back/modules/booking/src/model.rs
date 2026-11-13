use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::RecordScopeFilter;

pub(crate) const AVAILABILITY_UPDATED: &str = "booking_availability.updated";
pub(crate) const MEETING_TYPE_CREATED: &str = "meeting_type.created";
pub(crate) const MEETING_TYPE_UPDATED: &str = "meeting_type.updated";
pub(crate) const MEETING_TYPE_DELETED: &str = "meeting_type.deleted";

pub(crate) const AVAILABILITY_RECORD: RecordType =
    RecordType::from_static("booking.booking_availability");
pub(crate) const MEETING_TYPE_RECORD: RecordType = RecordType::from_static("booking.meeting_type");
pub(crate) const BOOKING_RECORD: RecordType = RecordType::from_static("booking.meeting_booking");

pub(crate) const MEETING_TYPE_NOT_FOUND: &str = "meeting_type_not_found";
pub(crate) const MEETING_TYPE_SLUG_TAKEN: &str = "meeting_type_slug_taken";
pub(crate) const BOOKING_NOT_FOUND: &str = "booking_not_found";
pub(crate) const BOOKING_SLOT_UNAVAILABLE: &str = "booking_slot_unavailable";
pub(crate) const BOOKING_NOT_CHANGEABLE: &str = "booking_not_changeable";
pub(crate) const BOOKING_REJECTED: &str = "booking_rejected";

pub(crate) const BOOKING_EMAIL_JOB: &str = "booking.booking_email";

pub(crate) const ACTIVITY_TYPE_KEY: &str = "meeting";
pub(crate) const TOKEN_KEY_OWNER: &str = "booking";
pub(crate) const TOKEN_KEY_NAME: &str = "booking_token_key";

pub(crate) const DEFAULT_START_MINUTE: u16 = 9 * 60;
pub(crate) const DEFAULT_END_MINUTE: u16 = 18 * 60;
pub(crate) const DEFAULT_MIN_NOTICE_MINUTES: u32 = 240;
pub(crate) const DEFAULT_HORIZON_DAYS: u32 = 60;
pub(crate) const DEFAULT_SLOT_INTERVAL_MINUTES: u32 = 30;
pub(crate) const MIN_DURATION_MINUTES: u32 = 5;
pub(crate) const MAX_DURATION_MINUTES: u32 = 480;
pub(crate) const MAX_SLOT_INTERVAL_MINUTES: u32 = 240;
pub(crate) const MAX_MIN_NOTICE_MINUTES: u32 = 60 * 24 * 30;
pub(crate) const MAX_PER_DAY: u32 = 50;
pub(crate) const MAX_BUFFER_MINUTES: u32 = 240;
pub(crate) const MAX_HORIZON_DAYS: u32 = 365;
pub(crate) const MAX_WEEKLY_INTERVALS: usize = 28;
pub(crate) const MAX_QUESTIONS: usize = 10;
pub(crate) const MAX_QUESTION_LABEL: usize = 200;
pub(crate) const MAX_ANSWER_CHARS: usize = 2000;
pub(crate) const MAX_NAME_CHARS: usize = 140;
pub(crate) const MAX_DESCRIPTION_CHARS: usize = 2000;
pub(crate) const MAX_LOCATION_CHARS: usize = 500;
pub(crate) const MAX_GUEST_NAME_CHARS: usize = 200;
pub(crate) const MAX_CANCEL_REASON_CHARS: usize = 500;
pub(crate) const MAX_SLUG_CHARS: usize = 80;
pub(crate) const MIN_SLUG_CHARS: usize = 3;
pub(crate) const MAX_WINDOW_DAYS: i64 = 42;
pub(crate) const RESERVED_SLUGS: [&str; 2] = ["manage", "public"];

pub(crate) const MAPPABLE_CONTACT_FIELDS: [&str; 7] = [
    "phone",
    "mobile",
    "job_title",
    "notes",
    "address_city",
    "address_state",
    "address_country",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct WeeklyInterval {
    pub(crate) weekday: u8,
    pub(crate) start_minute: u16,
    pub(crate) end_minute: u16,
}

impl WeeklyInterval {
    pub(crate) fn start_text(&self) -> String {
        clock_text(self.start_minute)
    }

    pub(crate) fn end_text(&self) -> String {
        clock_text(self.end_minute)
    }
}

pub(crate) fn clock_text(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

pub(crate) fn parse_clock(value: &str) -> Option<u16> {
    let (hours, minutes) = value.trim().split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    let hours: u16 = hours.parse().ok()?;
    let minutes: u16 = minutes.parse().ok()?;
    match (hours, minutes) {
        (24, 0) => Some(24 * 60),
        (0..=23, 0..=59) => Some(hours * 60 + minutes),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Availability {
    pub(crate) user_id: UserId,
    pub(crate) timezone: Tz,
    pub(crate) weekly: Vec<WeeklyInterval>,
    pub(crate) min_notice_minutes: u32,
    pub(crate) max_per_day: Option<u32>,
    pub(crate) buffer_before_minutes: u32,
    pub(crate) buffer_after_minutes: u32,
    pub(crate) horizon_days: u32,
    pub(crate) configured: bool,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

impl Availability {
    pub(crate) fn defaults(user_id: UserId, timezone: Tz) -> Self {
        Self {
            user_id,
            timezone,
            weekly: (1..=5)
                .map(|weekday| WeeklyInterval {
                    weekday,
                    start_minute: DEFAULT_START_MINUTE,
                    end_minute: DEFAULT_END_MINUTE,
                })
                .collect(),
            min_notice_minutes: DEFAULT_MIN_NOTICE_MINUTES,
            max_per_day: None,
            buffer_before_minutes: 0,
            buffer_after_minutes: 0,
            horizon_days: DEFAULT_HORIZON_DAYS,
            configured: false,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AvailabilitySnapshot {
    pub(crate) timezone: String,
    pub(crate) weekly_hours: Vec<String>,
    pub(crate) min_notice_minutes: u32,
    pub(crate) max_per_day: Option<u32>,
    pub(crate) buffer_before_minutes: u32,
    pub(crate) buffer_after_minutes: u32,
    pub(crate) horizon_days: u32,
}

impl From<&Availability> for AvailabilitySnapshot {
    fn from(availability: &Availability) -> Self {
        Self {
            timezone: availability.timezone.name().to_owned(),
            weekly_hours: availability
                .weekly
                .iter()
                .map(|interval| {
                    format!(
                        "{} {}-{}",
                        interval.weekday,
                        interval.start_text(),
                        interval.end_text()
                    )
                })
                .collect(),
            min_notice_minutes: availability.min_notice_minutes,
            max_per_day: availability.max_per_day,
            buffer_before_minutes: availability.buffer_before_minutes,
            buffer_after_minutes: availability.buffer_after_minutes,
            horizon_days: availability.horizon_days,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocationKind {
    None,
    Video,
    Address,
    Phone,
}

impl LocationKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Video => "video",
            Self::Address => "address",
            Self::Phone => "phone",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "video" => Some(Self::Video),
            "address" => Some(Self::Address),
            "phone" => Some(Self::Phone),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QuestionKind {
    Text,
    LongText,
}

impl QuestionKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::LongText => "long_text",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "long_text" => Some(Self::LongText),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Question {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: QuestionKind,
    pub(crate) required: bool,
    pub(crate) contact_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeetingType {
    pub(crate) id: Uuid,
    pub(crate) owner_id: UserId,
    pub(crate) owner_name: String,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) duration_minutes: u32,
    pub(crate) slot_interval_minutes: u32,
    pub(crate) location_kind: LocationKind,
    pub(crate) location_value: Option<String>,
    pub(crate) questions: Vec<Question>,
    pub(crate) spam_protection: bool,
    pub(crate) distribution_rule_id: Option<Uuid>,
    pub(crate) distribution_rule_name: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl MeetingType {
    pub(crate) fn is_team(&self) -> bool {
        self.distribution_rule_id.is_some()
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeetingTypeSnapshot {
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) duration_minutes: u32,
    pub(crate) slot_interval_minutes: u32,
    pub(crate) location_kind: LocationKind,
    pub(crate) location_value: Option<String>,
    pub(crate) questions: Vec<Question>,
    pub(crate) spam_protection: bool,
    pub(crate) distribution_rule_id: Option<Uuid>,
    pub(crate) is_active: bool,
}

impl From<&MeetingType> for MeetingTypeSnapshot {
    fn from(meeting_type: &MeetingType) -> Self {
        Self {
            slug: meeting_type.slug.clone(),
            name: meeting_type.name.clone(),
            description: meeting_type.description.clone(),
            duration_minutes: meeting_type.duration_minutes,
            slot_interval_minutes: meeting_type.slot_interval_minutes,
            location_kind: meeting_type.location_kind,
            location_value: meeting_type.location_value.clone(),
            questions: meeting_type.questions.clone(),
            spam_protection: meeting_type.spam_protection,
            distribution_rule_id: meeting_type.distribution_rule_id,
            is_active: meeting_type.is_active,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookingStatus {
    Confirmed,
    Canceled,
}

impl BookingStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Canceled => "canceled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "confirmed" => Some(Self::Confirmed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Booking {
    pub(crate) id: Uuid,
    pub(crate) meeting_type_id: Uuid,
    pub(crate) meeting_type_name: String,
    pub(crate) host_user_id: UserId,
    pub(crate) host_name: String,
    pub(crate) record_type: String,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) activity_id: Option<Uuid>,
    pub(crate) status: BookingStatus,
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) ends_at: DateTime<Utc>,
    pub(crate) guest_name: String,
    pub(crate) guest_email: String,
    pub(crate) guest_timezone: Option<String>,
    pub(crate) guest_locale: Option<String>,
    pub(crate) answers: Map<String, Value>,
    pub(crate) revision: u32,
    pub(crate) cancel_reason: Option<String>,
    pub(crate) canceled_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Slot {
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostProfile {
    pub(crate) user_id: UserId,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) timezone: Option<String>,
    pub(crate) locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrganizationProfile {
    pub(crate) name: String,
    pub(crate) timezone: Tz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContactFieldOption {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) is_custom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WeeklyIntervalInput {
    pub(crate) weekday: u8,
    pub(crate) start: String,
    pub(crate) end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveAvailabilityCommand {
    pub(crate) timezone: Option<String>,
    pub(crate) weekly_hours: Option<Vec<WeeklyIntervalInput>>,
    pub(crate) min_notice_minutes: Option<u32>,
    pub(crate) max_per_day: Option<Option<u32>>,
    pub(crate) buffer_before_minutes: Option<u32>,
    pub(crate) buffer_after_minutes: Option<u32>,
    pub(crate) horizon_days: Option<u32>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct QuestionInput {
    pub(crate) key: Option<String>,
    pub(crate) label: String,
    pub(crate) kind: Option<QuestionKind>,
    pub(crate) required: bool,
    pub(crate) contact_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveMeetingTypeCommand {
    pub(crate) name: Option<String>,
    pub(crate) slug: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) duration_minutes: Option<u32>,
    pub(crate) slot_interval_minutes: Option<u32>,
    pub(crate) location_kind: Option<LocationKind>,
    pub(crate) location_value: Option<Option<String>>,
    pub(crate) questions: Option<Vec<QuestionInput>>,
    pub(crate) spam_protection: Option<bool>,
    pub(crate) distribution_rule_id: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BookingListQuery {
    pub(crate) status: Option<BookingStatus>,
    pub(crate) meeting_type_id: Option<String>,
    pub(crate) upcoming: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct BookCommand {
    pub(crate) starts_at: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) timezone: Option<String>,
    pub(crate) locale: Option<String>,
    pub(crate) answers: Map<String, Value>,
    pub(crate) honeypot: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotWindow {
    pub(crate) from: DateTime<Utc>,
    pub(crate) to: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PublicMeetingType {
    pub(crate) meeting_type: MeetingType,
    pub(crate) host_name: Option<String>,
    pub(crate) organization_name: String,
    pub(crate) timezone: Tz,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PublicBooking {
    pub(crate) booking: Booking,
    pub(crate) meeting_type: MeetingType,
    pub(crate) organization_name: String,
    pub(crate) host_timezone: Tz,
    pub(crate) manage_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeetingTypeFilter {
    pub(crate) owners: RecordScopeFilter,
    pub(crate) window: ListWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookingFilter {
    pub(crate) hosts: RecordScopeFilter,
    pub(crate) status: Option<BookingStatus>,
    pub(crate) meeting_type_id: Option<Uuid>,
    pub(crate) after: Option<DateTime<Utc>>,
    pub(crate) between: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub(crate) without_activity: bool,
    pub(crate) window: ListWindow,
}

pub(crate) fn email_job_payload(booking_id: Uuid, kind: &str, recipient: &str) -> Value {
    serde_json::json!({
        "booking_id": booking_id,
        "kind": kind,
        "recipient": recipient,
    })
}
