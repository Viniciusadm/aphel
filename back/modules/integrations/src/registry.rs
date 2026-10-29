use std::sync::Arc;

use aphel_kernel::application::integrations::{IntegrationRegistry, WebhookIntegrationProvider};
use aphel_kernel::infrastructure::integrations::SmtpIntegrationProvider;

pub(crate) fn builtin_integrations() -> IntegrationRegistry {
    IntegrationRegistry::new()
        .with(Arc::new(SmtpIntegrationProvider))
        .with(Arc::new(WebhookIntegrationProvider))
}
