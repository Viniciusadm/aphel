use uuid::Uuid;

use crate::application::shared::RecordRef;
use crate::database::transaction::Tx;
use crate::errors::AppError;

pub(crate) async fn delete_extension_row(
    tx: &mut Tx,
    table: &str,
    record_column: &str,
    record_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM `{table}` WHERE `{record_column}` = ?"
    )))
    .bind(record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn clear_record_references(
    tx: &mut Tx,
    record: &RecordRef,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE v FROM custom_field_values v JOIN field_definitions f ON f.id = v.field_id \
         WHERE f.field_type = 'record' \
         AND JSON_UNQUOTE(JSON_EXTRACT(f.config, '$.target_record_type')) = ? \
         AND v.value_text = ?",
    )
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn delete_record_rows(
    tx: &mut Tx,
    record: &RecordRef,
) -> Result<Vec<Uuid>, AppError> {
    let kind = record.record_type.as_str();
    let id = record.record_id.to_string();
    let attached: Vec<String> = sqlx::query_scalar(
        "SELECT file_id FROM attachments WHERE record_type = ? AND record_id = ?",
    )
    .bind(kind)
    .bind(&id)
    .fetch_all(&mut **tx)
    .await?;
    for table in ["attachments", "record_shares"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ? AND record_id = ?"
        )))
        .bind(kind)
        .bind(&id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(attached
        .iter()
        .filter_map(|file_id| Uuid::parse_str(file_id).ok())
        .collect())
}
