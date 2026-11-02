use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::clock::automation_clock;
use crate::model::{ActiveAutomation, AutomationRun};
use crate::runtime::{AutomationRuntime, RunJobPayload, publisher_scope, run_job};
use crate::store::{
    AutomationRunStore, AutomationStore, AutomationTimeStore, RunInsert, TimeCursor,
};
use aphel_contract_automation::application::context::TriggerContext;
use aphel_contract_automation::application::gateway::{AutomationGateway, SharedAutomationGateway};
use aphel_contract_automation::application::time::triggers::{
    CREATED_AT_FIELD, DueRule, ScheduleRule, TimePlan, parse_plan,
};
use aphel_contract_automation::domain::RunStatus;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{ArchivedFilter, RecordQuery, SortSpec};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{
    Actor, JobContext, JobHandler, JobQueue, RecordRef, RecordType,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use pipelines::rotting::{TIME_DEAL_ROTTING, rotting_catch_up_filter};

pub(crate) const TIME_SWEEP_JOB: &str = "automations.time_sweep";
pub(crate) const TIME_SWEEP_CRON: &str = "0 * * * * *";
pub(crate) const SWEEP_AUTOMATIONS_LIMIT: u32 = 50;
pub(crate) const SWEEP_PAGE_SIZE: u64 = 100;
pub(crate) const SWEEP_RECORDS_BUDGET: usize = 1000;

pub(crate) fn time_sweep_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(TIME_SWEEP_JOB, TIME_SWEEP_CRON)
        .expect("the time sweep cron expression is valid")
        .with_max_attempts(1)
}

fn rfc3339(instant: DateTime<Utc>) -> String {
    instant.to_rfc3339_opts(SecondsFormat::Secs, true)
}

struct SweepContext<'a> {
    active: &'a ActiveAutomation,
    ctx: &'a Ctx<'a>,
    scope: &'a AccessScope,
    gateway: &'a dyn AutomationGateway,
    fields: &'a FieldSet,
    timezone: Tz,
    now: DateTime<Utc>,
}

struct SweepTarget {
    scope: AccessScope,
    gateway: SharedAutomationGateway,
    fields: FieldSet,
    timezone: Tz,
}

impl SweepTarget {
    fn context<'a>(
        &'a self,
        active: &'a ActiveAutomation,
        ctx: &'a Ctx<'a>,
        now: DateTime<Utc>,
    ) -> SweepContext<'a> {
        SweepContext {
            active,
            ctx,
            scope: &self.scope,
            gateway: self.gateway.as_ref(),
            fields: &self.fields,
            timezone: self.timezone,
            now,
        }
    }
}

pub(crate) struct TimeTriggerSweep(pub(crate) Arc<AutomationRuntime>);

impl TimeTriggerSweep {
    pub(crate) async fn sweep(&self) -> Result<usize, AppError> {
        let system = AccessScope::system();
        let ctx = self.0.ctx(&system).await?;
        let automations = AutomationTimeStore::of(&ctx)
            .time_automations(SWEEP_AUTOMATIONS_LIMIT)
            .await?;
        let mut budget = SWEEP_RECORDS_BUDGET;
        let mut fired = 0;
        for active in &automations {
            if budget == 0 {
                break;
            }
            match self.sweep_automation(&ctx, active, &mut budget).await {
                Ok(count) => fired += count,
                Err(error) => tracing::warn!(
                    automation = %active.automation.id,
                    "time trigger sweep failed: {error}"
                ),
            }
        }
        Ok(fired)
    }

    async fn target(
        &self,
        ctx: &Ctx<'_>,
        active: &ActiveAutomation,
    ) -> Result<Option<SweepTarget>, AppError> {
        let Some(scope) = publisher_scope(ctx, &active.automation, &active.version).await? else {
            return Ok(None);
        };
        let gateway = self.0.gateways().gateway(None);
        let fields = gateway
            .fields(&scope, &active.automation.record_type)
            .await?;
        let timezone = gateway.timezone().await?;
        Ok(Some(SweepTarget {
            scope,
            gateway,
            fields,
            timezone,
        }))
    }

