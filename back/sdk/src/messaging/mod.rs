pub mod text;
use crate::AppError;
use crate::party::ContactPointKind;
pub use aphel_contract_messaging::*;
use aphel_kernel::errors::FieldViolation;
use async_trait::async_trait;
use serde_json::Value;
pub struct ValidatedConfig {
    pub config: Value,
    pub violations: Vec<FieldViolation>,
}

#[async_trait]
pub trait MessagingChannel: Send + Sync {
    fn definition(&self) -> ChannelDefinition;
    fn key(&self) -> &'static str;
    fn contact_kind(&self) -> ContactPointKind;
    fn uses_references(&self) -> bool;
    fn reply_window(&self) -> Option<ReplyWindow> {
        None
    }
    fn secret_names(&self) -> &'static [&'static str];
    fn required_secrets(&self) -> &'static [&'static str];
    fn validate_config(&self, config: &Value) -> ValidatedConfig;
    fn max_body_chars(&self) -> usize;
    fn address(&self, config: &Value) -> String;
    async fn test(
        &self,
        ctx: &crate::Ctx<'_>,
        context: &ChannelContext<'_>,
    ) -> Result<String, ChannelError>;
    async fn send(
        &self,
        ctx: &crate::Ctx<'_>,
        context: &ChannelContext<'_>,
        message: &OutgoingMessage,
    ) -> Result<SentMessage, ChannelError>;
    async fn fetch(
        &self,
        _ctx: &crate::Ctx<'_>,
        _context: &ChannelContext<'_>,
        state: FetchState,
        _limit: usize,
    ) -> Result<FetchBatch, ChannelError> {
        Ok(FetchBatch {
            state,
            messages: Vec::new(),
        })
    }
    fn polls(&self) -> bool {
        false
    }
}

pub fn channel_provider(key: &str) -> Result<std::sync::Arc<dyn MessagingChannel>, AppError> {
    aphel_kernel::modules::app()
        .contributions::<dyn MessagingChannel>()
        .into_iter()
        .find(|provider| provider.key() == key)
        .ok_or_else(|| {
            AppError::conflict_code(
                "messaging_provider_unavailable",
                format!("messaging provider {key} is unavailable"),
            )
        })
}
pub fn compiled_channel_provider(key: &str) -> Option<std::sync::Arc<dyn MessagingChannel>> {
    aphel_kernel::modules::app()
        .all_contributions::<dyn MessagingChannel>()
        .into_iter()
        .map(|(_, provider)| provider)
        .find(|provider| provider.key() == key)
}

pub mod mentions;
