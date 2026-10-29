use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::notifications::push::NotificationPushHandler;
use crate::application::shared::JobHandler;
use crate::errors::AppError;
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::notifications::MySqlNotificationPorts;
use crate::infrastructure::notifications::push::{MySqlPushPorts, WebPushSender};
use crate::infrastructure::platform::SharedSecretVault;

pub(crate) fn push_delivery_handler(
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
