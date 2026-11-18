//! `site_redirects` (created when a record's address field changes) and
//! `site_path_redirects` (added by hand).

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

/// Newest record redirects first.
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

/// Locks and deletes a record redirect, returning what it was; `None` when unknown.
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

/// Newest hand-made redirects first.
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

/// Locks the redirect of `source_path` (or the gap where it would be) and tells whether it exists.
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

/// Inserts a hand-made redirect and returns its id.
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

/// Locks and deletes a hand-made redirect, returning `(source_path, target)`; `None` when unknown.
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
