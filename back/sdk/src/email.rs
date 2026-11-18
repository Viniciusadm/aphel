pub mod mailbox;
pub mod mime;
pub mod oauth;
use crate::{AppError, Ctx};
pub use aphel_contract_email::*;
use aphel_kernel::database::Db;
use aphel_kernel::database::transaction::Tx;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = EmailAddressInput)]
pub struct EmailAddressDto {
    #[schema(example = "maria@customer.com")]
    pub address: String,
    #[serde(default)]
    pub name: Option<String>,
}

impl From<EmailAddressDto> for EmailAddress {
    fn from(value: EmailAddressDto) -> Self {
        Self {
            address: value.address,
            name: value.name,
        }
    }
}

#[async_trait::async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, ctx: &Ctx<'_>, command: SendEmailCommand) -> Result<SentEmail, AppError>;
    async fn preview(
        &self,
        ctx: &Ctx<'_>,
        command: EmailPreviewCommand,
    ) -> Result<RenderedEmail, AppError>;

    async fn enqueue_in_tx(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut Tx,
        command: SendEmailCommand,
    ) -> Result<uuid::Uuid, AppError>;
}

use aphel_kernel::domain::UserId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmailEngagement {
    pub last_contacted_at: Option<DateTime<Utc>>,
    pub last_email_received_at: Option<DateTime<Utc>>,
    pub last_email_sent_at: Option<DateTime<Utc>>,
    pub interaction_count: i64,
    pub last_contacted_by: Option<UserId>,
}

#[async_trait::async_trait]
pub trait EmailEngagementReader: Send + Sync {
    async fn snapshot(
        &self,
        tx: &mut Tx,
        record: &crate::shared::RecordRef,
    ) -> Result<EmailEngagement, AppError>;
}
#[async_trait::async_trait]
pub trait EmailEngagementProjection: Send + Sync {
    fn record_type(&self) -> &'static str;
    async fn refresh(
        &self,
        pool: &Db,
        record: &crate::shared::RecordRef,
        reader: &dyn EmailEngagementReader,
    ) -> Result<(), AppError>;
}

pub mod html;
pub fn address_hash(address: &str) -> String {
    let normalized = normalize_address(address).unwrap_or_else(|_| address.trim().to_lowercase());
    aphel_kernel::application::files::sha256_hex(normalized.as_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmailPreviewCommand {
    pub template_id: Option<String>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
    pub subject: String,
    pub html: String,
    pub text: String,
    pub missing_variables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentEmail {
    pub email_id: uuid::Uuid,
    pub thread_id: uuid::Uuid,
    pub mailbox_id: uuid::Uuid,
}
pub fn sender() -> Result<std::sync::Arc<dyn EmailSender>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn EmailSender>()
        .ok_or_else(|| {
            AppError::unavailable_code("email_sender_unavailable", "email sending is not enabled")
        })
}

#[async_trait::async_trait]
pub trait EmailAutomationReader: Send + Sync {
    async fn organization_template_keys(
        &self,
        pool: &Db,
    ) -> Result<std::collections::HashMap<uuid::Uuid, String>, AppError>;
    async fn organization_template_ids_in(
        &self,
        tx: &mut Tx,
    ) -> Result<std::collections::HashMap<String, uuid::Uuid>, AppError>;
    async fn objection_hash(&self, pool: &Db, id: uuid::Uuid) -> Result<Option<String>, AppError>;
    async fn has_objection(&self, pool: &Db, address_hash: &str) -> Result<bool, AppError>;
    async fn sendable_mailbox(
        &self,
        pool: &Db,
        owner: &UserId,
        id: Option<uuid::Uuid>,
    ) -> Result<Option<uuid::Uuid>, AppError>;
}
pub fn automation_reader() -> Option<std::sync::Arc<dyn EmailAutomationReader>> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn EmailAutomationReader>()
}
