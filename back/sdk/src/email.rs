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
