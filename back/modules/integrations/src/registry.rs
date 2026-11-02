use std::sync::Arc;

use crate::engine::{IntegrationRegistry, WebhookIntegrationProvider};
use crate::smtp::SmtpIntegrationProvider;

pub(crate) fn builtin_integrations() -> IntegrationRegistry {
    IntegrationRegistry::new()
        .with(Arc::new(SmtpIntegrationProvider))
        .with(Arc::new(WebhookIntegrationProvider))
}
