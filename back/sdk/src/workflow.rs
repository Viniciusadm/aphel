use crate::fields::EntityTraits;
use crate::shared::RecordType;
use crate::{AppError, Ctx};
pub use aphel_contract_workflow::*;
use aphel_kernel::database::transaction::MySqlTx;
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::collections::HashMap;
use uuid::Uuid;
pub trait WorkflowTraits {
    fn staged(self, mapping: StagedMapping) -> Self;
    fn stage_mapping(&self) -> Option<StagedMapping>;
}
impl WorkflowTraits for EntityTraits {
    fn staged(self, mapping: StagedMapping) -> Self {
        self.with_trait_data(
            STAGED.key(),
            serde_json::to_value(mapping).expect("stage mapping"),
        )
    }
    fn stage_mapping(&self) -> Option<StagedMapping> {
        let value = self.trait_data(STAGED.key())?;
        if value == &serde_json::Value::Bool(true) {
            return Some(StagedMapping::default());
        }
        serde_json::from_value(value.clone()).ok()
    }
}
#[async_trait]
pub trait StagedRecordSource: Send + Sync {
    fn record_type(&self) -> RecordType;
    async fn stage_counts(
        &self,
        ctx: &Ctx<'_>,
        pipeline_id: Uuid,
    ) -> Result<HashMap<Uuid, u64>, AppError>;
    async fn usage(&self, pool: &MySqlPool, target: UsageTarget) -> Result<StageUsage, AppError>;
    async fn location(&self, pool: &MySqlPool, id: Uuid)
    -> Result<Option<StageLocation>, AppError>;
    async fn move_records(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        movement: &StageMove,
    ) -> Result<u64, AppError>;
}

pub trait WorkflowRecordTypes {
    const PIPELINE: RecordType;
    const PIPELINE_STAGE: RecordType;
    const LOST_REASON: RecordType;
}
impl WorkflowRecordTypes for RecordType {
    const PIPELINE: RecordType = RecordType::from_static("workflow.pipeline");
    const PIPELINE_STAGE: RecordType = RecordType::from_static("workflow.stage");
    const LOST_REASON: RecordType = RecordType::from_static("workflow.lost_reason");
}
