use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::application::fields::RecordTypeRegistry;
use crate::application::integrations::{OrganizationMailer, SharedOrganizationMailerResolver};
use crate::application::jobs::ScheduledJobDefinition;
use crate::application::notifications::{
    DailyDigestHandler, NOTIFICATION_EMAIL_JOB, NotificationEmailHandler,
    NotificationEventConsumer, daily_digest_schedule,
};
use crate::application::shared::{EventConsumer, JobContext, JobHandler};
use crate::errors::AppError;
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::notifications::MySqlNotificationPorts;

pub(crate) struct NotificationWorkerParts {
    pub(crate) consumer: Arc<dyn EventConsumer>,
    pub(crate) handlers: Vec<Arc<dyn JobHandler>>,
    pub(crate) schedules: Vec<ScheduledJobDefinition>,
}

pub(crate) fn notification_worker_parts(
    pool: MySqlPool,
    mailers: SharedOrganizationMailerResolver,
    app_base_url: impl Into<crate::application::organizations::PublicBaseUrl>,
) -> NotificationWorkerParts {
    let purge = crate::infrastructure::notifications::notifications_purge_handler(pool.clone());
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
            crate::infrastructure::notifications::notifications_purge_schedule(),
        ],
    }
}

struct OrganizationNotificationEmailHandler {
    ports: MySqlNotificationPorts,
    mailers: SharedOrganizationMailerResolver,
    app_base_url: crate::application::organizations::PublicBaseUrl,
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
