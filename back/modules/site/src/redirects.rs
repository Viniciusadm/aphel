use aphel_kernel::errors::AppError;
use sqlx::MySqlPool;

const MAX_VALUE_LENGTH: usize = 255;

pub(crate) async fn record_change(
    pool: &MySqlPool,
    record_type: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), AppError> {
    if old_value.is_empty()
        || old_value == new_value
        || old_value.len() > MAX_VALUE_LENGTH
        || new_value.len() > MAX_VALUE_LENGTH
    {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM site_redirects WHERE record_type = ? AND field = ? AND old_value = ?")
        .bind(record_type)
        .bind(field)
        .bind(new_value)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE site_redirects SET new_value = ? WHERE record_type = ? AND field = ? AND new_value = ?",
    )
    .bind(new_value)
    .bind(record_type)
    .bind(field)
    .bind(old_value)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO site_redirects (record_type, field, old_value, new_value) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE new_value = VALUES(new_value), created_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(record_type)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn lookup(
    pool: &MySqlPool,
    record_type: &str,
    field: &str,
    value: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT new_value FROM site_redirects WHERE record_type = ? AND field = ? AND old_value = ?",
    )
    .bind(record_type)
    .bind(field)
    .bind(value)
    .fetch_optional(pool)
    .await?)
}
