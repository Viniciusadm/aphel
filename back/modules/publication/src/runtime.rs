use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use serde_json::Value;
use uuid::Uuid;

use crate::activation;
use crate::domain::{
    ACTIVATE_JOB, APPROVAL_REJECTED, APPROVAL_WITHDRAWN, APPROVALS_CONSUMER, COLLECT_CRON,
    COLLECT_JOB, RETAIN_DAYS, error_code,
};
use crate::store;
use aphel_contract_publication::{ACTIVATE_MOMENT, MODULE, RequestStatus};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::approvals::{
    APPROVAL_APPROVED, APPROVAL_CANCELLED, APPROVAL_REJECTED as APPROVAL_REJECTED_EVENT,
    ApprovalMoment, approval_gate,
};
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler,
};
use aphel_sdk::platform::domain::access::AccessScope;

const COLLECT_BATCH: u32 = 500;

pub(crate) fn module_scope() -> AccessScope {
    AccessScope::system().acting_as("module", MODULE)
}

pub(crate) fn collect_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(COLLECT_JOB, COLLECT_CRON)
        .expect("the publication collect cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct ActivateHandler;

impl ActivateHandler {
    pub(crate) fn shared() -> Arc<dyn JobHandler> {
        Arc::new(Self)
    }
}

#[async_trait]
impl JobHandler for ActivateHandler {
    fn kind(&self) -> &str {
        ACTIVATE_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(request_id) = payload
            .get("request_id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
        else {
            return Ok(());
        };
        match activation::process(request_id, None).await {
            Ok(_) => Ok(()),
            Err(error) if context.attempt >= context.max_attempts => {
                tracing::error!(%request_id, %error, "publication failed after its last attempt");
                activation::settle(request_id, RequestStatus::Failed, error_code(&error)).await
            }
            Err(error) => Err(error),
        }
    }
}

pub(crate) struct CollectHandler;

impl CollectHandler {
    pub(crate) fn shared() -> Arc<dyn JobHandler> {
        Arc::new(Self)
    }
}

#[async_trait]
impl JobHandler for CollectHandler {
    fn kind(&self) -> &str {
        COLLECT_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let state = aphel_sdk::runtime()?;
        let scope = module_scope();
        let ctx = Ctx::from_state(state.as_ref(), &scope).await?;
        let before = ctx.now() - Duration::days(RETAIN_DAYS);
        let gate = approval_gate(&ctx);
        let moment = ApprovalMoment::from_static(ACTIVATE_MOMENT);
        let mut removed = 0;
        for (record, revision_id) in
            store::collectable_revisions(ctx.pool(), before, COLLECT_BATCH).await?
        {
            if let Some(gate) = &gate
                && gate
                    .retained_targets(&ctx, &record, &moment)
                    .await?
                    .contains(&revision_id)
            {
                continue;
            }
            let mut tx = ctx.begin().await?;
            store::delete_revision(&mut tx, revision_id).await?;
            tx.commit().await?;
            removed += 1;
        }
        if removed > 0 {
            tracing::info!(removed, "old publication revisions collected");
        }
        Ok(())
    }
}

pub(crate) struct ApprovalConsumer;

impl ApprovalConsumer {
    pub(crate) fn shared() -> Arc<dyn EventConsumer> {
        Arc::new(Self)
    }
}

#[async_trait]
impl EventConsumer for ApprovalConsumer {
    fn name(&self) -> &str {
        APPROVALS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        [
            APPROVAL_APPROVED,
            APPROVAL_REJECTED_EVENT,
            APPROVAL_CANCELLED,
        ]
        .contains(&event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        if event.payload.get("moment").and_then(Value::as_str) != Some(ACTIVATE_MOMENT) {
            return Ok(());
        }
        let (Some(record), Some(revision_id)) = (
            event.record.as_ref(),
            event
                .payload
                .get("target_id")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok()),
        ) else {
            return Ok(());
        };
        let state = aphel_sdk::runtime()?;
        let scope = module_scope();
        let ctx = Ctx::from_state(state.as_ref(), &scope).await?;
        if event.event_type != APPROVAL_APPROVED
            && let Some(gate) = approval_gate(&ctx)
            && gate
                .retained_targets(&ctx, record, &ApprovalMoment::from_static(ACTIVATE_MOMENT))
                .await?
                .contains(&revision_id)
        {
            return Ok(());
        }
        let mut tx = ctx.begin().await?;
        let waiting = store::awaiting_for_revision(&mut tx, record, revision_id).await?;
        let mut rejected = Vec::new();
        for mut request in waiting {
            if event.event_type != APPROVAL_APPROVED {
                rejected.push(request.id);
                continue;
            }
            request.status = if request.run_at.is_some_and(|at| at > ctx.now()) {
                RequestStatus::Scheduled
            } else {
                RequestStatus::Ready
            };
            store::save_request(&mut tx, &request).await?;
            crate::service::enqueue(&ctx, &mut tx, &request).await?;
        }
        tx.commit().await?;
        let reason = if event.event_type == APPROVAL_REJECTED_EVENT {
            APPROVAL_REJECTED
        } else {
            APPROVAL_WITHDRAWN
        };
        for request_id in rejected {
            activation::settle(request_id, RequestStatus::Blocked, reason).await?;
        }
        Ok(())
    }
}
