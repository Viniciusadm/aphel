use std::collections::HashMap;

use async_trait::async_trait;

use crate::database::Db;
use crate::errors::AppError;

#[async_trait]
pub trait ModuleToggleGuard: Send + Sync {
    async fn held_modules(&self, pool: &Db) -> Result<HashMap<String, String>, AppError>;

    async fn dependents(&self, _pool: &Db) -> Result<HashMap<String, Vec<String>>, AppError> {
        Ok(HashMap::new())
    }
}

pub async fn held_modules(pool: &Db) -> Result<HashMap<String, String>, AppError> {
    let mut held = HashMap::new();
    for guard in crate::modules::app().contributions::<dyn ModuleToggleGuard>() {
        held.extend(guard.held_modules(pool).await?);
    }
    Ok(held)
}

pub async fn dependents(pool: &Db) -> Result<HashMap<String, Vec<String>>, AppError> {
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for guard in crate::modules::app().contributions::<dyn ModuleToggleGuard>() {
        for (module, names) in guard.dependents(pool).await? {
            dependents.entry(module).or_default().extend(names);
        }
    }
    Ok(dependents)
}
