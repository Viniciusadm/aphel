use aphel_kernel::errors::AppError;
pub(crate) use aphel_sdk::documents::{
    EnvelopeRequest, EnvelopeSigner, NoticeKind, SentEnvelope, SignatureProvider, WebhookNotice,
};
pub(crate) const PROVIDER_UNAVAILABLE: &str = "esign_provider_unavailable";
pub(crate) const PROVIDER_UNAUTHORIZED: &str = "esign_provider_unauthorized";
pub(crate) const PROVIDER_REJECTED: &str = "esign_provider_rejected";

pub(crate) fn unavailable(message: impl Into<String>) -> AppError {
    AppError::unavailable_code(PROVIDER_UNAVAILABLE, message)
}

pub(crate) fn unauthorized() -> AppError {
    AppError::unavailable_code(
        PROVIDER_UNAUTHORIZED,
        "o provedor de assinatura recusou o token de acesso configurado",
    )
}

pub(crate) fn rejected(detail: impl Into<String>) -> AppError {
    AppError::validation_code(
        PROVIDER_REJECTED,
        format!(
            "o provedor de assinatura recusou a solicitação: {}",
            detail.into()
        ),
    )
}

pub(crate) struct ClicksignFactory;
#[async_trait::async_trait]
impl aphel_sdk::documents::SignatureProviderFactory for ClicksignFactory {
    fn key(&self) -> &'static str {
        crate::clicksign::PROVIDER_KEY
    }
    async fn create(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        configuration: &serde_json::Value,
    ) -> Result<std::sync::Arc<dyn SignatureProvider>, AppError> {
        let environment = configuration
            .get("environment")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(crate::SANDBOX);
        Ok(std::sync::Arc::new(
            crate::service::build_provider(ctx, environment).await?,
        ))
    }
}
