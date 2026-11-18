use serde_json::{Value, json};
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySql, QueryBuilder};

use crate::database::Db;
use crate::database::transaction::Tx;
use crate::errors::AppError;

pub(crate) async fn data_tables(
    pool: &Db,
    name_like: String,
    owned: Vec<&str>,
) -> Result<Vec<String>, AppError> {
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT TABLE_NAME FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' AND (TABLE_NAME LIKE ",
    );
    query.push_bind(name_like);
    if !owned.is_empty() {
        query.push(" OR TABLE_NAME IN (");
        let mut tables = query.separated(", ");
        for table in owned {
            tables.push_bind(table.to_owned());
        }
        query.push(")");
    }
    query.push(") ORDER BY TABLE_NAME");
    Ok(query.build_query_scalar().fetch_all(pool).await?)
}

pub(crate) async fn columns(pool: &Db, table: &str) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
    )
    .bind(table)
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn table_as_json(
    pool: &Db,
    table: &str,
    columns: &[String],
) -> Result<Value, AppError> {
    let object = columns
        .iter()
        .map(|column| format!("'{column}', `{column}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let rows: Option<Json<Value>> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT JSON_ARRAYAGG(JSON_OBJECT({object})) FROM `{table}`"
    )))
    .fetch_one(pool)
    .await?;
    Ok(rows.map(|rows| rows.0).unwrap_or_else(|| json!([])))
}

pub(crate) async fn delete_all(tx: &mut Tx, table: &str) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query(AssertSqlSafe(format!("DELETE FROM `{table}`")))
        .execute(&mut **tx)
        .await?
        .rows_affected())
}

pub(crate) async fn enabled_keys(pool: &Db) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT module_key FROM installed_modules WHERE enabled ORDER BY module_key",
    )
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn stored_settings(pool: &Db, module: &str) -> Result<Option<Value>, AppError> {
    let stored: Option<Json<Value>> =
        sqlx::query_scalar("SELECT settings FROM installed_modules WHERE module_key = ?")
            .bind(module)
            .fetch_optional(pool)
            .await?;
    Ok(stored.map(|json| json.0))
}
