use crate::{AppError, Ctx};
pub use aphel_contract_email::*;
use aphel_kernel::database::transaction::MySqlTx;
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
    async fn enqueue_in_tx(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
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
        tx: &mut MySqlTx,
        record: &crate::shared::RecordRef,
    ) -> Result<EmailEngagement, AppError>;
}
#[async_trait::async_trait]
pub trait EmailEngagementProjection: Send + Sync {
    fn record_type(&self) -> &'static str;
    async fn refresh(
        &self,
        pool: &sqlx::MySqlPool,
        record: &crate::shared::RecordRef,
        reader: &dyn EmailEngagementReader,
    ) -> Result<(), AppError>;
}
