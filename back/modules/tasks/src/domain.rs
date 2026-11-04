use aphel_contract_crm::CrmRecordTypes;
use chrono::{DateTime, SecondsFormat, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::recurrence::{ActivityRecurrenceView, ApplyTo, OccurrenceRef, RecurrenceRuleInput};
use aphel_kernel::application::fields::CustomFieldMap;
use aphel_kernel::application::fields::permissions::provided_key;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::RecordQuery;
use aphel_kernel::application::shared::{FieldChange, RecordRef};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::fields::is_identifier;
use aphel_kernel::errors::AppError;

pub(crate) const ACTIVITY_NOT_FOUND: &str = "activity_not_found";
pub(crate) const ACTIVITY_ALREADY_COMPLETED: &str = "activity_already_completed";
pub(crate) const ACTIVITY_ALREADY_OPEN: &str = "activity_already_open";
pub(crate) const ACTIVITY_TYPE_NOT_FOUND: &str = "activity_type_not_found";
pub(crate) const ACTIVITY_TYPE_KEY_TAKEN: &str = "activity_type_key_taken";
pub(crate) const LAST_ACTIVE_ACTIVITY_TYPE: &str = "last_active_activity_type";
pub(crate) const MAX_ACTIVITY_TYPES: usize = 100;
pub(crate) const MAX_LINKED_ACTIVITY_IDS: u64 = 1000;
pub(crate) const MAX_CALENDAR_RANGE_DAYS: i64 = 62;
pub(crate) const MAX_CALENDAR_ITEMS: u64 = 1000;
pub(crate) const MAX_INBOX_ITEMS: u64 = 500;
pub(crate) const DEAL_AMOUNT_FIELD: &str = "amount";

pub(crate) fn activity_not_found() -> AppError {
    AppError::not_found_code(ACTIVITY_NOT_FOUND, "activity was not found")
}

pub(crate) fn activity_type_not_found() -> AppError {
    AppError::not_found_code(ACTIVITY_TYPE_NOT_FOUND, "activity type was not found")
}

pub(crate) const MAX_ACTIVITY_TITLE_LENGTH: usize = 255;
pub(crate) const MAX_ACTIVITY_DESCRIPTION_LENGTH: usize = 20_000;
pub(crate) const MAX_ACTIVITY_DURATION_MINUTES: u32 = 60 * 24 * 7;
pub(crate) const MAX_ACTIVITY_LINKS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityStatus {
    Open,
    Done,
    Canceled,
}

impl ActivityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
            Self::Canceled => "canceled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "done" => Some(Self::Done),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActivityPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl ActivityPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "urgent" => Some(Self::Urgent),
            _ => None,
        }
    }
}

pub(crate) fn normalize_activity_title(value: &str) -> Result<String, &'static str> {
    let title = value.trim();
    if title.is_empty() {
        return Err("must not be empty");
    }
    if title.chars().count() > MAX_ACTIVITY_TITLE_LENGTH {
        return Err("must have at most 255 characters");
    }
    Ok(title.to_owned())
}

pub(crate) fn normalize_activity_description(
    value: Option<&str>,
) -> Result<Option<String>, &'static str> {
    let Some(description) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if description.chars().count() > MAX_ACTIVITY_DESCRIPTION_LENGTH {
        return Err("must have at most 20000 characters");
    }
    Ok(Some(description.to_owned()))
}

pub(crate) fn validate_duration_minutes(value: i64) -> Result<u32, &'static str> {
    u32::try_from(value)
        .ok()
        .filter(|minutes| *minutes <= MAX_ACTIVITY_DURATION_MINUTES)
        .ok_or("must be an integer between 0 and 10080")
}

pub(crate) const MAX_REMINDER_MINUTES_BEFORE: u32 = 60 * 24 * 30;
pub(crate) const DEFAULT_REMINDER_MINUTES_BEFORE: u32 = 60;

pub(crate) fn validate_reminder_minutes_before(value: i64) -> Result<u32, &'static str> {
    u32::try_from(value)
        .ok()
        .filter(|minutes| *minutes <= MAX_REMINDER_MINUTES_BEFORE)
        .ok_or("must be an integer between 0 and 43200")
}

