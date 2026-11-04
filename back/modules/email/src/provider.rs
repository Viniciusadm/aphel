use serde_json::Value;

use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub use aphel_sdk::email::mailbox::{
    MailProviderError, MailProviderErrorKind, MailboxProvider, SharedMailboxProvider,
};

#[derive(Clone)]
struct RegisteredProvider {
    owner: &'static str,
    provider: SharedMailboxProvider,
}
impl RegisteredProvider {
    fn enabled(&self) -> bool {
        let app = aphel_kernel::modules::app();
        app.is_enabled(self.owner)
            && self
                .provider
                .module()
                .is_none_or(|module| app.is_enabled(module))
    }
    fn allowed(&self, scope: &AccessScope) -> bool {
        self.enabled()
            && self.provider.module().is_none_or(|module| {
                scope.module_enabled(module)
                    || (scope.is_system() && scope.enabled_modules().is_empty())
            })
    }
}
#[derive(Clone, Default)]
pub struct MailboxProviderRegistry {
    providers: Vec<RegisteredProvider>,
}
impl MailboxProviderRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn register(&mut self, owner: &'static str, provider: SharedMailboxProvider) {
        self.providers.push(RegisteredProvider { owner, provider });
    }
    pub fn find(&self, key: &str) -> Option<SharedMailboxProvider> {
        let mut matching = self
            .providers
            .iter()
            .filter(|entry| entry.enabled() && entry.provider.key() == key);
        let entry = matching.next()?;
        matching.next().is_none().then(|| entry.provider.clone())
    }
    pub(crate) fn available_for(
        &self,
        scope: &AccessScope,
    ) -> Result<Vec<SharedMailboxProvider>, AppError> {
        let mut keys = std::collections::BTreeSet::new();
        let mut providers = Vec::new();
        for entry in self.providers.iter().filter(|entry| entry.allowed(scope)) {
            if !keys.insert(entry.provider.key()) {
                return Err(AppError::Configuration(format!(
                    "multiple mailbox providers for {}",
                    entry.provider.key()
                )));
            }
            providers.push(entry.provider.clone());
        }
        Ok(providers)
    }
    pub(crate) fn require_for(
        &self,
        scope: &AccessScope,
        key: &str,
    ) -> Result<SharedMailboxProvider, AppError> {
        let mut matching = self
            .providers
            .iter()
            .filter(|entry| entry.allowed(scope) && entry.provider.key() == key);
        let Some(entry) = matching.next() else {
            return Err(AppError::Validation(format!(
                "mail provider {key} is not available"
            )));
        };
        if matching.next().is_some() {
            return Err(AppError::Configuration(format!(
                "multiple mailbox providers for {key}"
            )));
        }
        Ok(entry.provider.clone())
    }
}
impl std::fmt::Debug for MailboxProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.providers
                    .iter()
                    .map(|entry| (entry.owner, entry.provider.key())),
            )
            .finish()
    }
}

pub(crate) fn mailbox_settings_or_empty(settings: Option<Value>) -> Value {
    match settings {
        Some(Value::Object(values)) => Value::Object(values),
        _ => Value::Object(Default::default()),
    }
}
