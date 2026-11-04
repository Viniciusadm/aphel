pub(crate) struct Jobs;
#[async_trait::async_trait]
impl aphel_sdk::exports::ExportJobs for Jobs {
    async fn request(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        command: aphel_sdk::exports::RequestExportCommand,
    ) -> Result<aphel_sdk::exports::ExportJobResponse, aphel_kernel::errors::AppError> {
        crate::service::request(ctx, command).await.map(Into::into)
    }
}
