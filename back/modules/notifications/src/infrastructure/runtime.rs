use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::application::{
    DailyDigestHandler, NOTIFICATION_EMAIL_JOB, NotificationEmailHandler,
    NotificationEventConsumer, daily_digest_schedule,
};
use crate::infrastructure::MySqlNotificationPorts;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::mail::{
    OrganizationMailer, SharedOrganizationMailerResolver,
};
use aphel_sdk::platform::application::shared::{EventConsumer, JobContext, JobHandler};
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;

pub struct NotificationWorkerParts {
    pub consumer: Arc<dyn EventConsumer>,
    pub handlers: Vec<Arc<dyn JobHandler>>,
    pub schedules: Vec<ScheduledJobDefinition>,
}

pub fn notification_worker_parts(
    pool: MySqlPool,
    mailers: SharedOrganizationMailerResolver,
    app_base_url: impl Into<aphel_sdk::platform::application::organization_profile::PublicBaseUrl>,
) -> NotificationWorkerParts {
    let purge = crate::infrastructure::notifications_purge_handler(pool.clone());
    let ports = MySqlNotificationPorts::new(
        pool.clone(),
        Arc::new(RecordTypeRegistry::builtin()),
        Arc::new(MySqlJobQueue::new(pool)),
    );
    NotificationWorkerParts {
        consumer: Arc::new(NotificationEventConsumer::new(ports.clone())),
        handlers: vec![
            Arc::new(OrganizationNotificationEmailHandler {
                ports: ports.clone(),
                mailers,
                app_base_url: app_base_url.into(),
            }),
            Arc::new(DailyDigestHandler::new(ports)),
            purge,
        ],
        schedules: vec![
            daily_digest_schedule(),
            crate::infrastructure::notifications_purge_schedule(),
        ],
    }
}

struct OrganizationNotificationEmailHandler {
    ports: MySqlNotificationPorts,
    mailers: SharedOrganizationMailerResolver,
    app_base_url: aphel_sdk::platform::application::organization_profile::PublicBaseUrl,
}

#[async_trait]
impl JobHandler for OrganizationNotificationEmailHandler {
    fn kind(&self) -> &str {
        NOTIFICATION_EMAIL_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let mailer = OrganizationMailer::new(self.mailers.clone());
        NotificationEmailHandler::new(self.ports.clone(), mailer, self.app_base_url.clone())
            .handle(context, payload)
            .await
    }
}
