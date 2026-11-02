pub mod transport;

pub use transport::*;

pub trait OutboundHttpPolicy: Send + Sync {
    fn configure(
        &self,
        settings: &crate::application::organizations::OrganizationConfig,
        base: &crate::config::OutboundHttpConfig,
    ) -> crate::config::OutboundHttpConfig;
}
