use std::ops::Deref;

use crate::database::Db;
use crate::modules::manifest::{ManifestBase, Migration, SettingField};
use crate::modules::plugin_manifest::{Layer, PluginManifest, Provides, Runtime, Uses};
use async_trait::async_trait;
use serde_json::Value;

use crate::database::transaction::Tx;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::modules::builder::AppBuilder;
use crate::modules::services::Services;

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub base: ManifestBase,
    pub uninstall_sql: Option<&'static str>,
    pub required: bool,
    pub enabled_by_default: bool,
    pub optional: Vec<&'static str>,
    pub layer: Option<Layer>,
    pub runtime: Runtime,
    pub provides: Provides,
    pub uses: Uses,
}

impl Manifest {
    pub fn new(key: &'static str, name: &'static str, version: &'static str) -> Self {
        Self {
            base: ManifestBase::new(key, name, version),
            uninstall_sql: None,
            required: false,
            enabled_by_default: false,
            optional: Vec::new(),
            layer: None,
            runtime: Runtime::Native,
            provides: Provides::default(),
            uses: Uses::default(),
        }
    }

    pub fn description(mut self, description: &'static str) -> Self {
        self.base = self.base.description(description);
        self
    }

    pub fn requires_aphel(mut self, version: &'static str) -> Self {
        self.base = self.base.min_aphel_version(version);
        self
    }

    pub fn depends_on(mut self, key: &'static str, version_req: &'static str) -> Self {
        self.base = self.base.depends_on(key, version_req);
        self
    }

    pub fn optional(mut self, key: &'static str) -> Self {
        if !self.optional.contains(&key) {
            self.optional.push(key);
        }
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn enabled_by_default(mut self) -> Self {
        self.enabled_by_default = true;
        self
    }

    pub fn setting(mut self, field: SettingField) -> Self {
        self.base = self.base.setting(field);
        self
    }

    pub fn migration(mut self, migration: Migration) -> Self {
        self.base = self.base.migration(migration);
        self
    }

    pub fn owned_tables(mut self, tables: &[&'static str]) -> Self {
        self.base = self.base.owned_tables(tables);
        self
    }

    pub fn uninstall_sql(mut self, sql: &'static str) -> Self {
        self.uninstall_sql = Some(sql);
        self
    }

    pub fn plugin_toml(mut self, text: &str) -> Self {
        match PluginManifest::parse(text) {
            Ok(plugin) if plugin.key == self.base.key => {
                self.layer = Some(plugin.layer);
                self.runtime = plugin.runtime;
                self.provides = plugin.provides;
                self.uses = plugin.uses;
            }
            Ok(plugin) => tracing::error!(
                "plugin.toml of `{}` declares the key `{}`",
                self.base.key,
                plugin.key
            ),
            Err(error) => tracing::error!("plugin.toml of `{}`: {error}", self.base.key),
        }
        self
    }

    pub fn key(&self) -> &'static str {
        self.base.key
    }

    pub fn load_after(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.base
            .dependencies
            .iter()
            .map(|dependency| dependency.key)
            .chain(self.optional.iter().copied())
    }
}

impl Deref for Manifest {
    type Target = ManifestBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub struct ModuleCtx<'a> {
    pub pool: &'a Db,
    pub scope: &'a AccessScope,
    pub services: &'a Services,
    pub module: &'static str,
}

#[async_trait]
pub trait Module: Send + Sync + 'static {
    fn manifest(&self) -> Manifest;

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError>;

    async fn on_enable(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> {
        Ok(())
    }

    async fn on_disable(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> {
        Ok(())
    }

    async fn uninstall(&self, _ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> {
        Ok(())
    }

    fn validate_settings(&self, _settings: &Value) -> Vec<FieldViolation> {
        Vec::new()
    }

    async fn export_data(&self, _ctx: &mut ModuleCtx<'_>) -> Result<Value, AppError> {
        Ok(Value::Null)
    }

    async fn purge_data(&self, _ctx: &mut ModuleCtx<'_>, _tx: &mut Tx) -> Result<(), AppError> {
        Ok(())
    }
}
