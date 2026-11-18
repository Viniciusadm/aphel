use sqlx::{MySqlConnection, MySqlPool};

pub(crate) async fn queue_invalidation(
    conn: &mut MySqlConnection,
    tag: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO site_cache_invalidations (tag) VALUES (?)")
        .bind(tag)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn newest_invalidation(pool: &MySqlPool) -> Result<Option<u64>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(id) FROM site_cache_invalidations")
        .fetch_one(pool)
        .await
}

pub(crate) async fn invalidations_after(
    pool: &MySqlPool,
    last: u64,
    limit: i64,
) -> Result<Vec<(u64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, tag FROM site_cache_invalidations WHERE id > ? ORDER BY id LIMIT ?")
        .bind(last)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub(crate) async fn record_invalidations(
    pool: &MySqlPool,
    tags: &[String],
) -> Result<(), sqlx::Error> {
    for tag in tags {
        sqlx::query("INSERT INTO site_cache_invalidations (tag) VALUES (?)")
            .bind(tag)
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "DELETE FROM site_cache_invalidations WHERE created_at < NOW(6) - INTERVAL 1 DAY LIMIT 1000",
    )
    .execute(pool)
    .await?;
    Ok(())
}
