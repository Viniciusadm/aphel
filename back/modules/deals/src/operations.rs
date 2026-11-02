use async_trait::async_trait;

use crate::domain::MoveDealCommand;
use aphel_kernel::Ctx;
use aphel_kernel::application::automations::StageOperations;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::errors::AppError;

pub(crate) struct AutomationStages;

#[async_trait]
impl StageOperations for AutomationStages {
    async fn move_to_stage(
        &self,
        ctx: &Ctx<'_>,
        deal: &RecordRef,
        stage_id: &str,
    ) -> Result<(), AppError> {
        crate::service::move_to_stage(
            ctx,
            &deal.record_id.to_string(),
            MoveDealCommand {
                stage_id: stage_id.to_owned(),
                ..MoveDealCommand::default()
            },
        )
        .await
        .map(|_| ())
    }
}

pub(crate) struct StageRefs;

#[async_trait]
impl aphel_sdk::points::ReferenceProvider for StageRefs {
    fn kind(&self) -> aphel_sdk::ReferenceKind {
        aphel_contract_crm::refs::STAGE
    }

    async fn exists(&self, ctx: &Ctx<'_>, id: &str) -> Result<bool, AppError> {
        aphel_sdk::points::lookup_exists(pipelines::service::get_stage(ctx, id).await)
    }
}
