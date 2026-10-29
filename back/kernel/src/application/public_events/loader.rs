use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::application::public_events::catalog::PublicResource;
use crate::context::Ctx;
use crate::errors::AppError;

#[async_trait]
pub trait PublicResourceLoader: Send + Sync {
    fn resource(&self) -> PublicResource;

    async fn load(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Value, AppError>;
}

pub fn public_resource_loader(
    resource: PublicResource,
) -> Option<std::sync::Arc<dyn PublicResourceLoader>> {
    crate::modules::try_app()?
        .contributions::<dyn PublicResourceLoader>()
        .into_iter()
        .find(|loader| loader.resource().key() == resource.key())
}
