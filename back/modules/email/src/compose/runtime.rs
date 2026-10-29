use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::EMAIL_RECEIVED;
use crate::compose::{DeliveryOutcome, SEND_SCHEDULED_JOB};
use crate::service::{compose, tracking};
use crate::tracking::tracking_scope;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, Authorizer, build_access_scope};
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, JobContext, JobHandler};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::modules::WorkerParts;
use aphel_kernel::state::AppState;

pub(crate) const EMAIL_REPLY_CONSUMER: &str = "email.reply_detection";

pub(crate) fn worker(state: AppState) -> WorkerParts {
    WorkerParts::default()
        .handler(Arc::new(ScheduledEmailHandler {
            state: state.clone(),
        }))
        .consumer(Arc::new(EmailEngagementConsumer { state }))
}

fn uuid_field(payload: &Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

async fn member_scope(state: &AppState, user_id: &UserId) -> Result<Option<AccessScope>, AppError> {
    let access = MySqlAccessRepository::new(state.db()?)
        .load_access(user_id)
        .await?;
    Ok(access.map(|access| build_access_scope(user_id.clone(), access)))
}

pub(crate) struct ScheduledEmailHandler {
    state: AppState,
}

#[async_trait]
impl JobHandler for ScheduledEmailHandler {
    fn kind(&self) -> &str {
        SEND_SCHEDULED_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(email_id) = uuid_field(&payload, "email_id") else {
            return Ok(());
        };
        let author = payload
            .get("requested_by")
            .and_then(Value::as_str)
            .and_then(|value| UserId::new(value).ok());
        let author_scope = match &author {
            Some(author) => member_scope(&self.state, author).await?,
            None => None,
        };
        let Some(scope) =
            author_scope.filter(|scope| scope.require(&Permission::EMAILS_SEND).is_ok())
        else {
            let scope = tracking_scope();
            let ctx = Ctx::from_state(&self.state, &scope).await?;
            return compose::fail_scheduled(
                &ctx,
                email_id,
                "the author is no longer allowed to send e-mails",
            )
            .await;
        };
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        match compose::send_scheduled(&ctx, email_id, context.attempt < context.max_attempts)
            .await?
        {
            DeliveryOutcome::Retry(error) => Err(error),
            DeliveryOutcome::Sent(_) | DeliveryOutcome::Failed(_) | DeliveryOutcome::Skipped => {
                Ok(())
            }
        }
    }
}

pub(crate) struct EmailEngagementConsumer {
    state: AppState,
}

#[async_trait]
impl EventConsumer for EmailEngagementConsumer {
    fn name(&self) -> &str {
        EMAIL_REPLY_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == EMAIL_RECEIVED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(email_id) = uuid_field(&event.payload, "email_id") else {
            return Ok(());
        };
        let scope = tracking_scope();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        tracking::process_received(&ctx, email_id).await
    }
}
