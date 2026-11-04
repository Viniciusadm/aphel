pub(crate) struct Sender;
#[async_trait::async_trait]
impl aphel_sdk::email::EmailSender for Sender {
    async fn send(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        command: aphel_sdk::email::SendEmailCommand,
    ) -> Result<aphel_sdk::email::SentEmail, aphel_kernel::errors::AppError> {
        let composed = crate::service::sender().send(ctx, command).await?;
        Ok(aphel_sdk::email::SentEmail {
            email_id: composed.message.id,
            thread_id: composed.message.thread_id,
            mailbox_id: composed.message.mailbox_id,
        })
    }
    async fn preview(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        command: aphel_sdk::email::EmailPreviewCommand,
    ) -> Result<aphel_sdk::email::RenderedEmail, aphel_kernel::errors::AppError> {
        crate::service::templates::preview(ctx, command).await
    }

    async fn enqueue_in_tx(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        command: aphel_sdk::email::SendEmailCommand,
    ) -> Result<uuid::Uuid, aphel_kernel::errors::AppError> {
        crate::service::compose::enqueue_in_tx(ctx, tx, command).await
    }
}
