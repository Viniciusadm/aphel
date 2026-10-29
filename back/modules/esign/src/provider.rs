use async_trait::async_trait;

use aphel_kernel::errors::AppError;

pub(crate) const PROVIDER_UNAVAILABLE: &str = "esign_provider_unavailable";
pub(crate) const PROVIDER_UNAUTHORIZED: &str = "esign_provider_unauthorized";
pub(crate) const PROVIDER_REJECTED: &str = "esign_provider_rejected";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvelopeSigner {
    pub(crate) name: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvelopeRequest {
    pub(crate) name: String,
    pub(crate) file_name: String,
    pub(crate) content: Vec<u8>,
    pub(crate) signers: Vec<EnvelopeSigner>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SentEnvelope {
    pub(crate) envelope_id: String,
    pub(crate) document_id: String,
    pub(crate) signer_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoticeKind {
    SignerSigned,
    SignerRefused,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebhookNotice {
    pub(crate) event_name: String,
    pub(crate) kind: Option<NoticeKind>,
    pub(crate) references: Vec<String>,
    pub(crate) signer_email: Option<String>,
}

#[async_trait]
pub(crate) trait SignatureProvider: Send + Sync {
    fn key(&self) -> &'static str;
    async fn send(&self, request: &EnvelopeRequest) -> Result<SentEnvelope, AppError>;
    async fn cancel(&self, envelope_id: &str) -> Result<(), AppError>;
    async fn signed_document(
        &self,
        envelope_id: &str,
        document_id: &str,
    ) -> Result<Option<Vec<u8>>, AppError>;
}

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
