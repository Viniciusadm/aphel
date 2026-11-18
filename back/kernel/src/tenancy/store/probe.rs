use crate::database::Db;

pub(crate) async fn current_database(pool: &Db) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, Option<String>>("SELECT DATABASE()")
        .fetch_one(pool)
        .await
}

pub(crate) async fn record_event_count(pool: &Db, record_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM domain_events WHERE record_id = ?")
        .bind(record_id)
        .fetch_one(pool)
        .await
}
