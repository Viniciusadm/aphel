use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::notifications::push::PushSubscriptionInput;

#[derive(Debug, Serialize, ToSchema)]
pub struct PushPublicKeyResponse {
    pub public_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushSubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePushSubscriptionRequest {
    pub endpoint: String,
    pub keys: PushSubscriptionKeys,
}

impl CreatePushSubscriptionRequest {
    pub fn into_input(self, user_agent: Option<String>) -> PushSubscriptionInput {
        PushSubscriptionInput {
            endpoint: self.endpoint,
            p256dh: self.keys.p256dh,
            auth: self.keys.auth,
            user_agent,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemovePushSubscriptionRequest {
    pub endpoint: String,
}
