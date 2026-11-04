use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::platform::SharedJobQueue;
use aphel_sdk::notifications::{
    DispatchOutcome, NotificationDelivery, NotificationDirectory, NotificationIntent,
};
use sqlx::MySqlPool;
use std::sync::Arc;

pub(crate) struct Delivery;

#[async_trait::async_trait]
impl NotificationDelivery for Delivery {
    fn directory(
        &self,
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
    ) -> Arc<dyn NotificationDirectory> {
        Arc::new(crate::infrastructure::MySqlNotificationDirectory::new(
            pool, registry,
        ))
    }

    async fn dispatch(
        &self,
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        jobs: SharedJobQueue,
        intent: NotificationIntent,
    ) -> Result<DispatchOutcome, AppError> {
        let ports = crate::infrastructure::MySqlNotificationPorts::new(pool, registry, jobs);
        crate::application::dispatch(&ports, intent).await
    }
}
