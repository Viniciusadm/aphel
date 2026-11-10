use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::domain::custom::{
    DeliveryRequest, REPORT_DELIVERY_JOB, REPORT_SCHEDULES_DISPATCH_CRON,
    REPORT_SCHEDULES_DISPATCH_JOB,
};
use crate::service::delivery;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::exports::system_scope;
use aphel_sdk::platform::application::custom_objects::RecordTypeCatalog;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::mail::SharedMailer;
use aphel_sdk::platform::application::organization_profile::PublicBaseUrl;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::audit::MySqlAuditLog;
use aphel_sdk::platform::infrastructure::events::MySqlEventPublisher;

pub(crate) fn report_schedules_dispatch_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(
        REPORT_SCHEDULES_DISPATCH_JOB,
        REPORT_SCHEDULES_DISPATCH_CRON,
    )
    .expect("the report schedules dispatch cron expression is valid")
    .with_max_attempts(2)
}

#[derive(Clone)]
pub(crate) struct ReportsRuntime {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl ReportsRuntime {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }

    async fn ctx<'s>(&self, scope: &'s AccessScope) -> Result<Ctx<'s>, AppError> {
        Ok(Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        ))
    }
}

pub(crate) struct ReportScheduleDispatchHandler(pub(crate) ReportsRuntime);

#[async_trait]
impl JobHandler for ReportScheduleDispatchHandler {
    fn kind(&self) -> &str {
        REPORT_SCHEDULES_DISPATCH_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        delivery::dispatch_due(&ctx).await.map(|_| ())
    }
}

pub(crate) struct ReportDeliveryHandler {
    pub(crate) runtime: ReportsRuntime,
    pub(crate) mailer: SharedMailer,
    pub(crate) public_url: PublicBaseUrl,
}

#[async_trait]
impl JobHandler for ReportDeliveryHandler {
    fn kind(&self) -> &str {
        REPORT_DELIVERY_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let request = DeliveryRequest::from_payload(&payload)?;
        let system = system_scope(request.user_id.clone())?;
        let ctx = self.runtime.ctx(&system).await?;
        delivery::deliver(
            &ctx,
            &request,
            self.mailer.as_ref(),
            &self.public_url.app_root(),
            context.attempt >= context.max_attempts,
        )
        .await
        .map(|_| ())
    }
}
