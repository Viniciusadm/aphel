use sqlx::MySqlPool;

use aphel_kernel::errors::AppError;

pub async fn find(
    pool: &MySqlPool,
    backend_hash: &str,
    engine_key: &str,
) -> Result<Option<Vec<u8>>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT artifact FROM plugins_compiled WHERE backend_hash = ? AND engine_key = ?",
    )
    .bind(backend_hash)
    .bind(engine_key)
    .fetch_optional(pool)
    .await?)
}

pub async fn save(
    pool: &MySqlPool,
    backend_hash: &str,
    engine_key: &str,
    artifact: &[u8],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO plugins_compiled (backend_hash, engine_key, artifact) VALUES (?, ?, ?) \
         ON DUPLICATE KEY UPDATE artifact = VALUES(artifact)",
    )
    .bind(backend_hash)
    .bind(engine_key)
    .bind(artifact)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn prune(pool: &MySqlPool, keep: &[String]) -> Result<(), AppError> {
    let stored: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT backend_hash FROM plugins_compiled")
            .fetch_all(pool)
            .await?;
    for hash in stored.iter().filter(|hash| !keep.contains(hash)) {
        sqlx::query("DELETE FROM plugins_compiled WHERE backend_hash = ?")
            .bind(hash)
            .execute(pool)
            .await?;
    }
    Ok(())
}
