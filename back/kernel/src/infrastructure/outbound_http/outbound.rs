use std::sync::Arc;

use crate::config::OutboundHttpConfig;
use crate::errors::AppError;
use crate::infrastructure::outbound_http::{
    HttpOutboundSender, NetworkUrlGuard, OutboundNetworkPolicy,
};

pub struct OutboundHttpNetwork {
    url_guard: NetworkUrlGuard,
    sender: Arc<HttpOutboundSender>,
}

impl OutboundHttpNetwork {
    pub(crate) fn new(config: &OutboundHttpConfig) -> Result<Self, AppError> {
        let policy = OutboundNetworkPolicy::from_config(config);
        Ok(Self {
            url_guard: NetworkUrlGuard::new(policy),
            sender: Arc::new(HttpOutboundSender::new(policy, config.timeout)?),
        })
    }

    pub fn guard(&self) -> &NetworkUrlGuard {
        &self.url_guard
    }

    pub fn http_sender(&self) -> &Arc<HttpOutboundSender> {
        &self.sender
    }
}
