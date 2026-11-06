use std::sync::Arc;

use async_trait::async_trait;

use crate::errors::AppError;

pub const UI_PLUGIN_UPDATED: &str = "organization.ui_plugin_updated";
pub const MAX_UI_PLUGIN_KEY_LENGTH: usize = 32;
pub const CORE_UI_PLUGIN: &str = "core";

#[async_trait]
pub trait UiPluginDirectory: Send + Sync {
    async fn is_available(&self, key: &str) -> Result<bool, AppError>;
}

pub fn is_ui_plugin_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_UI_PLUGIN_KEY_LENGTH
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn directory() -> Option<Arc<dyn UiPluginDirectory>> {
    crate::modules::try_app()?
        .services()
        .get::<dyn UiPluginDirectory>()
}

pub async fn ui_plugin_available(key: &str) -> Result<bool, AppError> {
    match directory() {
        Some(directory) => directory.is_available(key).await,
        None => Ok(false),
    }
}

pub async fn effective_ui_plugin(
    stored: Option<&str>,
    default: Option<&str>,
) -> Result<Option<String>, AppError> {
    if stored == Some(CORE_UI_PLUGIN) {
        return Ok(None);
    }
    for candidate in [stored, default].into_iter().flatten() {
        if ui_plugin_available(candidate).await? {
            return Ok(Some(candidate.to_owned()));
        }
    }
    Ok(None)
}