pub(crate) const MAX_ACTIVITY_TYPE_KEY_LENGTH: usize = 64;
pub(crate) const MAX_ACTIVITY_TYPE_NAME_LENGTH: usize = 140;
pub(crate) const MAX_ACTIVITY_TYPE_ICON_LENGTH: usize = 64;

pub(crate) fn normalize_activity_type_name(value: &str) -> Result<String, &'static str> {
    let name = value.trim();
    if name.is_empty() {
        return Err("must not be empty");
    }
    if name.chars().count() > MAX_ACTIVITY_TYPE_NAME_LENGTH {
        return Err("must have at most 140 characters");
    }
    Ok(name.to_owned())
}

pub(crate) fn validate_activity_type_key(value: &str) -> Result<String, &'static str> {
    if is_identifier(value, MAX_ACTIVITY_TYPE_KEY_LENGTH) {
        Ok(value.to_owned())
    } else {
        Err("must be lowercase letters, digits or single underscores and start with a letter")
    }
}

pub(crate) fn validate_activity_type_icon(value: &str) -> Result<String, &'static str> {
    let icon = value.trim();
    let valid = !icon.is_empty()
        && icon.len() <= MAX_ACTIVITY_TYPE_ICON_LENGTH
        && icon.starts_with(|character: char| character.is_ascii_lowercase())
        && icon.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if valid {
        Ok(icon.to_owned())
    } else {
        Err("must be an icon name in kebab case")
    }
}

pub(crate) fn validate_activity_type_color(value: &str) -> Result<String, &'static str> {
    let color = value.trim();
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    if valid {
        Ok(color.to_ascii_lowercase())
    } else {
        Err("must be a hex color such as #1f6feb")
    }
}

pub(crate) fn activity_type_key_from_name(name: &str) -> String {
    let mut key = String::new();
    for character in name.chars().flat_map(fold_accent) {
        if character.is_ascii_alphanumeric() {
            key.push(character.to_ascii_lowercase());
        } else if !key.is_empty() && !key.ends_with('_') {
            key.push('_');
        }
    }
    let mut key: String = key.trim_matches('_').chars().take(48).collect();
    while key.ends_with('_') {
        key.pop();
    }
    if key.is_empty() {
        return "activity_type".to_owned();
    }
    if !key.starts_with(|character: char| character.is_ascii_lowercase()) {
        key.insert_str(0, "type_");
    }
    key
}

fn fold_accent(character: char) -> Option<char> {
    let folded = match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other => other,
    };
    Some(folded)
}

pub(crate) const LOCATION_COORDINATE_SCALE: u32 = 6;
pub(crate) const MAX_LOCATION_ACCURACY_METERS: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityLocation {
    pub latitude: Decimal,
    pub longitude: Decimal,
    pub accuracy_m: Option<u32>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocationViolation {
    pub(crate) field: &'static str,
    pub(crate) message: &'static str,
}

impl LocationViolation {
    fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

fn coordinate(
    value: f64,
    limit: f64,
    field: &'static str,
    message: &'static str,
) -> Result<Decimal, LocationViolation> {
    if !value.is_finite() || value.abs() > limit {
        return Err(LocationViolation::new(field, message));
    }
    Decimal::from_f64(value)
        .map(|decimal| decimal.round_dp(LOCATION_COORDINATE_SCALE))
        .ok_or(LocationViolation::new(field, message))
}

impl ActivityLocation {
    pub(crate) fn new(
        latitude: f64,
        longitude: f64,
        accuracy_m: Option<f64>,
        captured_at: DateTime<Utc>,
    ) -> Result<Self, Vec<LocationViolation>> {
        let mut violations = Vec::new();
        let latitude = coordinate(
            latitude,
            90.0,
            "location.latitude",
            "must be a number between -90 and 90",
        )
        .map_err(|violation| violations.push(violation))
        .ok();
        let longitude = coordinate(
            longitude,
            180.0,
            "location.longitude",
            "must be a number between -180 and 180",
        )
        .map_err(|violation| violations.push(violation))
        .ok();
        let accuracy_m = match accuracy_m {
            None => Some(None),
            Some(value)
                if value.is_finite()
                    && value >= 0.0
                    && value <= f64::from(MAX_LOCATION_ACCURACY_METERS) =>
            {
                Some(Some(value.round() as u32))
            }
            Some(_) => {
                violations.push(LocationViolation::new(
                    "location.accuracy_m",
                    "must be a number of meters between 0 and 100000",
                ));
                None
            }
        };
        match (latitude, longitude, accuracy_m) {
            (Some(latitude), Some(longitude), Some(accuracy_m)) if violations.is_empty() => {
                Ok(Self {
                    latitude,
                    longitude,
                    accuracy_m,
                    captured_at,
                })
            }
            _ => Err(violations),
        }
    }

    pub fn map_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/?mlat={lat}&mlon={lng}#map=17/{lat}/{lng}",
            lat = self.latitude,
            lng = self.longitude,
        )
    }
}

