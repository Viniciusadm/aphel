use sqlx::MySqlConnection;

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
