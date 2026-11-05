use async_trait::async_trait;

use crate::domain::MoveDealCommand;
use aphel_contract_automation::application::StageOperations;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;

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
