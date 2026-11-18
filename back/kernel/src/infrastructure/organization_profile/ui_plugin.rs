use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::application::user_preferences::UiPluginDefaults;
use crate::database::transaction::Tx;
use crate::errors::AppError;

pub async fn find_ui_plugin_default(pool: &MySqlPool) -> Result<Option<String>, AppError> {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT JSON_UNQUOTE(JSON_EXTRACT(settings, '$.ui_plugin')) FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(value.filter(|value| value != "null" && !value.is_empty()))
}

pub async fn save_ui_plugin_default(
    tx: &mut Tx,
    key: Option<&str>,
) -> Result<Option<String>, AppError> {
    let current: Option<String> = sqlx::query_scalar(
        "SELECT JSON_UNQUOTE(JSON_EXTRACT(settings, '$.ui_plugin')) FROM organization_settings WHERE id = 1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?
    .flatten();
    let previous = current.filter(|value| value != "null" && !value.is_empty());
    match key {
        Some(key) => {
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_SET(CASE WHEN settings IS NULL OR JSON_TYPE(settings) = 'NULL' THEN JSON_OBJECT() ELSE settings END, '$.ui_plugin', ?) WHERE id = 1",
            )
            .bind(key)
            .execute(&mut **tx)
            .await?;
        }
        None => {
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_REMOVE(settings, '$.ui_plugin') WHERE id = 1 AND settings IS NOT NULL AND JSON_TYPE(settings) = 'OBJECT'",
            )
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(previous)
}

pub async fn clear_ui_plugin(tx: &mut Tx, key: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE user_preferences SET ui_plugin = NULL WHERE ui_plugin = ?")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    let default: Option<String> = sqlx::query_scalar(
        "SELECT JSON_UNQUOTE(JSON_EXTRACT(settings, '$.ui_plugin')) FROM organization_settings WHERE id = 1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?
    .flatten();
    if default.as_deref() == Some(key) {
        save_ui_plugin_default(tx, None).await?;
    }
    Ok(())
}

pub struct MySqlUiPluginDefaults {
    pool: MySqlPool,
}

impl MySqlUiPluginDefaults {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UiPluginDefaults for MySqlUiPluginDefaults {
    async fn find(&self) -> Result<Option<String>, AppError> {
        find_ui_plugin_default(&self.pool).await
    }
}