    async fn sweep_automation(
        &self,
        ctx: &Ctx<'_>,
        active: &ActiveAutomation,
        budget: &mut usize,
    ) -> Result<usize, AppError> {
        let automation = &active.automation;
        let version = &active.version;
        let time = AutomationTimeStore::of(ctx);
        let now = automation_clock().now();
        let stored = time
            .cursor(automation.id)
            .await?
            .filter(|cursor| cursor.version == version.version);
        let starting = stored.is_none();
        let mut cursor = stored.unwrap_or_else(|| {
            TimeCursor::start(automation.id, version.version, version.published_at)
        });
        let plan = parse_plan(&version.trigger).ok();
        let target = self.target(ctx, active).await?;
        let (Some(plan), Some(target)) = (plan, target) else {
            cursor.cursor_at = now;
            cursor.page_offset = 0;
            cursor.occurrence_at = None;
            time.save_cursor(&cursor, now).await?;
            return Ok(0);
        };
        let context = target.context(active, ctx, now);
        let mut fired = 0;
        if starting
            && version.trigger.kind == TIME_DEAL_ROTTING
            && let TimePlan::Due(rule) = &plan
        {
            fired += self.catch_up(&context, rule, None).await?;
        }
        fired += match plan {
            TimePlan::Due(rule) => self.due(&context, &rule, &mut cursor, budget).await?,
            TimePlan::Schedule(rule) => {
                self.scheduled(&context, &rule, &mut cursor, budget).await?
            }
        };
        time.save_cursor(&cursor, now).await?;
        Ok(fired)
    }

    pub(crate) async fn catch_up_rotting_stage(&self, stage_id: Uuid) -> Result<usize, AppError> {
        let system = AccessScope::system();
        let ctx = self.0.ctx(&system).await?;
        let automations = AutomationStore::of(&ctx)
            .active_for(RecordType::DEAL.as_str(), TIME_DEAL_ROTTING)
            .await?;
        let now = automation_clock().now();
        let mut fired = 0;
        for active in &automations {
            let Ok(TimePlan::Due(rule)) = parse_plan(&active.version.trigger) else {
                continue;
            };
            let Some(target) = self.target(&ctx, active).await? else {
                continue;
            };
            let context = target.context(active, &ctx, now);
            fired += self.catch_up(&context, &rule, Some(stage_id)).await?;
        }
        Ok(fired)
    }

    async fn catch_up(
        &self,
        context: &SweepContext<'_>,
        rule: &DueRule,
        stage_id: Option<Uuid>,
    ) -> Result<usize, AppError> {
        let record_type = context.active.automation.record_type.as_str();
        let mut offset = 0;
        let mut fired = 0;
        loop {
            let query = RecordQuery {
                filter: Some(rotting_catch_up_filter(stage_id)),
                sort: vec![SortSpec::asc(&rule.field)],
                archived: ArchivedFilter::Exclude,
                window: ListWindow {
                    offset,
                    limit: SWEEP_PAGE_SIZE,
                },
                ..RecordQuery::default()
            };
            let ids = context
                .gateway
                .find_records(context.scope, record_type, query)
                .await?
                .items;
            fired += self.fire_due(context, rule, &ids).await?;
            if (ids.len() as u64) < SWEEP_PAGE_SIZE {
                return Ok(fired);
            }
            offset += SWEEP_PAGE_SIZE;
        }
    }

    fn page(&self, cursor: &TimeCursor, budget: usize) -> ListWindow {
        ListWindow {
            offset: cursor.page_offset,
            limit: SWEEP_PAGE_SIZE.min(u64::try_from(budget).unwrap_or(SWEEP_PAGE_SIZE)),
        }
    }

    async fn due(
        &self,
        context: &SweepContext<'_>,
        rule: &DueRule,
        cursor: &mut TimeCursor,
        budget: &mut usize,
    ) -> Result<usize, AppError> {
        let Some(filter) = rule.window(
            context.fields,
            cursor.cursor_at,
            context.now,
            context.timezone,
        ) else {
            cursor.cursor_at = context.now;
            cursor.page_offset = 0;
            return Ok(0);
        };
        let window = self.page(cursor, *budget);
        let limit = window.limit;
        let query = RecordQuery {
            filter: Some(filter),
            sort: vec![SortSpec::asc(&rule.field)],
            archived: ArchivedFilter::Exclude,
            window,
            ..RecordQuery::default()
        };
        let record_type = context.active.automation.record_type.as_str();
        let ids = context
            .gateway
            .find_records(context.scope, record_type, query)
            .await?
            .items;
        let fired = self.fire_due(context, rule, &ids).await?;
        *budget = budget.saturating_sub(ids.len());
        if (ids.len() as u64) < limit {
            cursor.cursor_at = context.now;
            cursor.page_offset = 0;
        } else {
            cursor.page_offset += limit;
        }
        Ok(fired)
    }

