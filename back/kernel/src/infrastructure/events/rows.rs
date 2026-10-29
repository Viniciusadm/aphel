use serde_json::Value;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use uuid::Uuid;

use crate::application::shared::{Actor, EventCausation, EventOrigin, RecordRef, RecordType};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

pub(crate) fn causation_column(row: &MySqlRow) -> Result<Option<EventCausation>, AppError> {
    let value: Option<Json<Value>> = row.try_get("causation")?;
    Ok(value.and_then(|Json(value)| serde_json::from_value(value).ok()))
}

pub(crate) fn origin_columns(row: &MySqlRow) -> Option<EventOrigin> {
    let kind: String = row.try_get::<Option<String>, _>("origin_type").ok()??;
    let id: Option<String> = row.try_get("origin_id").ok().flatten();
    Some(EventOrigin::new(kind, id))
}

pub(crate) fn actor_from_columns(kind: String, id: Option<String>) -> Result<Actor, AppError> {
    Actor::from_parts(&kind, id).map_err(|_| mapper::invalid_column("actor_type", &kind))
}

pub(crate) fn record_from_columns(
    record_type: Option<String>,
    record_id: Option<String>,
) -> Result<Option<RecordRef>, AppError> {
    let (Some(record_type), Some(record_id)) = (record_type, record_id) else {
        return Ok(None);
    };
    let parsed_type = RecordType::parse(record_type.as_str())
        .map_err(|_| mapper::invalid_column("record_type", &record_type))?;
    Ok(Some(RecordRef::new(
        parsed_type,
        uuid_column("record_id", record_id)?,
    )))
}

pub fn uuid_column(column: &str, value: String) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}
