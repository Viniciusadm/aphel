pub(crate) struct Sender;
#[async_trait::async_trait]
impl aphel_sdk::email::EmailSender for Sender {
    async fn enqueue_in_tx(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        command: aphel_sdk::email::SendEmailCommand,
    ) -> Result<uuid::Uuid, aphel_kernel::errors::AppError> {
        crate::service::compose::enqueue_in_tx(ctx, tx, command).await
    }
}
