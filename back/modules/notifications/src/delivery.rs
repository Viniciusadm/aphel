use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::{
    DispatchOutcome, NotificationDelivery, NotificationDirectory, NotificationIntent,
};
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::infrastructure::platform::SharedJobQueue;
use std::sync::Arc;

pub(crate) struct Delivery;

#[async_trait::async_trait]
impl NotificationDelivery for Delivery {
    fn directory(
        &self,
        pool: Db,
        registry: Arc<RecordTypeRegistry>,
    ) -> Arc<dyn NotificationDirectory> {
        Arc::new(crate::infrastructure::MySqlNotificationDirectory::new(
            pool, registry,
        ))
    }

    async fn dispatch(
        &self,
        pool: Db,
        registry: Arc<RecordTypeRegistry>,
        jobs: SharedJobQueue,
        intent: NotificationIntent,
    ) -> Result<DispatchOutcome, AppError> {
        let ports = crate::infrastructure::MySqlNotificationPorts::new(pool, registry, jobs);
        crate::application::dispatch(&ports, intent).await
    }
}
