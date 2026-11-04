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

#[async_trait::async_trait]
impl aphel_sdk::tasks::TaskTypes for Writer {
    async fn list(
        &self,
        ctx: &Ctx<'_>,
        include_inactive: bool,
        window: aphel_kernel::application::organization_admin::ListWindow,
    ) -> Result<
        aphel_kernel::application::organization_admin::Listing<aphel_sdk::tasks::ActivityTypeView>,
        AppError,
    > {
        crate::types::list(ctx, include_inactive, window).await
    }
}