    async fn fire_due(
        &self,
        context: &SweepContext<'_>,
        rule: &DueRule,
        ids: &[Uuid],
    ) -> Result<usize, AppError> {
        let record_type = context.active.automation.record_type.as_str();
        let mut fired = 0;
        for id in ids {
            let record = RecordRef::parse(record_type, &id.to_string())?;
            let Some(data) = context.gateway.load_record(context.scope, &record).await? else {
                continue;
            };
            let Some(due) = rule.value(&data, context.fields, context.timezone) else {
                continue;
            };
            let payload = json!({
                "field": due.field,
                "value": due.value,
                "due_at": rfc3339(due.instant),
                "fallback": due.field == CREATED_AT_FIELD && rule.field != CREATED_AT_FIELD,
            });
            let Some(dedup) = rule.dedup_value(&data, context.fields, &due) else {
                continue;
            };
            let key = format!("time:{dedup}:{id}");
            if self
                .fire(context, &record, key, due.instant, payload)
                .await?
            {
                fired += 1;
            }
        }
        Ok(fired)
    }

    async fn scheduled(
        &self,
        context: &SweepContext<'_>,
        rule: &ScheduleRule,
        cursor: &mut TimeCursor,
        budget: &mut usize,
    ) -> Result<usize, AppError> {
        let occurrence = match cursor.occurrence_at {
            Some(occurrence) => occurrence,
            None => match rule.latest_occurrence(cursor.cursor_at, context.now, context.timezone) {
                Some(occurrence) => {
                    cursor.page_offset = 0;
                    occurrence
                }
                None => {
                    cursor.cursor_at = context.now;
                    return Ok(0);
                }
            },
        };
        let window = self.page(cursor, *budget);
        let limit = window.limit;
        let query = RecordQuery {
            filter: rule.filter(),
            sort: vec![SortSpec::asc(CREATED_AT_FIELD)],
            archived: ArchivedFilter::Exclude,
            window,
            ..RecordQuery::default()
        };
        let record_type = context.active.automation.record_type.as_str();
        let ids = context
            .gateway
            .find_records(context.scope, record_type, query)
            .await?
            .items;
        let mut fired = 0;
        let occurred = rfc3339(occurrence);
        for id in &ids {
            let record = RecordRef::parse(record_type, &id.to_string())?;
            let key = format!("time:{occurred}:{id}");
            let payload = json!({ "occurrence_at": occurred, "cron": rule.expression });
            if self
                .fire(context, &record, key, occurrence, payload)
                .await?
            {
                fired += 1;
            }
        }
        *budget = budget.saturating_sub(ids.len());
        if (ids.len() as u64) < limit {
            cursor.cursor_at = occurrence;
            cursor.occurrence_at = None;
            cursor.page_offset = 0;
        } else {
            cursor.occurrence_at = Some(occurrence);
            cursor.page_offset += limit;
        }
        Ok(fired)
    }

    async fn fire(
        &self,
        context: &SweepContext<'_>,
        record: &RecordRef,
        trigger_key: String,
        occurred_at: DateTime<Utc>,
        payload: Value,
    ) -> Result<bool, AppError> {
        let automation = &context.active.automation;
        let version = &context.active.version;
        let mut trigger = TriggerContext::new(version.trigger.kind.clone());
        trigger.occurred_at = Some(occurred_at);
        trigger.actor_type = Some(Actor::SYSTEM.to_owned());
        trigger.payload = payload;
        let run = AutomationRun {
            id: Uuid::now_v7(),
            automation_id: automation.id,
            version: version.version,
            trigger_key: trigger_key.clone(),
            trigger_event_id: None,
            trigger_type: version.trigger.kind.clone(),
            record: record.clone(),
            status: RunStatus::Running,
            depth: 0,
            chain: Vec::new(),
            reason: None,
            error: None,
            attempts: 0,
            started_at: context.now,
            finished_at: None,
            resume_at: None,
            waiting_step_id: None,
            context: Some(trigger.clone()),
        };
        if let RunInsert::Existing(_) = AutomationRunStore::of(context.ctx).insert_run(&run).await?
        {
            return Ok(false);
        }
        let payload = RunJobPayload {
            automation_id: automation.id,
            version: version.version,
            trigger_key,
            record_type: record.record_type.as_str().to_owned(),
            record_id: record.record_id,
            context: trigger,
            causation: None,
        };
        context.ctx.jobs().enqueue_now(run_job(&payload)?).await?;
        Ok(true)
    }
}

#[async_trait]
impl JobHandler for TimeTriggerSweep {
    fn kind(&self) -> &str {
        TIME_SWEEP_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let fired = self.sweep().await?;
        if fired > 0 {
            tracing::info!(fired, "time triggers queued automation runs");
        }
        Ok(())
    }
}
