use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::service::subject::SubjectMatcher;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::{
    FileReferenceColumn, FileReferenceSource, UserReference, declared_file_columns,
    unreferenced_file_condition,
};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::infrastructure::repositories::mapper;

pub(crate) const PRIVACY_FILE_COLUMNS: [FileReferenceColumn; 1] =
    [FileReferenceColumn::new("privacy_requests", "file_id")];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PrivacyFileReferences;

impl FileReferenceSource for PrivacyFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        PRIVACY_FILE_COLUMNS.to_vec()
    }
}

pub(crate) async fn existing_columns(
    pool: &MySqlPool,
    declared: Vec<(&'static str, &'static str)>,
) -> Result<Vec<(&'static str, &'static str)>, AppError> {
    if declared.is_empty() {
        return Ok(declared);
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN (",
    );
    let mut tables = builder.separated(", ");
    for (table, _) in &declared {
        tables.push_bind(*table);
    }
    builder.push(")");
    let found: Vec<(String, String)> = builder.build_query_as().fetch_all(pool).await?;
    Ok(declared
        .into_iter()
        .filter(|(table, column)| {
            found
                .iter()
                .any(|(found_table, found_column)| found_table == table && found_column == column)
        })
        .collect())
}

pub async fn file_reference_columns(
    pool: &MySqlPool,
) -> Result<Vec<FileReferenceColumn>, AppError> {
    let declared = declared_file_columns(aphel_sdk::platform::modules::app())
        .into_iter()
        .map(|reference| (reference.table, reference.column))
        .collect();
    Ok(existing_columns(pool, declared)
        .await?
        .into_iter()
        .map(|(table, column)| FileReferenceColumn::new(table, column))
        .collect())
}

pub fn unreferenced_file_predicate(columns: &[FileReferenceColumn]) -> String {
    unreferenced_file_condition(columns, "f")
}

pub(crate) fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

pub(crate) fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    Ok(value.and_then(|value| Uuid::parse_str(&value).ok()))
}

pub(crate) fn instant(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    Ok(mapper::utc(
        row.try_get::<chrono::NaiveDateTime, _>(column)?,
    ))
}

pub(crate) fn optional_instant(
    row: &MySqlRow,
    column: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row
        .try_get::<Option<chrono::NaiveDateTime>, _>(column)?
        .map(mapper::utc))
}

pub(crate) fn user_reference(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    Ok(match id {
        Some(id) => Some(UserReference {
            id: UserId::new(id)?,
            display_name: name.unwrap_or_default(),
        }),
        None => None,
    })
}

pub(crate) fn contains_pattern(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len() + 2);
    escaped.push('%');
    for character in term.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.push('%');
    escaped
}

pub(crate) fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

pub(crate) fn push_refs(
    builder: &mut QueryBuilder<MySql>,
    type_column: &str,
    id_column: &str,
    records: &[RecordRef],
) {
    builder.push("(");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            builder.push(" OR ");
        }
        builder
            .push(format!("({type_column} = "))
            .push_bind(record.record_type.as_str().to_owned())
            .push(format!(" AND {id_column} = "))
            .push_bind(record.record_id.to_string())
            .push(")");
    }
    builder.push(")");
}

pub(crate) fn phone_digits(matcher: &SubjectMatcher) -> Vec<String> {
    let mut digits: Vec<String> = matcher.digits.iter().cloned().collect();
    if let Some(phone) = &matcher.phone {
        let international = phone.trim_start_matches('+').to_owned();
        if !digits.contains(&international) {
            digits.push(international);
        }
    }
    digits
}
