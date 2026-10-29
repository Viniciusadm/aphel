use serde_json::{Map, Value};
use sqlx::MySqlPool;

use crate::application::module_admin::SECRET_OWNER_TYPE;
use crate::application::secrets::SecretKey;
use crate::errors::AppError;

pub fn module_secret_key(module: &str, name: &str) -> SecretKey {
    SecretKey::new(SECRET_OWNER_TYPE, module, name)
}

pub async fn module_settings(pool: &MySqlPool, module: &str) -> Result<Value, AppError> {
    let stored: Option<sqlx::types::Json<Value>> =
        sqlx::query_scalar("SELECT settings FROM installed_modules WHERE module_key = ?")
            .bind(module)
            .fetch_optional(pool)
            .await?;
    let mut settings = match stored.map(|json| json.0) {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    if let Some(entry) = crate::modules::app().module(module) {
        for field in &entry.manifest.settings {
            if field.is_secret() {
                continue;
            }
            if let Some(default) = &field.default
                && !settings.contains_key(field.key)
            {
                settings.insert(field.key.to_owned(), default.clone());
            }
        }
    }
    Ok(Value::Object(settings))
}
