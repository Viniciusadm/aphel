use async_trait::async_trait;

use crate::domain::MoveDealCommand;
use aphel_kernel::Ctx;
use aphel_kernel::application::automations::StageOperations;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::errors::AppError;

pub(crate) struct AutomationStages;

#[async_trait]
impl StageOperations for AutomationStages {
    async fn find_stage(&self, ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
        pipelines::service::get_stage(ctx, id).await.map(|_| ())
    }

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
