use aphel_kernel::database::transaction::MySqlTx;
use aphel_sdk::{
    AppError, Ctx,
    leads::LeadMergeHistory,
    shared::{RecordRef, RecordType},
};
use uuid::Uuid;
pub(crate) struct History;
#[async_trait::async_trait]
impl LeadMergeHistory for History {
    async fn stage(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        entity: &RecordType,
        id: Uuid,
    ) -> Result<Option<Option<String>>, AppError> {
        crate::store::LeadRecords::of(ctx)
            .current_stage_in(tx, entity, id)
            .await
    }
    async fn changed(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record: &RecordRef,
        before: Option<String>,
        after: Option<String>,
    ) -> Result<(), AppError> {
        crate::service::record_stage_history(ctx, tx, record, before, after).await
    }
}
