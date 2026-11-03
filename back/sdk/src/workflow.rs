pub mod dto;
pub use dto::*;
pub mod rotting;
use aphel_kernel::application::access::Authorizer;
pub use aphel_kernel::application::shared::parse_uuid as parse_id;
use aphel_kernel::domain::access::{AccessScope, Permission};
pub const PIPELINE_NOT_FOUND: &str = "pipeline_not_found";
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

#[async_trait]
pub trait WorkflowCatalog: Send + Sync {
    async fn pipelines(&self, pool: &MySqlPool) -> Result<Vec<Pipeline>, AppError>;
    async fn pipeline(&self, pool: &MySqlPool, id: Uuid) -> Result<Option<Pipeline>, AppError>;
    async fn default_pipeline(&self, pool: &MySqlPool) -> Result<Option<Pipeline>, AppError>;
    async fn stages(
        &self,
        pool: &MySqlPool,
        pipeline: Uuid,
    ) -> Result<Vec<PipelineStage>, AppError>;
    async fn stages_for_pipelines(
        &self,
        pool: &MySqlPool,
        ids: &[Uuid],
    ) -> Result<Vec<PipelineStage>, AppError>;
    async fn stage(&self, pool: &MySqlPool, id: Uuid) -> Result<Option<PipelineStage>, AppError>;
    async fn lost_reasons(&self, pool: &MySqlPool) -> Result<Vec<LostReason>, AppError>;
    async fn lost_reason(&self, pool: &MySqlPool, id: Uuid)
    -> Result<Option<LostReason>, AppError>;
}

#[derive(Clone)]
pub struct WorkflowReader {
    pool: MySqlPool,
    catalog: std::sync::Arc<dyn WorkflowCatalog>,
}
impl WorkflowReader {
    pub fn new(
        pool: MySqlPool,
        services: &aphel_kernel::modules::Services,
    ) -> Result<Self, AppError> {
        Ok(Self {
            pool,
            catalog: services
                .get::<dyn WorkflowCatalog>()
                .ok_or_else(|| AppError::Configuration("workflow is unavailable".into()))?,
        })
    }
    pub fn of(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        Self::new(ctx.pool().clone(), ctx.services())
    }
    pub async fn list_pipelines(&self) -> Result<Vec<Pipeline>, AppError> {
        self.catalog.pipelines(&self.pool).await
    }
    pub async fn find_pipeline(&self, id: Uuid) -> Result<Option<Pipeline>, AppError> {
        self.catalog.pipeline(&self.pool, id).await
    }
    pub async fn default_pipeline(&self) -> Result<Option<Pipeline>, AppError> {
        self.catalog.default_pipeline(&self.pool).await
    }
    pub async fn list_stages(&self, id: Uuid) -> Result<Vec<PipelineStage>, AppError> {
        self.catalog.stages(&self.pool, id).await
    }
    pub async fn stages_for_pipelines(&self, ids: &[Uuid]) -> Result<Vec<PipelineStage>, AppError> {
        self.catalog.stages_for_pipelines(&self.pool, ids).await
    }
    pub async fn find_stage(&self, id: Uuid) -> Result<Option<PipelineStage>, AppError> {
        self.catalog.stage(&self.pool, id).await
    }
    pub async fn list_lost_reasons(&self) -> Result<Vec<LostReason>, AppError> {
        self.catalog.lost_reasons(&self.pool).await
    }
    pub async fn find_lost_reason(&self, id: Uuid) -> Result<Option<LostReason>, AppError> {
        self.catalog.lost_reason(&self.pool, id).await
    }
}

pub fn sees_every_pipeline(scope: &AccessScope) -> bool {
    scope.require(&Permission::SETTINGS_MANAGE).is_ok()
}
pub fn pipeline_visible(scope: &AccessScope, pipeline: &Pipeline) -> bool {
    sees_every_pipeline(scope)
        || pipeline.visible_to_teams(scope.team_ids().iter().map(|team| team.as_str()))
}
pub fn pipeline_restriction(scope: &AccessScope, workflow: &[Pipeline]) -> Option<Vec<Uuid>> {
    if sees_every_pipeline(scope) || !workflow.iter().any(Pipeline::is_restricted) {
        return None;
    }
    Some(
        workflow
            .iter()
            .filter(|pipeline| pipeline_visible(scope, pipeline))
            .map(|pipeline| pipeline.id)
            .collect(),
    )
}
