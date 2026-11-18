use aphel_sdk::platform::database::Db;
use aphel_sdk::{
    AppError,
    email::{EmailEngagementProjection, EmailEngagementReader},
    shared::RecordRef,
};
pub(crate) struct Projection;
#[async_trait::async_trait]
impl EmailEngagementProjection for Projection {
    fn record_type(&self) -> &'static str {
        aphel_sdk::people::RECORD_TYPE
    }
    async fn refresh(
        &self,
        pool: &Db,
        record: &RecordRef,
        reader: &dyn EmailEngagementReader,
    ) -> Result<(), AppError> {
        if record.record_type.as_str() != self.record_type() {
            return Ok(());
        }
        let mut tx = aphel_sdk::platform::database::receipt::begin(pool).await?;
        if crate::store::lock_person(&mut tx, record.record_id).await? {
            let snapshot = reader.snapshot(&mut tx, record).await?;
            crate::store::set_engagement(
                &mut tx,
                record.record_id,
                crate::store::Engagement {
                    last_contacted_at: snapshot.last_contacted_at.map(|value| value.naive_utc()),
                    last_email_received_at: snapshot
                        .last_email_received_at
                        .map(|value| value.naive_utc()),
                    last_email_sent_at: snapshot.last_email_sent_at.map(|value| value.naive_utc()),
                    interaction_count: snapshot.interaction_count,
                    last_contacted_by: snapshot.last_contacted_by.map(|user| user.into_inner()),
                },
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
