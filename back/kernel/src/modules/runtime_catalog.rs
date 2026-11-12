use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::state::AppState;

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

pub struct PluginInstallRequest {
    pub package: Vec<u8>,
    pub role_presets: Vec<String>,
    pub dry_run: bool,
    pub acknowledge_conflicts: bool,
    pub acknowledge_web_version: Option<String>,
}

pub struct PluginToggleRequest {
    pub enabled: bool,
    pub acknowledge_conflicts: bool,
    pub acknowledge_web_version: Option<String>,
}

pub struct PluginPackResult {
    pub status: u16,
    pub body: serde_json::Value,
}

pub struct PluginInstallResult {
    pub installed: bool,
    pub body: serde_json::Value,
}

#[async_trait]
pub trait PluginAdministration: Send + Sync {
    async fn list(&self, state: &AppState) -> Result<serde_json::Value, AppError>;

    async fn install(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallResult, AppError>;

    async fn set_enabled(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        key: &str,
        request: PluginToggleRequest,
    ) -> Result<serde_json::Value, AppError>;

    async fn uninstall(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        key: &str,
        purge: bool,
    ) -> Result<(), AppError>;

    async fn install_pack(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        request: PluginInstallRequest,
    ) -> Result<PluginPackResult, AppError>;

    async fn reapply_grants(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        key: &str,
    ) -> Result<serde_json::Value, AppError>;

    async fn readiness(&self, state: &AppState) -> Result<serde_json::Value, AppError>;

    async fn settings(&self, state: &AppState, key: &str) -> Result<serde_json::Value, AppError>;

    async fn update_settings(
        &self,
        state: &AppState,
        scope: &AccessScope,
        ip: Option<String>,
        key: &str,
        values: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AppError>;
}

pub fn plugin_administration() -> Result<std::sync::Arc<dyn PluginAdministration>, AppError> {
    crate::modules::app()
        .contributions::<dyn PluginAdministration>()
        .into_iter()
        .next()
        .ok_or_else(|| {
            AppError::not_found_code(
                "module_not_installed",
                "this build has no runtime plugin module",
            )
        })
}
