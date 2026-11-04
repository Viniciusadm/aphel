use crate::timeline::LinkedRecordResponse;
use aphel_kernel::api::dto::files::UserReferenceResponse;
use chrono::Weekday;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ActivityStatus)]
pub enum ActivityStatusSchema {
    Open,
    Done,
    Canceled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ActivityPriority)]
pub enum ActivityPrioritySchema {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RecurrenceFrequency)]
pub enum RecurrenceFrequencySchema {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RecurrenceWeekday)]
pub enum RecurrenceWeekdaySchema {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityRecurrenceResponse {
    pub series_id: String,
    pub index: u32,
    pub frequency: RecurrenceFrequencySchema,
    pub interval: u32,
    pub by_weekday: Vec<RecurrenceWeekdaySchema>,
    pub month_day: Option<u32>,
    pub until: Option<String>,
    pub count: Option<u32>,
    #[schema(example = "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31")]
    pub rrule: String,
    pub ended: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityLocationResponse {
    #[schema(example = -23.55052)]
    pub latitude: f64,
    #[schema(example = -46.633308)]
    pub longitude: f64,
    #[schema(example = 12)]
    pub accuracy_m: Option<u32>,
    pub captured_at: String,
    #[schema(
        example = "https://www.openstreetmap.org/?mlat=-23.55052&mlon=-46.633308#map=17/-23.55052/-46.633308"
    )]
    pub map_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityResponse {
    pub id: String,
    pub title: String,
    pub activity_type_id: String,
    pub status: ActivityStatusSchema,
    pub priority: ActivityPrioritySchema,
    pub due_at: Option<String>,
    pub completed_at: Option<String>,
    pub owner_id: Option<String>,
    pub created_by: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub duration_minutes: Option<u32>,
    pub description: Option<String>,
    pub location: Option<ActivityLocationResponse>,
    #[schema(example = -23.55052)]
    pub location_latitude: Option<f64>,
    #[schema(example = -46.633308)]
    pub location_longitude: Option<f64>,
    #[schema(example = 12)]
    pub location_accuracy_m: Option<u32>,
    pub location_captured_at: Option<String>,
    pub reminder_minutes_before: Option<u32>,
    pub recurrence: Option<ActivityRecurrenceResponse>,
    pub created_at: String,
    pub updated_at: String,
    #[schema(value_type = Object)]
    pub custom_fields: Map<String, Value>,
    pub activity_type: ActivityTypeSummaryResponse,
    pub owner: Option<UserReferenceResponse>,
    pub record: Option<LinkedRecordResponse>,
    pub links: Vec<LinkedRecordResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityTypeSummaryResponse {
    pub id: String,
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl From<Weekday> for RecurrenceWeekdaySchema {
    fn from(value: Weekday) -> Self {
        match value {
            Weekday::Mon => Self::Mo,
            Weekday::Tue => Self::Tu,
            Weekday::Wed => Self::We,
            Weekday::Thu => Self::Th,
            Weekday::Fri => Self::Fr,
            Weekday::Sat => Self::Sa,
            Weekday::Sun => Self::Su,
        }
    }
}

impl From<RecurrenceWeekdaySchema> for Weekday {
    fn from(value: RecurrenceWeekdaySchema) -> Self {
        match value {
            RecurrenceWeekdaySchema::Mo => Self::Mon,
            RecurrenceWeekdaySchema::Tu => Self::Tue,
            RecurrenceWeekdaySchema::We => Self::Wed,
            RecurrenceWeekdaySchema::Th => Self::Thu,
            RecurrenceWeekdaySchema::Fr => Self::Fri,
            RecurrenceWeekdaySchema::Sa => Self::Sat,
            RecurrenceWeekdaySchema::Su => Self::Sun,
        }
    }
}
