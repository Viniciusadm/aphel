use std::sync::Arc;

use crate::engine::{IntegrationRegistry, WebhookIntegrationProvider};

pub(crate) fn builtin_integrations() -> IntegrationRegistry {
    IntegrationRegistry::new().with(Arc::new(WebhookIntegrationProvider))
}
