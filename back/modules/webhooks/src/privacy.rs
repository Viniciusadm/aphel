use aphel_kernel::{
    Ctx, application::fields::LocalizedLabels, database::transaction::MySqlTx,
    domain::access::Permission, errors::AppError,
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::privacy::{PrivacySource, Redaction};
pub(crate) struct WebhookPrivacy;
#[async_trait::async_trait]
impl PrivacySource for WebhookPrivacy {
    fn key(&self) -> &'static str {
        "webhooks"
    }
    fn labels(&self) -> LocalizedLabels {
        LocalizedLabels(std::collections::BTreeMap::from([
            ("en".into(), "Webhook history".into()),
            ("pt-BR".into(), "Histórico de webhooks".into()),
        ]))
    }
    async fn available(&self, ctx: &Ctx<'_>) -> Result<bool, AppError> {
        aphel_sdk::privacy::sql::table_exists(ctx, "webhook_deliveries").await
    }
    async fn redact(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        redaction: &Redaction,
    ) -> Result<u64, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let mut updated = 0;
        for (table, column) in [
            ("webhook_deliveries", "body"),
            ("webhook_deliveries", "response_body"),
            ("incoming_webhook_requests", "body"),
            ("incoming_webhook_requests", "headers"),
            ("incoming_webhook_requests", "results"),
        ] {
            updated += aphel_sdk::privacy::sql::redact_text(tx, table, column, redaction).await?;
        }
        Ok(updated)
    }
}
