use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::public_events::{PublicResource, PublicResourceLoader};
use aphel_kernel::errors::AppError;

use crate::http::ExchangeRateResponse;

pub(crate) struct ExchangeRatePublicResource;

#[async_trait]
impl PublicResourceLoader for ExchangeRatePublicResource {
    fn resource(&self) -> PublicResource {
        crate::EXCHANGE_RATE_RESOURCE
    }

    async fn load(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Value, AppError> {
        let resource =
            ExchangeRateResponse::from(crate::service::get_rate(ctx, &id.to_string()).await?);
        serde_json::to_value(resource)
            .map_err(|error| AppError::Internal(format!("failed to render public event: {error}")))
    }
}
