use crate::store::PipelineStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::workflow::{LostReason, Pipeline, PipelineStage, WorkflowCatalog};
use sqlx::MySqlPool;
use uuid::Uuid;
pub(crate) struct NativeWorkflowCatalog;
#[async_trait::async_trait]
impl WorkflowCatalog for NativeWorkflowCatalog {
    async fn pipelines(&self, pool: &MySqlPool) -> Result<Vec<Pipeline>, AppError> {
        PipelineStore::new(pool.clone()).list_pipelines().await
    }
    async fn pipeline(&self, pool: &MySqlPool, id: Uuid) -> Result<Option<Pipeline>, AppError> {
        PipelineStore::new(pool.clone()).find_pipeline(id).await
    }
    async fn default_pipeline(&self, pool: &MySqlPool) -> Result<Option<Pipeline>, AppError> {
        PipelineStore::new(pool.clone()).default_pipeline().await
    }
    async fn stages(&self, pool: &MySqlPool, id: Uuid) -> Result<Vec<PipelineStage>, AppError> {
        PipelineStore::new(pool.clone()).list_stages(id).await
    }
    async fn stages_for_pipelines(
        &self,
        pool: &MySqlPool,
        ids: &[Uuid],
    ) -> Result<Vec<PipelineStage>, AppError> {
        PipelineStore::new(pool.clone())
            .stages_for_pipelines(ids)
            .await
    }
    async fn stage_names(
        &self,
        pool: &MySqlPool,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, AppError> {
        if ids.len() > 1000 {
            return Err(AppError::Validation(
                "at most 1000 stage names may be requested".into(),
            ));
        }
        PipelineStore::new(pool.clone()).stage_names(ids).await
    }
    async fn stage(&self, pool: &MySqlPool, id: Uuid) -> Result<Option<PipelineStage>, AppError> {
        PipelineStore::new(pool.clone()).find_stage(id).await
    }
    async fn lost_reasons(&self, pool: &MySqlPool) -> Result<Vec<LostReason>, AppError> {
        PipelineStore::new(pool.clone()).list_lost_reasons().await
    }
    async fn lost_reason(
        &self,
        pool: &MySqlPool,
        id: Uuid,
    ) -> Result<Option<LostReason>, AppError> {
        PipelineStore::new(pool.clone()).find_lost_reason(id).await
    }
}

pub(crate) struct StageRefs;
#[async_trait::async_trait]
impl aphel_sdk::points::ReferenceProvider for StageRefs {
    fn kind(&self) -> aphel_sdk::ReferenceKind {
        aphel_sdk::workflow::STAGE
    }
    async fn exists(&self, ctx: &aphel_kernel::Ctx<'_>, id: &str) -> Result<bool, AppError> {
        aphel_sdk::points::lookup_exists(crate::service::get_stage(ctx, id).await)
    }
}
