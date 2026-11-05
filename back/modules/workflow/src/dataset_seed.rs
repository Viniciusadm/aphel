use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, MaintenanceCatalog};
use aphel_sdk::shared::RecordType;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::Row;

use crate::domain::UpdateStageCommand;

const ROTTING_DAYS: [i64; 4] = [7, 10, 14, 21];
const STAGE_OPEN: &str = "open";

pub(crate) struct WorkflowSeed;

#[async_trait]
impl DatasetSeed for WorkflowSeed {
    fn order(&self) -> i32 {
        600
    }

    fn entities(&self) -> Vec<RecordType> {
        Vec::new()
    }

    async fn catalog(
        &self,
        ctx: &Ctx<'_>,
        catalog: &mut MaintenanceCatalog,
    ) -> Result<(), AppError> {
        catalog.stages = sqlx::query(
            "SELECT s.id, s.kind FROM workflow_stages s \
             JOIN workflow p ON p.id = s.pipeline_id \
             WHERE p.is_default = TRUE AND s.is_active = TRUE \
             ORDER BY s.position, s.id",
        )
        .fetch_all(ctx.pool())
        .await?
        .into_iter()
        .map(|row| Ok((row.try_get("id")?, row.try_get("kind")?)))
        .collect::<Result<Vec<_>, AppError>>()?;
        catalog.lost_reason_ids = sqlx::query_scalar(
            "SELECT id FROM lost_reasons WHERE is_active = TRUE ORDER BY position, id",
        )
        .fetch_all(ctx.pool())
        .await?;
        Ok(())
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let open_stages: Vec<String> = session
            .catalog()
            .stages
            .iter()
            .filter(|(_, kind)| kind == STAGE_OPEN)
            .map(|(id, _)| id.clone())
            .collect();
        for (index, stage_id) in open_stages.iter().enumerate() {
            let current = crate::service::get_stage(ctx, stage_id).await?;
            if current.rotting_days.is_some() {
                continue;
            }
            crate::service::update_stage(
                ctx,
                stage_id,
                UpdateStageCommand {
                    rotting_days: Some(Some(ROTTING_DAYS[index.min(ROTTING_DAYS.len() - 1)])),
                    ..UpdateStageCommand::default()
                },
            )
            .await?;
        }
        Ok(())
    }
}
