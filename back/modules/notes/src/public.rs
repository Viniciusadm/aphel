use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::public_events::{PublicResource, PublicResourceLoader};

use crate::http::NoteResponse;

pub(crate) struct NotePublicResource;

#[async_trait]
impl PublicResourceLoader for NotePublicResource {
    fn resource(&self) -> PublicResource {
        crate::NOTE_RESOURCE
    }

    async fn load(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Value, AppError> {
        let resource = NoteResponse::from(crate::service::get(ctx, &id.to_string()).await?);
        serde_json::to_value(resource)
            .map_err(|error| AppError::Internal(format!("failed to render public event: {error}")))
    }
}
