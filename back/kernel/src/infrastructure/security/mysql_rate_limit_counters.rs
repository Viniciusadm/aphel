use chrono::NaiveDateTime;
use sqlx::{MySqlPool, Row};

pub(crate) async fn record_hit(
    pool: &MySqlPool,
    bucket: &str,
    window_start: i64,
    expires_at: NaiveDateTime,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO rate_limit_counters (bucket, window_start, hits, expires_at) VALUES (?, ?, 1, ?) \
         ON DUPLICATE KEY UPDATE hits = hits + 1",
    )
    .bind(bucket)
    .bind(window_start)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn window_hits(
    pool: &MySqlPool,
    bucket: &str,
    current_start: i64,
    previous_start: i64,
) -> Result<Vec<(i64, u32)>, sqlx::Error> {
    sqlx::query(
        "SELECT window_start, hits FROM rate_limit_counters WHERE bucket = ? AND window_start IN (?, ?)",
    )
    .bind(bucket)
    .bind(current_start)
    .bind(previous_start)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| Ok((row.try_get("window_start")?, row.try_get("hits")?)))
    .collect()
}

pub(crate) async fn undo_hit(
    pool: &MySqlPool,
    bucket: &str,
    window_start: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE rate_limit_counters SET hits = hits - 1 WHERE bucket = ? AND window_start = ? AND hits > 0",
    )
    .bind(bucket)
    .bind(window_start)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete_expired(
    pool: &MySqlPool,
    now: NaiveDateTime,
    batch: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM rate_limit_counters WHERE expires_at < ? LIMIT ?")
        .bind(now)
        .bind(batch)
        .execute(pool)
        .await?;
    Ok(())
}
