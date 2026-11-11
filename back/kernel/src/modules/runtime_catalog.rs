use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::errors::AppError;

#[async_trait]
pub trait RuntimeCatalogSource: Send + Sync {
    async fn ensure_ready(&self, pool: &MySqlPool) -> Result<(), AppError>;

    fn backend_errors(&self) -> Vec<String> {
        Vec::new()
    }
}

pub async fn backend_errors(pool: &MySqlPool) -> Result<Vec<String>, AppError> {
    let mut errors = Vec::new();
    for source in crate::modules::app().contributions::<dyn RuntimeCatalogSource>() {
        source.ensure_ready(pool).await?;
        errors.extend(source.backend_errors());
    }
    Ok(errors)
}

pub async fn ensure_ready(pool: &MySqlPool) -> Result<(), AppError> {
    for source in crate::modules::app().contributions::<dyn RuntimeCatalogSource>() {
        source.ensure_ready(pool).await?;
    }
    Ok(())
}
