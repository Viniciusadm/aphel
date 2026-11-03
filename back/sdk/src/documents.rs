use crate::{
    AppError, Ctx,
    shared::{RecordRef, RecordType},
};
pub use aphel_contract_documents::*;
use async_trait::async_trait;
use uuid::Uuid;
pub const KEY: &str = "documents";
pub trait DocumentRenderer: Send + Sync {
    fn render(&self, document: &Document) -> Result<Vec<u8>, AppError>;
}
#[async_trait]
pub trait DocumentSource: Send + Sync {
    fn record_type(&self) -> RecordType;
    async fn document(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<DocumentFile, AppError>;
}
pub fn render_pdf(document: &Document) -> Result<Vec<u8>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .require::<dyn DocumentRenderer>()?
        .render(document)
}
pub async fn resolve(ctx: &Ctx<'_>, record: &RecordRef) -> Result<DocumentFile, AppError> {
    let source = aphel_kernel::modules::app()
        .contributions::<dyn DocumentSource>()
        .into_iter()
        .find(|source| source.record_type() == record.record_type)
        .ok_or_else(|| {
            AppError::not_found_code(
                "document_source_unavailable",
                "the document source is unavailable",
            )
        })?;
    source.document(ctx, record.record_id).await
}

#[async_trait]
pub trait SignatureProvider: Send + Sync {
    fn key(&self) -> &'static str;
    async fn send(&self, request: &EnvelopeRequest) -> Result<SentEnvelope, AppError>;
    async fn cancel(&self, envelope_id: &str) -> Result<(), AppError>;
    async fn signed_document(
        &self,
        envelope_id: &str,
        document_id: &str,
    ) -> Result<Option<Vec<u8>>, AppError>;
}

#[async_trait]
pub trait SignatureProviderFactory: Send + Sync {
    fn key(&self) -> &'static str;
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        configuration: &serde_json::Value,
    ) -> Result<std::sync::Arc<dyn SignatureProvider>, AppError>;
}
pub async fn signature_provider(
    ctx: &Ctx<'_>,
    key: &str,
    configuration: &serde_json::Value,
) -> Result<std::sync::Arc<dyn SignatureProvider>, AppError> {
    let factory = aphel_kernel::modules::app()
        .contributions::<dyn SignatureProviderFactory>()
        .into_iter()
        .find(|factory| factory.key() == key)
        .ok_or_else(|| {
            AppError::unavailable_code(
                "document_signature_provider_unavailable",
                "the document signature provider is unavailable",
            )
        })?;
    factory.create(ctx, configuration).await
}
