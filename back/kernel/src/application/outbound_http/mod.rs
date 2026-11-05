pub(crate) mod transport;

pub use transport::*;

pub trait OutboundHttpPolicy: Send + Sync {
    fn configure(
        &self,
        settings: &crate::application::organization_profile::OrganizationConfig,
        base: &crate::config::OutboundHttpConfig,
    ) -> crate::config::OutboundHttpConfig;
}
