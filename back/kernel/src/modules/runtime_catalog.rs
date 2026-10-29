use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::errors::AppError;

#[async_trait]
pub trait RuntimeCatalogSource: Send + Sync {
    async fn ensure_ready(&self, pool: &MySqlPool) -> Result<(), AppError>;
}

pub async fn ensure_ready(pool: &MySqlPool) -> Result<(), AppError> {
    for source in crate::modules::app().contributions::<dyn RuntimeCatalogSource>() {
        source.ensure_ready(pool).await?;
    }
    Ok(())
}