pub use aphel_sdk::tasks::ActivityTypeView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTypeSummary {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

pub use aphel_sdk::timeline::LinkedRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityView {
    pub id: Uuid,
    pub title: String,
    pub activity_type: ActivityTypeSummary,
    pub status: ActivityStatus,
    pub priority: ActivityPriority,
    pub due_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub owner: Option<UserReference>,
    pub created_by: Option<UserReference>,
    pub record: Option<LinkedRecord>,
    pub links: Vec<LinkedRecord>,
    pub duration_minutes: Option<u32>,
    pub description: Option<String>,
    pub location: Option<ActivityLocation>,
    pub(crate) reminder_minutes_before: Option<u32>,
    pub(crate) recurrence: Option<ActivityRecurrenceView>,
    pub(crate) created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ActivityView {
    pub(crate) fn record_ref(&self) -> RecordRef {
        RecordRef::new(
            aphel_kernel::application::shared::RecordType::ACTIVITY,
            self.id,
        )
    }

    pub(crate) fn data(&self) -> ActivityData {
        ActivityData {
            title: self.title.clone(),
            activity_type_id: self.activity_type.id,
            status: self.status,
            priority: self.priority,
            due_at: self.due_at,
            completed_at: self.completed_at,
            owner_id: self.owner.as_ref().map(|owner| owner.id.clone()),
            record: self.record.as_ref().map(|linked| linked.record.clone()),
            links: self
                .links
                .iter()
                .map(|linked| linked.record.clone())
                .collect(),
            duration_minutes: self.duration_minutes,
            description: self.description.clone(),
            location: self.location,
            reminder_minutes_before: self.reminder_minutes_before,
            occurrence: self.recurrence.as_ref().map(|recurrence| OccurrenceRef {
                series_id: recurrence.series_id,
                index: recurrence.index,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityRecord {
    pub activity: ActivityView,
    pub(crate) custom_fields: CustomFieldMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityData {
    pub(crate) title: String,
    pub(crate) activity_type_id: Uuid,
    pub(crate) status: ActivityStatus,
    pub(crate) priority: ActivityPriority,
    pub(crate) due_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) owner_id: Option<UserId>,
    pub record: Option<RecordRef>,
    pub(crate) links: Vec<RecordRef>,
    pub(crate) duration_minutes: Option<u32>,
    pub(crate) description: Option<String>,
    pub(crate) location: Option<ActivityLocation>,
    pub(crate) reminder_minutes_before: Option<u32>,
    pub(crate) occurrence: Option<OccurrenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkedDealValue {
    pub(crate) deal_id: Uuid,
    pub(crate) title: String,
    pub(crate) amount: Option<Decimal>,
    pub(crate) currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateActivityTypeCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateActivityTypeCommand {
    pub(crate) name: Option<String>,
    pub(crate) icon: Option<Option<String>>,
    pub(crate) color: Option<Option<String>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

pub use aphel_sdk::tasks::{ActivityLocationInput, CreateActivityCommand, RecordLinkInput};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpdateActivityCommand {
    pub title: Option<String>,
    pub activity_type_id: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<Option<String>>,
    pub completed_at: Option<Option<String>>,
    pub owner_id: Option<Option<String>>,
    pub record_type: Option<Option<String>>,
    pub record_id: Option<Option<String>>,
    pub links: Option<Vec<RecordLinkInput>>,
    pub duration_minutes: Option<Option<i64>>,
    pub description: Option<Option<String>>,
    pub location: Option<Option<ActivityLocationInput>>,
    pub custom_fields: Map<String, Value>,
    pub reminder_minutes_before: Option<Option<i64>>,
    pub recurrence: Option<Option<RecurrenceRuleInput>>,
    pub apply_to: Option<ApplyTo>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivityScheduleInput {
    pub reminder_minutes_before: Option<Option<i64>>,
    pub recurrence: Option<RecurrenceRuleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CalendarQuery {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) owner_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) activity_type_id: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum InboxSort {
    #[default]
    Due,
    Priority,
    Value,
}

impl InboxSort {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "due" => Some(Self::Due),
            "priority" => Some(Self::Priority),
            "value" => Some(Self::Value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct InboxQuery {
    pub(crate) owner: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) sort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListActivitiesQuery {
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub my_open: bool,
    pub query: RecordQuery,
}

pub(crate) trait CreateActivityFields {
    fn provided_keys(&self) -> Vec<&'static str>;
}

impl CreateActivityFields for CreateActivityCommand {
    fn provided_keys(&self) -> Vec<&'static str> {
        let mut keys = vec!["title"];
        provided_key(
            &mut keys,
            "activity_type_id",
            !self.activity_type_id.trim().is_empty(),
        );
        provided_key(&mut keys, "status", self.status.is_some());
        provided_key(&mut keys, "priority", self.priority.is_some());
        provided_key(&mut keys, "due_at", self.due_at.is_some());
        provided_key(&mut keys, "completed_at", self.completed_at.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        provided_key(&mut keys, "record_type", self.record_type.is_some());
        provided_key(&mut keys, "record_id", self.record_id.is_some());
        provided_key(
            &mut keys,
            "duration_minutes",
            self.duration_minutes.is_some(),
        );
        provided_key(&mut keys, "description", self.description.is_some());
        provide_location_keys(&mut keys, self.location.is_some());
        keys
    }
}

impl UpdateActivityCommand {
    pub(crate) fn provided_keys(&self) -> Vec<&'static str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "title", self.title.is_some());
        provided_key(
            &mut keys,
            "activity_type_id",
            self.activity_type_id.is_some(),
        );
        provided_key(&mut keys, "status", self.status.is_some());
        provided_key(&mut keys, "priority", self.priority.is_some());
        provided_key(&mut keys, "due_at", self.due_at.is_some());
        provided_key(&mut keys, "completed_at", self.completed_at.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        provided_key(&mut keys, "record_type", self.record_type.is_some());
        provided_key(&mut keys, "record_id", self.record_id.is_some());
        provided_key(
            &mut keys,
            "duration_minutes",
            self.duration_minutes.is_some(),
        );
        provided_key(&mut keys, "description", self.description.is_some());
        provide_location_keys(&mut keys, self.location.is_some());
        keys
    }
}

pub(crate) const LOCATION_FIELD_KEYS: [&str; 4] = [
    "location_latitude",
    "location_longitude",
    "location_accuracy_m",
    "location_captured_at",
];

fn provide_location_keys(keys: &mut Vec<&'static str>, provided: bool) {
    for key in LOCATION_FIELD_KEYS {
        provided_key(keys, key, provided);
    }
}

pub(crate) const ACTIVITY_CREATED: &str = "activity.created";
pub(crate) const ACTIVITY_UPDATED: &str = "activity.updated";
pub(crate) const ACTIVITY_COMPLETED: &str = "activity.completed";
pub(crate) const ACTIVITY_REOPENED: &str = "activity.reopened";
pub(crate) const ACTIVITY_DELETED: &str = "activity.deleted";

pub(crate) const ACTIVITY_TYPE_CREATED: &str = "activity_type.created";
pub(crate) const ACTIVITY_TYPE_UPDATED: &str = "activity_type.updated";
pub(crate) const ACTIVITY_TYPE_REORDERED: &str = "activity_type.reordered";
pub(crate) const ACTIVITY_TYPE_ACTIVATED: &str = "activity_type.activated";
pub(crate) const ACTIVITY_TYPE_DEACTIVATED: &str = "activity_type.deactivated";

pub fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn optional_instant(value: Option<DateTime<Utc>>) -> Value {
    value.map(instant).map(Value::String).unwrap_or(Value::Null)
}

fn record_json(record: &RecordRef) -> Value {
    json!({
        "record_type": record.record_type.as_str(),
        "record_id": record.record_id.to_string(),
    })
}

fn location_json(location: &ActivityLocation) -> Value {
    json!({
        "latitude": location.latitude.to_string(),
        "longitude": location.longitude.to_string(),
        "accuracy_m": location.accuracy_m,
        "captured_at": instant(location.captured_at),
    })
}

pub(crate) fn activity_payload(
    data: &ActivityData,
    activity_type_key: &str,
    occurred_at: DateTime<Utc>,
    changes: Option<&[FieldChange]>,
) -> Value {
    let mut payload = json!({
        "title": data.title,
        "status": data.status.as_str(),
        "priority": data.priority.as_str(),
        "activity_type_id": data.activity_type_id.to_string(),
        "activity_type_key": activity_type_key,
        "owner_id": data.owner_id.as_ref().map(|owner| owner.as_str()),
        "record_type": data.record.as_ref().map(|record| record.record_type.as_str()),
        "record_id": data.record.as_ref().map(|record| record.record_id.to_string()),
        "links": data.links.iter().map(record_json).collect::<Vec<_>>(),
        "due_at": optional_instant(data.due_at),
        "completed_at": optional_instant(data.completed_at),
        "recurrence_id": data.occurrence.map(|occurrence| occurrence.series_id.to_string()),
        "location": data.location.as_ref().map(location_json),
        "occurred_at": instant(occurred_at),
    });
    if let (Some(changes), Some(object)) = (changes, payload.as_object_mut()) {
        object.insert(
            "changes".to_owned(),
            changes
                .iter()
                .map(
                    |change| json!({ "field": change.field, "old": change.old, "new": change.new }),
                )
                .collect(),
        );
    }
    payload
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivitySnapshot {
    pub(crate) title: String,
    pub(crate) activity_type_id: String,
    pub(crate) status: &'static str,
    pub(crate) priority: &'static str,
    pub(crate) due_at: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) links: Vec<String>,
    pub(crate) duration_minutes: Option<u32>,
    pub(crate) description: Option<String>,
    pub(crate) location_latitude: Option<String>,
    pub(crate) location_longitude: Option<String>,
    pub(crate) location_accuracy_m: Option<u32>,
    pub(crate) location_captured_at: Option<String>,
    pub(crate) reminder_minutes_before: Option<u32>,
}

impl From<&ActivityData> for ActivitySnapshot {
    fn from(data: &ActivityData) -> Self {
        Self {
            title: data.title.clone(),
            activity_type_id: data.activity_type_id.to_string(),
            status: data.status.as_str(),
            priority: data.priority.as_str(),
            due_at: data.due_at.map(instant),
            completed_at: data.completed_at.map(instant),
            owner_id: data
                .owner_id
                .as_ref()
                .map(|owner| owner.as_str().to_owned()),
            record_type: data
                .record
                .as_ref()
                .map(|record| record.record_type.as_str().to_owned()),
            record_id: data
                .record
                .as_ref()
                .map(|record| record.record_id.to_string()),
            links: data
                .links
                .iter()
                .map(|link| format!("{}:{}", link.record_type, link.record_id))
                .collect(),
            duration_minutes: data.duration_minutes,
            description: data.description.clone(),
            location_latitude: data.location.map(|location| location.latitude.to_string()),
            location_longitude: data.location.map(|location| location.longitude.to_string()),
            location_accuracy_m: data.location.and_then(|location| location.accuracy_m),
            location_captured_at: data.location.map(|location| instant(location.captured_at)),
            reminder_minutes_before: data.reminder_minutes_before,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CalendarListing {
    pub(crate) items: Vec<ActivityRecord>,
    pub(crate) total: u64,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InboxGroupKey {
    Overdue,
    Today,
    Upcoming,
    NoDue,
}

impl InboxGroupKey {
    pub(crate) const ALL: [Self; 4] = [Self::Overdue, Self::Today, Self::Upcoming, Self::NoDue];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Overdue => "overdue",
            Self::Today => "today",
            Self::Upcoming => "upcoming",
            Self::NoDue => "no_due",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboxItem {
    pub(crate) record: ActivityRecord,
    pub(crate) deal: Option<LinkedDealValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboxGroup {
    pub(crate) key: InboxGroupKey,
    pub(crate) items: Vec<InboxItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboxListing {
    pub(crate) timezone: String,
    pub(crate) groups: Vec<InboxGroup>,
    pub(crate) total: u64,
    pub(crate) truncated: bool,
    pub(crate) deal_amounts_visible: bool,
}
