use std::sync::Arc;

use crate::config::WebhooksConfig;
use crate::errors::AppError;
use crate::infrastructure::webhooks::outgoing::{
    HttpWebhookSender, NetworkWebhookUrlGuard, WebhookNetworkPolicy,
};

pub struct OutgoingWebhookNetwork {
    url_guard: NetworkWebhookUrlGuard,
    sender: Arc<HttpWebhookSender>,
    disable_after_failures: u32,
}

impl OutgoingWebhookNetwork {
    pub(crate) fn new(config: &WebhooksConfig) -> Result<Self, AppError> {
        let policy = WebhookNetworkPolicy::from_config(config);
        Ok(Self {
            url_guard: NetworkWebhookUrlGuard::new(policy),
            sender: Arc::new(HttpWebhookSender::new(policy, config.timeout)?),
            disable_after_failures: config.disable_after_failures,
        })
    }

    pub fn guard(&self) -> &NetworkWebhookUrlGuard {
        &self.url_guard
    }

    pub fn http_sender(&self) -> &Arc<HttpWebhookSender> {
        &self.sender
    }

    pub fn disable_after_failures(&self) -> u32 {
        self.disable_after_failures
    }
}
