use crate::engine::IntegrationRegistry;

pub(crate) fn builtin_integrations() -> IntegrationRegistry {
    IntegrationRegistry::new()
}
