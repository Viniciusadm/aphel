use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CallDirection {
    Inbound,
    #[default]
    Outbound,
}

impl CallDirection {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "inbound" => Some(Self::Inbound),
            "outbound" => Some(Self::Outbound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallOutcomeSummary {
    pub(crate) key: String,
    pub(crate) name: Option<String>,
    pub(crate) is_connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CallDetails {
    pub(crate) direction: CallDirection,
    pub(crate) outcome_key: String,
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) session_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityCallView {
    pub(crate) direction: CallDirection,
    pub(crate) outcome: CallOutcomeSummary,
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ActivityCallView {
    pub(crate) fn details(&self) -> CallDetails {
        CallDetails {
            direction: self.direction,
            outcome_key: self.outcome.key.clone(),
            duration_seconds: self.duration_seconds,
            phone_number: self.phone_number.clone(),
            recording_url: self.recording_url.clone(),
            provider: self.provider.clone(),
            provider_call_id: self.provider_call_id.clone(),
            session_id: self.session_id,
        }
    }
}

pub(crate) fn call_columns_sql(alias: &str) -> String {
    format!(
        "{alias}_call.direction AS call_direction, {alias}_call.outcome_key AS call_outcome_key, \
         {alias}_outcome.name AS call_outcome_name, {alias}_outcome.is_connected AS call_outcome_connected, \
         {alias}_call.duration_seconds AS call_duration_seconds, {alias}_call.phone_number AS call_phone_number, \
         {alias}_call.recording_url AS call_recording_url, {alias}_call.provider AS call_provider, \
         {alias}_call.provider_call_id AS call_provider_call_id, {alias}_call.session_id AS call_session_id, \
         {alias}_call.updated_at AS call_updated_at"
    )
}

pub(crate) fn call_from_row(row: &MySqlRow) -> Result<Option<ActivityCallView>, AppError> {
    let direction: Option<String> = row.try_get("call_direction")?;
    let Some(direction) = direction else {
        return Ok(None);
    };
    let outcome_key: String = row.try_get("call_outcome_key")?;
    let updated_at: NaiveDateTime = row.try_get("call_updated_at")?;
    let session_id: Option<String> = row.try_get("call_session_id")?;
    let connected: Option<bool> = row.try_get("call_outcome_connected")?;
    Ok(Some(ActivityCallView {
        direction: CallDirection::parse(&direction)
            .ok_or_else(|| mapper::invalid_column("direction", &direction))?,
        outcome: CallOutcomeSummary {
            key: outcome_key,
            name: row.try_get("call_outcome_name")?,
            is_connected: connected.unwrap_or(false),
        },
        duration_seconds: row.try_get("call_duration_seconds")?,
        phone_number: row.try_get("call_phone_number")?,
        recording_url: row.try_get("call_recording_url")?,
        provider: row.try_get("call_provider")?,
        provider_call_id: row.try_get("call_provider_call_id")?,
        session_id: session_id
            .map(|value| {
                Uuid::parse_str(&value).map_err(|_| mapper::invalid_column("session_id", &value))
            })
            .transpose()?,
        updated_at: mapper::utc(updated_at),
    }))
}
