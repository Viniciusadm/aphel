use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::errors::AppError;

#[async_trait]
pub trait ModuleToggleGuard: Send + Sync {
    async fn held_modules(&self, pool: &MySqlPool) -> Result<HashMap<String, String>, AppError>;
}

pub async fn held_modules(pool: &MySqlPool) -> Result<HashMap<String, String>, AppError> {
    let mut held = HashMap::new();
    for guard in crate::modules::app().contributions::<dyn ModuleToggleGuard>() {
        held.extend(guard.held_modules(pool).await?);
    }
    Ok(held)
}
