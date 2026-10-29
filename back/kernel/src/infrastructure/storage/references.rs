use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::application::files::declared_file_columns;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;

async fn reference_columns(tx: &mut MySqlTx) -> Result<Vec<(String, String)>, AppError> {
    let declared = declared_file_columns(crate::modules::app());
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN (",
    );
    let mut tables = builder.separated(", ");
    for reference in &declared {
        tables.push_bind(reference.table);
    }
    builder.push(")");
    let existing: Vec<(String, String)> = builder.build_query_as().fetch_all(&mut **tx).await?;
    Ok(declared
        .into_iter()
        .filter(|reference| {
            existing
                .iter()
                .any(|(table, column)| table == reference.table && column == reference.column)
        })
        .map(|reference| (reference.table.to_owned(), reference.column.to_owned()))
        .collect())
}

async fn is_referenced(
    tx: &mut MySqlTx,
    columns: &[(String, String)],
    file_id: &str,
) -> Result<bool, AppError> {
    for (table, column) in columns {
        let found: Option<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT 1 FROM `{table}` WHERE `{column}` = ? LIMIT 1"
        )))
        .bind(file_id)
        .fetch_optional(&mut **tx)
        .await?;
        if found.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn delete_unreferenced_files(
    tx: &mut MySqlTx,
    candidates: &[Uuid],
) -> Result<Vec<String>, AppError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let columns = reference_columns(tx).await?;
    let mut orphans = Vec::new();
    for file_id in candidates {
        let id = file_id.to_string();
        if !is_referenced(tx, &columns, &id).await? {
            orphans.push(id);
        }
    }
    if orphans.is_empty() {
        return Ok(Vec::new());
    }
    let mut select = QueryBuilder::<MySql>::new("SELECT storage_key FROM files WHERE id IN (");
    let mut ids = select.separated(", ");
    for id in &orphans {
        ids.push_bind(id);
    }
    select.push(")");
    let keys: Vec<String> = select.build_query_scalar().fetch_all(&mut **tx).await?;
    let mut delete = QueryBuilder::<MySql>::new("DELETE FROM files WHERE id IN (");
    let mut ids = delete.separated(", ");
    for id in &orphans {
        ids.push_bind(id);
    }
    delete.push(")");
    delete.build().execute(&mut **tx).await?;
    Ok(keys)
}
