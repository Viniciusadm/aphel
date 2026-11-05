use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::QUOTE;
use crate::domain::{
    QUOTE_APPROVALS_CONSUMER, QUOTE_DEAL_SYNC_JOB, QUOTES_EXPIRE_CRON, QUOTES_EXPIRE_JOB,
};
use crate::{sync, workflow};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;
use aphel_sdk::approvals::{APPROVAL_APPROVED, APPROVAL_CANCELLED, APPROVAL_REJECTED};
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler,
};
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) fn quotes_expire_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(QUOTES_EXPIRE_JOB, QUOTES_EXPIRE_CRON)
        .expect("the quotes expiration cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct QuoteDealSyncHandler {
    state: AppState,
}

#[async_trait]
impl JobHandler for QuoteDealSyncHandler {
    fn kind(&self) -> &str {
        QUOTE_DEAL_SYNC_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(quote_id) = payload
            .get("quote_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        sync::sync_deal(&ctx, quote_id).await
    }
}

pub(crate) struct QuotesExpireHandler {
    state: AppState,
}

#[async_trait]
impl JobHandler for QuotesExpireHandler {
    fn kind(&self) -> &str {
        QUOTES_EXPIRE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        let expired = workflow::expire_due(&ctx).await?;
        if expired > 0 {
            tracing::info!(expired, "quotes expired");
        }
        Ok(())
    }
}

pub(crate) struct QuoteApprovalsConsumer {
    state: AppState,
}

#[async_trait]
impl EventConsumer for QuoteApprovalsConsumer {
    fn name(&self) -> &str {
        QUOTE_APPROVALS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            APPROVAL_APPROVED | APPROVAL_REJECTED | APPROVAL_CANCELLED
        )
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        if event
            .record
            .as_ref()
            .is_none_or(|record| record.record_type != QUOTE)
        {
            return Ok(());
        }
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        workflow::apply_approval(&ctx, event).await
    }
}

pub(crate) fn quote_worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.state();
    WorkerParts::default()
        .handler(Arc::new(QuoteDealSyncHandler {
            state: state.clone(),
        }))
        .handler(Arc::new(QuotesExpireHandler {
            state: state.clone(),
        }))
        .consumer(Arc::new(QuoteApprovalsConsumer { state }))
        .schedule(quotes_expire_schedule())
}
