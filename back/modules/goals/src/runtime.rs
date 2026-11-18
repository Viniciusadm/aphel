use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler,
};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;

pub(crate) const GOALS_PURGE_ORPHANS_JOB: &str = "goals.purge_orphans";
pub(crate) const GOALS_PURGE_ORPHANS_CRON: &str = "0 20 * * * *";

pub(crate) const GOALS_OWNER_REMOVED_CONSUMER: &str = "goals.owner_removed";
const OWNER_REMOVAL_EVENTS: [&str; 2] = ["team.deleted", "member.removed"];

pub(crate) fn purge_orphans_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(GOALS_PURGE_ORPHANS_JOB, GOALS_PURGE_ORPHANS_CRON)
        .expect("the goals orphan purge cron expression is valid")
        .with_max_attempts(2)
}

#[derive(Clone)]
pub(crate) struct PurgeOrphanGoalsHandler {
    pub(crate) pool: Db,
    pub(crate) catalog: Arc<RecordTypeCatalog>,
}

impl PurgeOrphanGoalsHandler {
    async fn purge(&self) -> Result<usize, AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_parts(
            &scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        );
        service::purge_orphans(&ctx).await
    }
}

#[async_trait]
impl JobHandler for PurgeOrphanGoalsHandler {
    fn kind(&self) -> &str {
        GOALS_PURGE_ORPHANS_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        self.purge().await.map(|_| ())
    }
}

pub(crate) struct GoalOwnerRemovedConsumer(pub(crate) PurgeOrphanGoalsHandler);

#[async_trait]
impl EventConsumer for GoalOwnerRemovedConsumer {
    fn name(&self) -> &str {
        GOALS_OWNER_REMOVED_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        OWNER_REMOVAL_EVENTS.contains(&event_type)
    }

    async fn consume(&self, _event: &DomainEvent) -> Result<(), AppError> {
        self.0.purge().await.map(|_| ())
    }
}
