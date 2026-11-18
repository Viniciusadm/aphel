use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySqlConnection, MySqlPool};

#[derive(FromRow)]
pub(crate) struct RecordRedirect {
    pub id: u64,
    pub record_type: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow)]
pub(crate) struct PathRedirect {
    pub id: u64,
    pub source_path: String,
    pub target: String,
    pub permanent: bool,
    pub created_at: DateTime<Utc>,
}

pub(crate) async fn list_record_redirects(
    pool: &MySqlPool,
    limit: i64,
) -> Result<Vec<RecordRedirect>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, record_type, field, old_value, new_value, created_at FROM site_redirects \
         ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub(crate) async fn delete_record_redirect(
    conn: &mut MySqlConnection,
    id: u64,
) -> Result<Option<RecordRedirect>, sqlx::Error> {
    let found: Option<RecordRedirect> = sqlx::query_as(
        "SELECT id, record_type, field, old_value, new_value, created_at FROM site_redirects \
         WHERE id = ? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    if found.is_some() {
        sqlx::query("DELETE FROM site_redirects WHERE id = ?")
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(found)
}

pub(crate) async fn list_path_redirects(
    pool: &MySqlPool,
    limit: i64,
) -> Result<Vec<PathRedirect>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, source_path, target, permanent, created_at FROM site_path_redirects \
         ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub(crate) async fn lock_path_redirect(
    conn: &mut MySqlConnection,
    source_path: &str,
) -> Result<bool, sqlx::Error> {
    let found: Option<u64> =
        sqlx::query_scalar("SELECT id FROM site_path_redirects WHERE source_path = ? FOR UPDATE")
            .bind(source_path)
            .fetch_optional(conn)
            .await?;
    Ok(found.is_some())
}

pub(crate) async fn insert_path_redirect(
    conn: &mut MySqlConnection,
    source_path: &str,
    target: &str,
    permanent: bool,
) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query(
        "INSERT INTO site_path_redirects (source_path, target, permanent) VALUES (?, ?, ?)",
    )
    .bind(source_path)
    .bind(target)
    .bind(permanent)
    .execute(conn)
    .await?
    .last_insert_id())
}

pub(crate) async fn delete_path_redirect(
    conn: &mut MySqlConnection,
    id: u64,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let found: Option<(String, String)> = sqlx::query_as(
        "SELECT source_path, target FROM site_path_redirects WHERE id = ? FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    if found.is_some() {
        sqlx::query("DELETE FROM site_path_redirects WHERE id = ?")
            .bind(id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(found)
}

pub(crate) async fn save_record_change(
    pool: &MySqlPool,
    record_type: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
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
    tx.commit().await
}

pub(crate) async fn record_target(
    pool: &MySqlPool,
    record_type: &str,
    field: &str,
    value: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT new_value FROM site_redirects WHERE record_type = ? AND field = ? AND old_value = ?",
    )
    .bind(record_type)
    .bind(field)
    .bind(value)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn path_target(
    pool: &MySqlPool,
    source_path: &str,
) -> Result<Option<(String, bool)>, sqlx::Error> {
    sqlx::query_as("SELECT target, permanent FROM site_path_redirects WHERE source_path = ?")
        .bind(source_path)
        .fetch_optional(pool)
        .await
}
