pub(crate) mod http_sender;
pub mod network;
pub(crate) mod outbound;

pub(crate) use http_sender::HttpWebhookSender;
pub use network::{GuardedResolver, NetworkWebhookUrlGuard, WebhookNetworkPolicy};
pub use outbound::OutgoingWebhookNetwork;
