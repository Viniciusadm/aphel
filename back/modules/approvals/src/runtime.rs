use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::domain::{
    APPROVAL_OVERDUE, APPROVALS_CLEANUP_CONSUMER, APPROVALS_OVERDUE_CRON, APPROVALS_OVERDUE_JOB,
    DELETED_RECORD_REASON,
};
use crate::engine::{self, RequestEvent};
use crate::store::ApprovalStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{
    DomainEvent, EventConsumer, FieldChange, JobContext, JobHandler,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;

const OVERDUE_BATCH: u32 = 200;

pub(crate) fn approvals_overdue_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(APPROVALS_OVERDUE_JOB, APPROVALS_OVERDUE_CRON)
        .expect("the approvals overdue cron expression is valid")
        .with_max_attempts(1)
}

fn is_record_deletion(event_type: &str) -> bool {
    event_type
        .split_once('.')
        .is_some_and(|(prefix, action)| action == "deleted" && !prefix.starts_with("approval"))
}

#[derive(Clone)]
pub(crate) struct ApprovalRuntime {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl ApprovalRuntime {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }

    async fn ctx<'a>(&self, scope: &'a AccessScope) -> Result<Ctx<'a>, AppError> {
        Ok(Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        ))
    }
}

pub(crate) struct ApprovalCleanupConsumer {
    runtime: ApprovalRuntime,
}

impl ApprovalCleanupConsumer {
    pub(crate) fn shared(runtime: ApprovalRuntime) -> Arc<dyn EventConsumer> {
        Arc::new(Self { runtime })
    }
}

#[async_trait]
impl EventConsumer for ApprovalCleanupConsumer {
    fn name(&self) -> &str {
        APPROVALS_CLEANUP_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        is_record_deletion(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = &event.record else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = self.runtime.ctx(&scope).await?;
        let mut tx = ctx.begin().await?;
        engine::cancel_open(&ctx, &mut tx, record, DELETED_RECORD_REASON).await?;
        Ok(tx.commit().await?)
    }
}

pub(crate) struct ApprovalOverdueHandler {
    runtime: ApprovalRuntime,
}

impl ApprovalOverdueHandler {
    pub(crate) fn shared(runtime: ApprovalRuntime) -> Arc<dyn JobHandler> {
        Arc::new(Self { runtime })
    }

    async fn flag_overdue(&self) -> Result<u32, AppError> {
        let scope = AccessScope::system();
        let ctx = self.runtime.ctx(&scope).await?;
        let store = ApprovalStore::of(&ctx);
        let now = ctx.now();
        let mut notified = 0;
        for record in store.overdue(now, OVERDUE_BATCH).await? {
            let request = &record.request;
            let waiting: Vec<UserId> = record
                .step_approvers(request.current_step)
                .filter(|approver| approver.decision.is_none())
                .map(|approver| approver.user_id.clone())
                .collect();
            let mut tx = ctx.begin().await?;
            if !store.mark_overdue(&mut tx, request.id, now).await? {
                drop(tx);
                continue;
            }
            engine::publish(
                &ctx,
                &mut tx,
                RequestEvent::new(APPROVAL_OVERDUE, request)
                    .step(request.current_step)
                    .approvers(&waiting)
                    .changes(vec![FieldChange::new("overdue", Value::Null, json!(true))]),
            )
            .await?;
            tx.commit().await?;
            notified += 1;
        }
        Ok(notified)
    }
}

#[async_trait]
impl JobHandler for ApprovalOverdueHandler {
    fn kind(&self) -> &str {
        APPROVALS_OVERDUE_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let notified = self.flag_overdue().await?;
        if notified > 0 {
            tracing::info!(notified, "overdue approval requests flagged");
        }
        Ok(())
    }
}
