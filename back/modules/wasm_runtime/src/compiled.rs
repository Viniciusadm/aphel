use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

pub async fn find(
    pool: &Db,
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
    pool: &Db,
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

pub async fn prune(pool: &Db, keep: &[String]) -> Result<(), AppError> {
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
