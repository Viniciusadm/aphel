use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::files::UserReference;
use crate::domain::UserId;
use crate::records::Address;

use crate::domain::{Email, NonEmptyString, UtcDateTime};
use crate::errors::AppError;

pub fn typed_id<T>(
    value: String,
    build: impl FnOnce(String) -> Result<T, crate::domain::DomainValidationError>,
) -> Result<T, AppError> {
    Ok(build(value)?)
}

pub(crate) fn non_empty(value: String, field: &'static str) -> Result<NonEmptyString, AppError> {
    Ok(NonEmptyString::new(value, field)?)
}

pub(crate) fn email(value: String) -> Result<Email, AppError> {
    Ok(Email::new(value)?)
}

pub(crate) fn datetime(value: NaiveDateTime) -> Result<UtcDateTime, AppError> {
    Ok(UtcDateTime::new_iso8601(value.to_string())?)
}

pub(crate) fn opt_datetime(value: Option<NaiveDateTime>) -> Result<Option<UtcDateTime>, AppError> {
    value.map(datetime).transpose()
}

pub fn utc(value: NaiveDateTime) -> DateTime<Utc> {
    value.and_utc()
}

pub fn naive(value: DateTime<Utc>) -> NaiveDateTime {
    value.naive_utc()
}

pub fn invalid_column(column: &str, value: &str) -> AppError {
    AppError::Persistence(format!("unexpected value {value:?} in column {column}"))
}

pub fn parse_uuid(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

pub const ADDRESS_COLUMNS: &str = "r.address_street, r.address_number, r.address_complement, \
     r.address_district, r.address_city, r.address_state, r.address_postal_code, r.address_country";

pub fn address_from_row(row: &MySqlRow) -> Result<Address, AppError> {
    Ok(Address {
        street: row.try_get("address_street")?,
        number: row.try_get("address_number")?,
        complement: row.try_get("address_complement")?,
        district: row.try_get("address_district")?,
        city: row.try_get("address_city")?,
        state: row.try_get("address_state")?,
        postal_code: row.try_get("address_postal_code")?,
        country: row.try_get("address_country")?,
    })
}

pub fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|value| Uuid::parse_str(&value).map_err(|_| invalid_column(column, &value)))
        .transpose()
}

pub fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|id| typed_id(id, UserId::new))
        .transpose()
}

pub fn user_reference(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    id.map(|id| {
        Ok(UserReference {
            id: typed_id(id, UserId::new)?,
            display_name: name.unwrap_or_default(),
        })
    })
    .transpose()
}

pub fn owner_reference(row: &MySqlRow) -> Result<Option<UserReference>, AppError> {
    let owner = optional_user(row, "owner_id")?;
    let name: Option<String> = row.try_get("owner_display_name")?;
    Ok(owner
        .zip(name)
        .map(|(id, display_name)| UserReference { id, display_name }))
}

pub fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row.try_get::<Option<NaiveDateTime>, _>(column)?.map(utc))
}

pub fn instant(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    Ok(utc(row.try_get::<NaiveDateTime, _>(column)?))
}

pub fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}
