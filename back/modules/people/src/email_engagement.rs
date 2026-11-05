use aphel_sdk::{
    AppError,
    email::{EmailEngagementProjection, EmailEngagementReader},
    shared::RecordRef,
};
use sqlx::MySqlPool;
pub(crate) struct Projection;
#[async_trait::async_trait]
impl EmailEngagementProjection for Projection {
    fn record_type(&self) -> &'static str {
        aphel_sdk::people::RECORD_TYPE
    }
    async fn refresh(
        &self,
        pool: &MySqlPool,
        record: &RecordRef,
        reader: &dyn EmailEngagementReader,
    ) -> Result<(), AppError> {
        if record.record_type.as_str() != self.record_type() {
            return Ok(());
        }
        let mut tx = aphel_sdk::platform::database::receipt::begin(pool).await?;
        let found: Option<String> =
            sqlx::query_scalar("SELECT id FROM people WHERE id = ? FOR UPDATE")
                .bind(record.record_id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_some() {
            let snapshot = reader.snapshot(&mut tx, record).await?;
            sqlx::query("UPDATE people SET last_contacted_at = ?, last_email_received_at = ?, last_email_sent_at = ?, interaction_count = ?, last_contacted_by = ?, updated_at = updated_at WHERE id = ?")
                .bind(snapshot.last_contacted_at.map(|value| value.naive_utc()))
                .bind(snapshot.last_email_received_at.map(|value| value.naive_utc()))
                .bind(snapshot.last_email_sent_at.map(|value| value.naive_utc()))
                .bind(snapshot.interaction_count)
                .bind(snapshot.last_contacted_by.map(|user| user.into_inner()))
                .bind(record.record_id.to_string()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
