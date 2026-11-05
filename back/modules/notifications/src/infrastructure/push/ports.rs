use sqlx::MySqlPool;

use crate::application::push::PushPorts;
use crate::infrastructure::push::{MySqlPushSubscriptionRepository, VaultVapidKeys};
use aphel_sdk::platform::infrastructure::auth::SystemClock;
use aphel_sdk::platform::infrastructure::platform::SharedSecretVault;

#[derive(Clone)]
pub struct MySqlPushPorts {
    subscriptions: MySqlPushSubscriptionRepository,
    keys: VaultVapidKeys,
}

impl MySqlPushPorts {
    pub fn new(pool: MySqlPool, secrets: SharedSecretVault) -> Self {
        Self {
            subscriptions: MySqlPushSubscriptionRepository::new(pool),
            keys: VaultVapidKeys::new(secrets),
        }
    }

    pub fn vapid_keys(&self) -> &VaultVapidKeys {
        &self.keys
    }
}

impl PushPorts for MySqlPushPorts {
    type Subscriptions = MySqlPushSubscriptionRepository;
    type Keys = VaultVapidKeys;
    type Clock = SystemClock;

    fn subscriptions(&self) -> &Self::Subscriptions {
        &self.subscriptions
    }

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
