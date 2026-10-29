use chrono::{DateTime, NaiveDateTime, Utc};
use uuid::Uuid;

use crate::application::shared::ensure_version;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::modules::manifest::is_identifier;

pub async fn lock_version(
    tx: &mut MySqlTx,
    table: &str,
    id: Uuid,
    expected: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    if expected.is_none() {
        return Ok(());
    }
    if !is_identifier(table) {
        return Err(AppError::Internal(format!("invalid table name {table}")));
    }
    let current: Option<NaiveDateTime> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT updated_at FROM `{table}` WHERE id = ? FOR UPDATE"
    )))
    .bind(id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    match current {
        Some(current) => ensure_version(expected, current.and_utc()),
        None => Ok(()),
    }
}
