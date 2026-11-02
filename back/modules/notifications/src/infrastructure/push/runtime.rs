use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::push::NotificationPushHandler;
use crate::infrastructure::MySqlNotificationPorts;
use crate::infrastructure::push::{MySqlPushPorts, WebPushSender};
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::JobHandler;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::infrastructure::platform::SharedSecretVault;

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
