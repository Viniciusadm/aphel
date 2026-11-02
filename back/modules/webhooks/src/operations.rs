use async_trait::async_trait;

use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

pub(crate) struct IncomingWebhookRefs;

#[async_trait]
impl aphel_sdk::points::ReferenceProvider for IncomingWebhookRefs {
    fn kind(&self) -> aphel_sdk::ReferenceKind {
        aphel_contract_crm::refs::INCOMING_WEBHOOK
    }

    async fn exists(&self, ctx: &Ctx<'_>, id: &str) -> Result<bool, AppError> {
        aphel_sdk::points::lookup_exists(crate::incoming::service::get(ctx, id).await)
    }
}
