use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelephonyStartRequest {
    pub(crate) user_id: Uuid,
    pub(crate) to: String,
    pub(crate) record: Option<RecordRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelephonyCall {
    pub(crate) provider_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelephonyRecordingRequest {
    pub(crate) user_id: Uuid,
    pub(crate) provider_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelephonyRecording {
    pub(crate) url: String,
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub(crate) trait TelephonyProvider: Send + Sync {
    fn key(&self) -> &str;

    fn module(&self) -> Option<&str> {
        None
    }

    fn label(&self) -> &str;

    async fn start_call(&self, request: &TelephonyStartRequest) -> Result<TelephonyCall, AppError>;

    async fn recording(
        &self,
        _request: &TelephonyRecordingRequest,
    ) -> Result<Option<TelephonyRecording>, AppError> {
        Ok(None)
    }
}

pub(crate) type SharedTelephonyProvider = Arc<dyn TelephonyProvider>;

#[derive(Default)]
pub(crate) struct TelephonyProviderRegistry {
    providers: Vec<SharedTelephonyProvider>,
}

impl TelephonyProviderRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, provider: SharedTelephonyProvider) {
        self.providers
            .retain(|existing| existing.key() != provider.key());
        self.providers.push(provider);
    }

    pub(crate) fn available_for(&self, scope: &AccessScope) -> Vec<SharedTelephonyProvider> {
        self.providers
            .iter()
            .filter(|provider| {
                provider
                    .module()
                    .is_none_or(|module| scope.module_enabled(module))
            })
            .cloned()
            .collect()
    }

    pub(crate) fn find_for(
        &self,
        scope: &AccessScope,
        key: &str,
    ) -> Option<SharedTelephonyProvider> {
        self.available_for(scope)
            .into_iter()
            .find(|provider| provider.key() == key)
    }
}

pub(crate) trait TelephonyRouter: Send + Sync {
    fn providers(&self, scope: &AccessScope) -> Vec<SharedTelephonyProvider>;

    fn provider(&self, scope: &AccessScope, key: &str) -> Option<SharedTelephonyProvider> {
        self.providers(scope)
            .into_iter()
            .find(|provider| provider.key() == key)
    }
}

#[derive(Default)]
pub(crate) struct RegistryTelephonyRouter {
    registry: OnceLock<TelephonyProviderRegistry>,
}

impl RegistryTelephonyRouter {
    fn registry(&self) -> &TelephonyProviderRegistry {
        self.registry.get_or_init(|| {
            let mut registry = TelephonyProviderRegistry::new();
            for (_, provider) in
                aphel_kernel::modules::app().all_contributions::<dyn TelephonyProvider>()
            {
                registry.register(provider);
            }
            registry
        })
    }
}

impl TelephonyRouter for RegistryTelephonyRouter {
    fn providers(&self, scope: &AccessScope) -> Vec<SharedTelephonyProvider> {
        self.registry().available_for(scope)
    }

    fn provider(&self, scope: &AccessScope, key: &str) -> Option<SharedTelephonyProvider> {
        self.registry().find_for(scope, key)
    }
}

pub(crate) fn shared_router() -> Arc<dyn TelephonyRouter> {
    Arc::new(RegistryTelephonyRouter::default())
}
