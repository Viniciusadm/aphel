use async_trait::async_trait;
use sqlx::MySqlPool;
use sqlx::types::Json;

use crate::application::keybindings::{Keybindings, keybindings_from_json, keybindings_to_json};
use crate::application::user_preferences::KeybindingDefaults;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;

fn decode(value: Option<Option<Json<serde_json::Value>>>) -> Keybindings {
    value
        .flatten()
        .map(|Json(value)| keybindings_from_json(&value))
        .unwrap_or_default()
}

pub async fn find_organization_keybindings(pool: &MySqlPool) -> Result<Keybindings, AppError> {
    let value: Option<Option<Json<serde_json::Value>>> = sqlx::query_scalar(
        "SELECT JSON_EXTRACT(settings, '$.keybindings') FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(decode(value))
}

pub async fn save_organization_keybindings(
    tx: &mut MySqlTx,
    keybindings: &Keybindings,
) -> Result<Keybindings, AppError> {
    let current: Option<Option<Json<serde_json::Value>>> = sqlx::query_scalar(
        "SELECT JSON_EXTRACT(settings, '$.keybindings') FROM organization_settings WHERE id = 1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE organization_settings SET settings = JSON_SET(CASE WHEN settings IS NULL OR JSON_TYPE(settings) = 'NULL' THEN JSON_OBJECT() ELSE settings END, '$.keybindings', CAST(? AS JSON)) WHERE id = 1",
    )
    .bind(keybindings_to_json(keybindings).to_string())
    .execute(&mut **tx)
    .await?;
    Ok(decode(current))
}

pub struct MySqlKeybindingDefaults {
    pool: MySqlPool,
}

impl MySqlKeybindingDefaults {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KeybindingDefaults for MySqlKeybindingDefaults {
    async fn find(&self) -> Result<Keybindings, AppError> {
        find_organization_keybindings(&self.pool).await
    }
}
