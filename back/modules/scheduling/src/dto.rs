use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::{
    Availability, BookCommand, Booking, BookingListQuery, BookingStatus, ContactFieldOption,
    LocationKind, MeetingType, PublicBooking, PublicMeetingType, Question, QuestionInput,
    QuestionKind, SaveAvailabilityCommand, SaveMeetingTypeCommand, Slot, WeeklyIntervalInput,
};
use aphel_kernel::api::dto::common::double_option;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SchedulingWeeklyInterval {
    #[schema(minimum = 1, maximum = 7, example = 1)]
    pub(crate) weekday: u8,
    #[schema(example = "09:00")]
    pub(crate) start: String,
    #[schema(example = "18:00")]
    pub(crate) end: String,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SchedulingUserParams {
    pub(crate) user_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SchedulingAvailabilityRequest {
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: Option<String>,
    pub(crate) weekly_hours: Option<Vec<SchedulingWeeklyInterval>>,
    #[schema(example = 240)]
    pub(crate) min_notice_minutes: Option<u32>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<u32>, example = 6)]
    pub(crate) max_per_day: Option<Option<u32>>,
    #[schema(example = 0)]
    pub(crate) buffer_before_minutes: Option<u32>,
    #[schema(example = 15)]
    pub(crate) buffer_after_minutes: Option<u32>,
    #[schema(example = 60)]
    pub(crate) horizon_days: Option<u32>,
}

