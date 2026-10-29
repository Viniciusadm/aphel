use crate::application::shared::RecordType;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;

pub async fn purge_record_type_data(
    tx: &mut MySqlTx,
    record_type: &RecordType,
) -> Result<u64, AppError> {
    let mut removed = 0;
    for table in ["custom_field_values", "custom_field_multi_values"] {
        removed += sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ?"
        )))
        .bind(record_type.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
    }
    removed += sqlx::query(
        "DELETE l FROM record_relations l JOIN relationship_definitions d ON d.id = l.relationship_id \
         WHERE d.from_type = ? OR d.to_type = ?",
    )
    .bind(record_type.as_str())
    .bind(record_type.as_str())
    .execute(&mut **tx)
    .await?
    .rows_affected();
    Ok(removed)
}
