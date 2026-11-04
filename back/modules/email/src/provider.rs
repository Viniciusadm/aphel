use serde_json::Value;

use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub use aphel_sdk::email::mailbox::{
    MailProviderError, MailProviderErrorKind, MailboxProvider, SharedMailboxProvider,
};

#[derive(Clone, Default)]
pub struct MailboxProviderRegistry {
    providers: Vec<SharedMailboxProvider>,
}

impl MailboxProviderRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, provider: SharedMailboxProvider) {
        self.providers
            .retain(|existing| existing.key() != provider.key());
        self.providers.push(provider);
    }

    pub fn find(&self, key: &str) -> Option<SharedMailboxProvider> {
        self.providers
            .iter()
            .find(|provider| provider.key() == key)
            .cloned()
    }

    pub(crate) fn available_for(&self, scope: &AccessScope) -> Vec<SharedMailboxProvider> {
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

    pub(crate) fn require_for(
        &self,
        scope: &AccessScope,
        key: &str,
    ) -> Result<SharedMailboxProvider, AppError> {
        self.available_for(scope)
            .into_iter()
            .find(|provider| provider.key() == key)
            .ok_or_else(|| AppError::Validation(format!("mail provider {key} is not available")))
    }
}

impl std::fmt::Debug for MailboxProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.providers
                    .iter()
                    .map(|provider| provider.key().to_owned()),
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
