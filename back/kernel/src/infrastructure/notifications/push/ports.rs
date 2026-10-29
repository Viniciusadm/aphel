use sqlx::MySqlPool;

use crate::application::notifications::push::PushPorts;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::notifications::push::{MySqlPushSubscriptionRepository, VaultVapidKeys};
use crate::infrastructure::platform::SharedSecretVault;

#[derive(Clone)]
pub(crate) struct MySqlPushPorts {
    subscriptions: MySqlPushSubscriptionRepository,
    keys: VaultVapidKeys,
}

impl MySqlPushPorts {
    pub(crate) fn new(pool: MySqlPool, secrets: SharedSecretVault) -> Self {
        Self {
            subscriptions: MySqlPushSubscriptionRepository::new(pool),
            keys: VaultVapidKeys::new(secrets),
        }
    }

    pub(crate) fn vapid_keys(&self) -> &VaultVapidKeys {
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
