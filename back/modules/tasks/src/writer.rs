use aphel_sdk::{
    AppError, Ctx,
    tasks::{CreateActivityCommand, TaskWriter},
};
use uuid::Uuid;

pub(crate) struct Writer;

#[async_trait::async_trait]
impl TaskWriter for Writer {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Uuid, AppError> {
        Ok(crate::service::create(ctx, command).await?.activity.id)
    }
}
