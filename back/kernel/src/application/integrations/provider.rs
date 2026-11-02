use std::sync::Arc;

use async_trait::async_trait;

use crate::application::integrations::schema::{ConfigSchema, ResolvedConfig};
use crate::domain::access::AccessScope;
use crate::errors::FieldViolation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Configurable,
    Shortcut { path: &'static str },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionTestOptions {
    pub send_test_email_to: Option<String>,
}

#[async_trait]
pub trait IntegrationProvider: Send + Sync {
    fn key(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn icon(&self) -> &'static str;

    fn kind(&self) -> ProviderKind {
        ProviderKind::Configurable
    }

    fn module(&self) -> Option<&'static str> {
        None
    }

    fn is_available(&self, scope: &AccessScope) -> bool {
        self.module()
            .is_none_or(|module| scope.module_enabled(module))
    }

    fn config_schema(&self) -> ConfigSchema;

    fn validate(&self, _config: &ResolvedConfig) -> Vec<FieldViolation> {
        Vec::new()
    }

    fn requires_successful_test_to_activate(&self) -> bool {
        false
    }

    async fn test_connection(
        &self,
        config: &ResolvedConfig,
        options: &ConnectionTestOptions,
    ) -> Result<(), String>;

    async fn on_activate(&self, _config: &ResolvedConfig) -> Result<(), String> {
        Ok(())
    }

    async fn on_deactivate(&self) -> Result<(), String> {
        Ok(())
    }
}

pub type SharedIntegrationProvider = Arc<dyn IntegrationProvider>;

#[derive(Clone, Default)]
pub struct IntegrationRegistry {
    providers: Vec<SharedIntegrationProvider>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, provider: SharedIntegrationProvider) -> Self {
        self.providers
            .retain(|existing| existing.key() != provider.key());
        self.providers.push(provider);
        self
    }

    pub fn providers(&self) -> &[SharedIntegrationProvider] {
        &self.providers
    }

    pub fn get(&self, key: &str) -> Option<&SharedIntegrationProvider> {
        self.providers.iter().find(|provider| provider.key() == key)
    }
}

pub const WEBHOOK_PROVIDER_KEY: &str = "webhook";
pub const WEBHOOKS_SETTINGS_PATH: &str = "/integrations/webhooks";

pub struct WebhookIntegrationProvider;

#[async_trait]
impl IntegrationProvider for WebhookIntegrationProvider {
    fn key(&self) -> &'static str {
        WEBHOOK_PROVIDER_KEY
    }

    fn name(&self) -> &'static str {
        "Generic webhook"
    }

    fn description(&self) -> &'static str {
        "Send application events to any HTTPS endpoint"
    }

    fn icon(&self) -> &'static str {
        "webhook"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Shortcut {
            path: WEBHOOKS_SETTINGS_PATH,
        }
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema::default()
    }

    async fn test_connection(
        &self,
        _config: &ResolvedConfig,
        _options: &ConnectionTestOptions,
    ) -> Result<(), String> {
        Err("webhook endpoints are tested from the webhooks settings".to_owned())
    }
}
