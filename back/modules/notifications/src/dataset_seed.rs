use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, push_record_tuples};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::QueryBuilder;

pub(crate) struct NotificationsSeed;

#[async_trait]
impl DatasetSeed for NotificationsSeed {
    fn order(&self) -> i32 {
        1000
    }

    fn entities(&self) -> Vec<RecordType> {
        Vec::new()
    }

    async fn seed(&self, _ctx: &Ctx<'_>, _session: &mut DatasetSession) -> Result<(), AppError> {
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM notifications")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let mut delete = QueryBuilder::new("DELETE FROM notifications WHERE ");
        push_record_tuples(&mut delete, records);
        delete.build().execute(&mut **tx).await?;
        Ok(true)
    }
}
