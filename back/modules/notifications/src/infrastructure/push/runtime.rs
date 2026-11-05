use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::push::NotificationPushHandler;
use crate::infrastructure::MySqlNotificationPorts;
use crate::infrastructure::push::{MySqlPushPorts, WebPushSender};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::JobHandler;
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;
use aphel_sdk::platform::infrastructure::platform::SharedSecretVault;

pub fn push_delivery_handler(
    pool: MySqlPool,
    secrets: SharedSecretVault,
    app_base_url: &str,
) -> Result<Arc<dyn JobHandler>, AppError> {
    let notifications = MySqlNotificationPorts::new(
        pool.clone(),
        Arc::new(RecordTypeRegistry::builtin()),
        Arc::new(MySqlJobQueue::new(pool.clone())),
    );
    let push = MySqlPushPorts::new(pool, secrets);
    let sender = WebPushSender::new(push.vapid_keys().clone(), app_base_url)?;
    Ok(Arc::new(NotificationPushHandler::new(
        notifications,
        push,
        sender,
    )))
}