impl SchedulingAvailabilityRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveAvailabilityCommand {
        SaveAvailabilityCommand {
            timezone: self.timezone,
            weekly_hours: self.weekly_hours.map(|intervals| {
                intervals
                    .into_iter()
                    .map(|interval| WeeklyIntervalInput {
                        weekday: interval.weekday,
                        start: interval.start,
                        end: interval.end,
                    })
                    .collect()
            }),
            min_notice_minutes: self.min_notice_minutes,
            max_per_day: self.max_per_day,
            buffer_before_minutes: self.buffer_before_minutes,
            buffer_after_minutes: self.buffer_after_minutes,
            horizon_days: self.horizon_days,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SchedulingAvailabilityResponse {
    pub(crate) user_id: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) weekly_hours: Vec<SchedulingWeeklyInterval>,
    pub(crate) min_notice_minutes: u32,
    pub(crate) max_per_day: Option<u32>,
    pub(crate) buffer_before_minutes: u32,
    pub(crate) buffer_after_minutes: u32,
    pub(crate) horizon_days: u32,
    pub(crate) configured: bool,
    pub(crate) updated_at: Option<String>,
}

impl From<Availability> for SchedulingAvailabilityResponse {
    fn from(availability: Availability) -> Self {
        Self {
            user_id: availability.user_id.into_inner(),
            timezone: availability.timezone.name().to_owned(),
            weekly_hours: availability
                .weekly
                .iter()
                .map(|interval| SchedulingWeeklyInterval {
                    weekday: interval.weekday,
                    start: interval.start_text(),
                    end: interval.end_text(),
                })
                .collect(),
            min_notice_minutes: availability.min_notice_minutes,
            max_per_day: availability.max_per_day,
            buffer_before_minutes: availability.buffer_before_minutes,
            buffer_after_minutes: availability.buffer_after_minutes,
            horizon_days: availability.horizon_days,
            configured: availability.configured,
            updated_at: availability.updated_at.map(timestamp),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = MeetingLocationKind)]
pub(crate) enum MeetingLocationKindSchema {
    None,
    Video,
    Address,
    Phone,
}

impl From<LocationKind> for MeetingLocationKindSchema {
    fn from(value: LocationKind) -> Self {
        match value {
            LocationKind::None => Self::None,
            LocationKind::Video => Self::Video,
            LocationKind::Address => Self::Address,
            LocationKind::Phone => Self::Phone,
        }
    }
}

impl From<MeetingLocationKindSchema> for LocationKind {
    fn from(value: MeetingLocationKindSchema) -> Self {
        match value {
            MeetingLocationKindSchema::None => Self::None,
            MeetingLocationKindSchema::Video => Self::Video,
            MeetingLocationKindSchema::Address => Self::Address,
            MeetingLocationKindSchema::Phone => Self::Phone,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = MeetingQuestionKind)]
pub(crate) enum MeetingQuestionKindSchema {
    Text,
    LongText,
}

impl From<QuestionKind> for MeetingQuestionKindSchema {
    fn from(value: QuestionKind) -> Self {
        match value {
            QuestionKind::Text => Self::Text,
            QuestionKind::LongText => Self::LongText,
        }
    }
}

impl From<MeetingQuestionKindSchema> for QuestionKind {
    fn from(value: MeetingQuestionKindSchema) -> Self {
        match value {
            MeetingQuestionKindSchema::Text => Self::Text,
            MeetingQuestionKindSchema::LongText => Self::LongText,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = MeetingBookingStatus)]
pub(crate) enum MeetingBookingStatusSchema {
    Confirmed,
    Canceled,
}

impl From<BookingStatus> for MeetingBookingStatusSchema {
    fn from(value: BookingStatus) -> Self {
        match value {
            BookingStatus::Confirmed => Self::Confirmed,
            BookingStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<MeetingBookingStatusSchema> for BookingStatus {
    fn from(value: MeetingBookingStatusSchema) -> Self {
        match value {
            MeetingBookingStatusSchema::Confirmed => Self::Confirmed,
            MeetingBookingStatusSchema::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeetingQuestionRequest {
    #[schema(example = "organizations.organization")]
    pub(crate) key: Option<String>,
    #[schema(example = "Company")]
    pub(crate) label: String,
    pub(crate) kind: Option<MeetingQuestionKindSchema>,
    #[serde(default)]
    pub(crate) required: bool,
    #[schema(example = "job_title")]
    pub(crate) contact_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct MeetingQuestionResponse {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: MeetingQuestionKindSchema,
    pub(crate) required: bool,
    pub(crate) contact_field: Option<String>,
}

impl From<&Question> for MeetingQuestionResponse {
    fn from(question: &Question) -> Self {
        Self {
            key: question.key.clone(),
            label: question.label.clone(),
            kind: question.kind.into(),
            required: question.required,
            contact_field: question.contact_field.clone(),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeetingTypeRequest {
    #[schema(example = "Intro call")]
    pub(crate) name: Option<String>,
    #[schema(example = "intro-call-ana")]
    pub(crate) slug: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(example = 30)]
    pub(crate) duration_minutes: Option<u32>,
    #[schema(example = 30)]
    pub(crate) slot_interval_minutes: Option<u32>,
    pub(crate) location_kind: Option<MeetingLocationKindSchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "https://meet.example.com/ana")]
    pub(crate) location_value: Option<Option<String>>,
    pub(crate) questions: Option<Vec<MeetingQuestionRequest>>,
    pub(crate) spam_protection: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) distribution_rule_id: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
}

impl MeetingTypeRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveMeetingTypeCommand {
        SaveMeetingTypeCommand {
            name: self.name,
            slug: self.slug,
            description: self.description,
            duration_minutes: self.duration_minutes,
            slot_interval_minutes: self.slot_interval_minutes,
            location_kind: self.location_kind.map(Into::into),
            location_value: self.location_value,
            questions: self.questions.map(|questions| {
                questions
                    .into_iter()
                    .map(|question| QuestionInput {
                        key: question.key,
                        label: question.label,
                        kind: question.kind.map(Into::into),
                        required: question.required,
                        contact_field: question.contact_field,
                    })
                    .collect()
            }),
            spam_protection: self.spam_protection,
            distribution_rule_id: self.distribution_rule_id,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct SchedulingReference {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MeetingTypeResponse {
    pub(crate) id: String,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) duration_minutes: u32,
    pub(crate) slot_interval_minutes: u32,
    pub(crate) location_kind: MeetingLocationKindSchema,
    pub(crate) location_value: Option<String>,
    pub(crate) questions: Vec<MeetingQuestionResponse>,
    pub(crate) spam_protection: bool,
    #[schema(example = "personal")]
    pub(crate) kind: String,
    pub(crate) owner: SchedulingReference,
    pub(crate) distribution_rule: Option<SchedulingReference>,
    pub(crate) is_active: bool,
    #[schema(example = "/book/intro-call-ana")]
    pub(crate) booking_path: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<MeetingType> for MeetingTypeResponse {
    fn from(meeting_type: MeetingType) -> Self {
        Self {
            id: meeting_type.id.to_string(),
            booking_path: format!("/book/{}", meeting_type.slug),
            kind: if meeting_type.is_team() {
                "team"
            } else {
                "personal"
            }
            .to_owned(),
            questions: meeting_type
                .questions
                .iter()
                .map(MeetingQuestionResponse::from)
                .collect(),
            distribution_rule: meeting_type
                .distribution_rule_id
                .map(|id| SchedulingReference {
                    id: id.to_string(),
                    name: meeting_type
                        .distribution_rule_name
                        .clone()
                        .unwrap_or_default(),
                }),
            owner: SchedulingReference {
                id: meeting_type.owner_id.as_str().to_owned(),
                name: meeting_type.owner_name,
            },
            slug: meeting_type.slug,
            name: meeting_type.name,
            description: meeting_type.description,
            duration_minutes: meeting_type.duration_minutes,
            slot_interval_minutes: meeting_type.slot_interval_minutes,
            location_kind: meeting_type.location_kind.into(),
            location_value: meeting_type.location_value,
            spam_protection: meeting_type.spam_protection,
            is_active: meeting_type.is_active,
            created_at: timestamp(meeting_type.created_at),
            updated_at: timestamp(meeting_type.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SchedulingContactFieldResponse {
    #[schema(example = "job_title")]
    pub(crate) key: String,
    #[schema(example = "Job title")]
    pub(crate) label: String,
    pub(crate) is_custom: bool,
}

impl From<ContactFieldOption> for SchedulingContactFieldResponse {
    fn from(option: ContactFieldOption) -> Self {
        Self {
            key: option.key,
            label: option.label,
            is_custom: option.is_custom,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct MeetingBookingListParams {
    pub(crate) status: Option<MeetingBookingStatusSchema>,
    pub(crate) meeting_type_id: Option<String>,
    pub(crate) upcoming: Option<bool>,
}

impl MeetingBookingListParams {
    pub(crate) fn into_query(self) -> BookingListQuery {
        BookingListQuery {
            status: self.status.map(Into::into),
            meeting_type_id: self.meeting_type_id,
            upcoming: self.upcoming.unwrap_or(false),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeetingBookingCancelRequest {
    #[schema(example = "Scheduling conflict")]
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MeetingBookingResponse {
    pub(crate) id: String,
    pub(crate) meeting_type: SchedulingReference,
    pub(crate) host: SchedulingReference,
    pub(crate) contact_id: Option<String>,
    pub(crate) activity_id: Option<String>,
    pub(crate) status: MeetingBookingStatusSchema,
    pub(crate) starts_at: String,
    pub(crate) ends_at: String,
    pub(crate) guest_name: String,
    pub(crate) guest_email: String,
    pub(crate) guest_timezone: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) answers: Map<String, Value>,
    pub(crate) cancel_reason: Option<String>,
    pub(crate) canceled_at: Option<String>,
    pub(crate) created_at: String,
}

impl From<Booking> for MeetingBookingResponse {
    fn from(booking: Booking) -> Self {
        Self {
            id: booking.id.to_string(),
            meeting_type: SchedulingReference {
                id: booking.meeting_type_id.to_string(),
                name: booking.meeting_type_name,
            },
            host: SchedulingReference {
                id: booking.host_user_id.as_str().to_owned(),
                name: booking.host_name,
            },
            contact_id: booking.contact_id.map(|id| id.to_string()),
            activity_id: booking.activity_id.map(|id| id.to_string()),
            status: booking.status.into(),
            starts_at: timestamp(booking.starts_at),
            ends_at: timestamp(booking.ends_at),
            guest_name: booking.guest_name,
            guest_email: booking.guest_email,
            guest_timezone: booking.guest_timezone,
            answers: booking.answers,
            cancel_reason: booking.cancel_reason,
            canceled_at: booking.canceled_at.map(timestamp),
            created_at: timestamp(booking.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicMeetingQuestion {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: MeetingQuestionKindSchema,
    pub(crate) required: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicMeetingTypeResponse {
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) duration_minutes: u32,
    pub(crate) location_kind: MeetingLocationKindSchema,
    pub(crate) host_name: Option<String>,
    pub(crate) organization_name: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) questions: Vec<PublicMeetingQuestion>,
    pub(crate) spam_protection: bool,
}

impl From<PublicMeetingType> for PublicMeetingTypeResponse {
    fn from(public: PublicMeetingType) -> Self {
        let meeting_type = public.meeting_type;
        Self {
            questions: meeting_type
                .questions
                .iter()
                .map(|question| PublicMeetingQuestion {
                    key: question.key.clone(),
                    label: question.label.clone(),
                    kind: question.kind.into(),
                    required: question.required,
                })
                .collect(),
            slug: meeting_type.slug,
            name: meeting_type.name,
            description: meeting_type.description,
            duration_minutes: meeting_type.duration_minutes,
            location_kind: meeting_type.location_kind.into(),
            host_name: public.host_name,
            organization_name: public.organization_name,
            timezone: public.timezone.name().to_owned(),
            spam_protection: meeting_type.spam_protection,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PublicSlotsParams {
    #[param(example = "2026-10-26T00:00:00-03:00")]
    pub(crate) from: String,
    #[param(example = "2026-11-02T00:00:00-03:00")]
    pub(crate) to: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicSlot {
    pub(crate) starts_at: String,
    pub(crate) ends_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicSlotsResponse {
    pub(crate) slots: Vec<PublicSlot>,
}

impl From<Vec<Slot>> for PublicSlotsResponse {
    fn from(slots: Vec<Slot>) -> Self {
        Self {
            slots: slots
                .into_iter()
                .map(|slot| PublicSlot {
                    starts_at: timestamp(slot.starts_at),
                    ends_at: timestamp(slot.ends_at),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PublicBookingRequest {
    #[schema(example = "2026-10-27T13:00:00Z")]
    pub(crate) starts_at: String,
    #[schema(example = "Maria Souza")]
    pub(crate) name: String,
    #[schema(example = "maria@customer.com")]
    pub(crate) email: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: Option<String>,
    #[schema(example = "pt-BR")]
    pub(crate) locale: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) answers: Map<String, Value>,
    pub(crate) website: Option<String>,
}

impl PublicBookingRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> BookCommand {
        BookCommand {
            starts_at: self.starts_at,
            name: self.name,
            email: self.email,
            timezone: self.timezone,
            locale: self.locale,
            answers: self.answers,
            honeypot: self.website,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicRescheduleRequest {
    #[schema(example = "2026-10-28T14:00:00Z")]
    pub(crate) starts_at: String,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicCancelRequest {
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicBookingResponse {
    pub(crate) status: MeetingBookingStatusSchema,
    pub(crate) starts_at: String,
    pub(crate) ends_at: String,
    pub(crate) slug: String,
    pub(crate) meeting_type_name: String,
    pub(crate) duration_minutes: u32,
    pub(crate) host_name: String,
    pub(crate) organization_name: String,
    pub(crate) location_kind: MeetingLocationKindSchema,
    pub(crate) location_value: Option<String>,
    pub(crate) guest_name: String,
    pub(crate) guest_email: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) cancel_reason: Option<String>,
    pub(crate) can_change: bool,
    pub(crate) manage_token: String,
}

impl PublicBookingResponse {
    pub(crate) fn from_public(public: PublicBooking, now: DateTime<Utc>) -> Self {
        let booking = public.booking;
        Self {
            status: booking.status.into(),
            can_change: booking.status == BookingStatus::Confirmed && booking.starts_at > now,
            starts_at: timestamp(booking.starts_at),
            ends_at: timestamp(booking.ends_at),
            slug: public.meeting_type.slug,
            meeting_type_name: public.meeting_type.name,
            duration_minutes: public.meeting_type.duration_minutes,
            host_name: booking.host_name,
            organization_name: public.organization_name,
            location_kind: public.meeting_type.location_kind.into(),
            location_value: public.meeting_type.location_value,
            guest_name: booking.guest_name,
            guest_email: booking.guest_email,
            timezone: public.host_timezone.name().to_owned(),
            cancel_reason: booking.cancel_reason,
            manage_token: public.manage_token,
        }
    }
}
