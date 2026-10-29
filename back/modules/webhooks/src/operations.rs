use async_trait::async_trait;

use aphel_kernel::Ctx;
use aphel_kernel::application::automations::IncomingWebhookOperations;
use aphel_kernel::errors::AppError;

pub(crate) struct AutomationIncomingWebhooks;

#[async_trait]
impl IncomingWebhookOperations for AutomationIncomingWebhooks {
    async fn find_incoming_webhook(&self, ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
        crate::incoming::service::get(ctx, id).await.map(|_| ())
    }
}
