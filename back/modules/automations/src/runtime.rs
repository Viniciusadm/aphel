use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration as StdDuration, Instant};

use async_trait::async_trait;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::clock::automation_clock;
use crate::engine::{EngineInput, EngineMode, EngineOutcome, StepRecorder};
use crate::model::*;
use crate::service::template_scope;
use crate::store::{AutomationLocks, AutomationRunStore, AutomationStore, RunInsert};
use aphel_kernel::Ctx;
use aphel_kernel::api::automation_operations::automation_gateways;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::auth::UserRepository;
use aphel_kernel::application::automations::actions::ActionRegistry;
use aphel_kernel::application::automations::context::TriggerContext;
use aphel_kernel::application::automations::gateway::AutomationGatewayFactory;
use aphel_kernel::application::automations::triggers::{
    RECORD_CREATED, RECORD_DELETED, RECORD_UPDATED, TriggerRegistry,
};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{
    Actor, DomainEvent, EventCausation, EventConsumer, EventPublisher, Job, JobContext, JobHandler,
    JobQueue, RecordRef,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::automations::{AutomationStatus, RunStatus, SkipReason};
use aphel_kernel::domain::users::User;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::infrastructure::repositories::mysql_users::MySqlUserRepository;
use aphel_kernel::state::AppState;

pub(crate) const AUTOMATIONS_CONSUMER: &str = "automations.trigger";
pub(crate) const AUTOMATION_RUN_JOB: &str = "automation.run";
pub(crate) const AUTOMATION_RUN_MAX_ATTEMPTS: u32 = 5;
const ACTIVE_TRIGGERS_TTL: StdDuration = StdDuration::from_secs(300);
const AUTOMATION_EVENT_PREFIX: &str = "automation.";

struct LoadedTriggers {
    generation: u64,
    loaded_at: Instant,
    pairs: HashSet<(String, String)>,
}

#[derive(Default)]
pub(crate) struct ActiveTriggers {
    generation: AtomicU64,
    loading: AtomicBool,
    loaded: RwLock<Option<LoadedTriggers>>,
}

impl ActiveTriggers {
    pub(crate) fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    fn current(&self) -> Option<HashSet<(String, String)>> {
        let generation = self.generation.load(Ordering::SeqCst);
        let loaded = self.loaded.read().ok()?;
        loaded
            .as_ref()
            .filter(|loaded| {
                loaded.generation == generation && loaded.loaded_at.elapsed() < ACTIVE_TRIGGERS_TTL
            })
            .map(|loaded| loaded.pairs.clone())
    }

    fn refresh(self: &Arc<Self>, store: AutomationStore) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if self.loading.swap(true, Ordering::SeqCst) {
            return;
        }
        let cache = self.clone();
        handle.spawn(async move {
            let generation = cache.generation.load(Ordering::SeqCst);
            match store.active_trigger_pairs().await {
                Ok(pairs) => {
                    if let Ok(mut loaded) = cache.loaded.write() {
                        *loaded = Some(LoadedTriggers {
                            generation,
                            loaded_at: Instant::now(),
                            pairs: pairs.into_iter().collect(),
                        });
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "failed to load active automation triggers");
                }
            }
            cache.loading.store(false, Ordering::SeqCst);
        });
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RunJobPayload {
    pub(crate) automation_id: Uuid,
    pub(crate) version: u32,
    pub(crate) trigger_key: String,
    pub(crate) record_type: String,
    pub(crate) record_id: Uuid,
    pub(crate) context: TriggerContext,
    #[serde(default)]
    pub(crate) causation: Option<EventCausation>,
}

pub(crate) fn run_job(payload: &RunJobPayload) -> Result<Job, AppError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        AppError::Internal(format!("failed to serialize automation run: {error}"))
    })?;
    Ok(Job::new(AUTOMATION_RUN_JOB, value)
        .with_max_attempts(AUTOMATION_RUN_MAX_ATTEMPTS)
        .with_unique_key(format!(
            "automation:{}:{}",
            payload.automation_id, payload.trigger_key
        )))
}

pub(crate) fn resume_payload(run: &AutomationRun) -> RunJobPayload {
    RunJobPayload {
        automation_id: run.automation_id,
        version: run.version,
        trigger_key: run.trigger_key.clone(),
        record_type: run.record.record_type.as_str().to_owned(),
        record_id: run.record.record_id,
        context: run
            .context
            .clone()
            .unwrap_or_else(|| TriggerContext::new(run.trigger_type.clone())),
        causation: None,
    }
}

pub(crate) async fn publisher_scope(
    ctx: &Ctx<'_>,
    automation: &Automation,
    version: &AutomationVersion,
) -> Result<Option<AccessScope>, AppError> {
    let Some(publisher) = &version.published_by else {
        return Ok(None);
    };
    let Some(_user) = MySqlUserRepository::new(ctx.pool().clone())
        .find_by_id(publisher)
        .await?
        .filter(User::is_active)
    else {
        return Ok(None);
    };
    Ok(MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(publisher)
        .await?
        .map(|access| {
            build_access_scope(publisher.clone(), access)
                .through_automation(automation.id.to_string())
        }))
}

pub(crate) fn lock_name(automation_id: Uuid, record_id: Uuid) -> String {
    let digest = Sha256::digest(format!("{automation_id}:{record_id}").as_bytes());
    let hex: String = digest
        .iter()
        .take(20)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("aphel_auto_{hex}")
}

#[derive(Clone)]
pub(crate) struct AutomationRuntime {
    state: AppState,
    triggers: TriggerRegistry,
    actions: ActionRegistry,
    gateways: Arc<dyn AutomationGatewayFactory>,
    active_triggers: Arc<ActiveTriggers>,
}

impl AutomationRuntime {
    pub(crate) fn new(state: AppState) -> Self {
        Self {
            active_triggers: Arc::new(ActiveTriggers::default()),
            gateways: automation_gateways(state.clone()),
            state,
            triggers: TriggerRegistry::builtin(),
            actions: ActionRegistry::builtin(),
        }
    }

    pub(crate) async fn ctx<'s>(&self, scope: &'s AccessScope) -> Result<Ctx<'s>, AppError> {
        Ctx::from_state(&self.state, scope).await
    }

    pub(crate) fn gateways(&self) -> &dyn AutomationGatewayFactory {
        self.gateways.as_ref()
    }

    fn has_active_trigger(&self, event_type: &str) -> bool {
        let Some(pairs) = self.active_triggers.current() else {
            if let Ok(pool) = self.state.db() {
                self.active_triggers.refresh(AutomationStore::new(pool));
            }
            return true;
        };
        let record_event = event_type
            .split_once('.')
            .filter(|(_, suffix)| matches!(*suffix, "created" | "updated" | "deleted"));
        pairs.iter().any(|(record_type, trigger_key)| {
            let Some(trigger) = self.triggers.get(trigger_key) else {
                return false;
            };
            if !trigger.listens_to(event_type) {
                return false;
            }
            match (trigger_key.as_str(), record_event) {
                (RECORD_CREATED | RECORD_UPDATED | RECORD_DELETED, Some((prefix, _))) => {
                    prefix == record_type
                }
                _ => true,
            }
        })
    }
}

pub(crate) struct AutomationTriggerConsumer(pub(crate) Arc<AutomationRuntime>);

#[async_trait]
impl EventConsumer for AutomationTriggerConsumer {
    fn name(&self) -> &str {
        AUTOMATIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        if event_type.starts_with(AUTOMATION_EVENT_PREFIX) {
            self.0.active_triggers.invalidate();
        }
        self.0.triggers.listens_to(event_type) && self.0.has_active_trigger(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = self.0.ctx(&scope).await?;
        let store = AutomationStore::of(&ctx);
        for (trigger, candidate) in self.0.triggers.candidates(event) {
            let automations = store
                .active_for(candidate.record.record_type.as_str(), trigger.key())
                .await?;
            for active in automations {
                if event.occurred_at < active.version.published_at
                    || !trigger.accepts(&active.version.trigger, &candidate)
                {
                    continue;
                }
                let payload = RunJobPayload {
                    automation_id: active.automation.id,
                    version: active.version.version,
                    trigger_key: format!("{}:{}", event.id, candidate.record.record_id),
                    record_type: candidate.record.record_type.as_str().to_owned(),
                    record_id: candidate.record.record_id,
                    context: candidate.context.clone(),
                    causation: event.causation.clone(),
                };
                ctx.jobs().enqueue_now(run_job(&payload)?).await?;
            }
        }
        Ok(())
    }
}

struct RunStepRecorder<'a> {
    runs: &'a AutomationRunStore,
}

#[async_trait]
impl StepRecorder for RunStepRecorder<'_> {
    async fn record(&self, step: &RunStep) -> Result<(), AppError> {
        self.runs.save_step(step).await
    }
}

pub(crate) struct AutomationRunHandler(pub(crate) Arc<AutomationRuntime>);

impl AutomationRunHandler {
    async fn finish(
        &self,
        ctx: &Ctx<'_>,
        run: &mut AutomationRun,
        status: RunStatus,
        reason: Option<SkipReason>,
        error: Option<String>,
    ) -> Result<(), AppError> {
        run.status = status;
        run.reason = reason;
        run.error = error.map(|message| truncate_error(&message));
        run.finished_at = Some(automation_clock().now());
        run.resume_at = None;
        run.waiting_step_id = None;
        AutomationRunStore::of(ctx).update_run(run).await
    }

    async fn skip(
        &self,
        ctx: &Ctx<'_>,
        automation: &Automation,
        run: &mut AutomationRun,
        reason: SkipReason,
    ) -> Result<(), AppError> {
        self.finish(ctx, run, RunStatus::Skipped, Some(reason), None)
            .await?;
        if !reason.is_loop_protection() {
            return Ok(());
        }
        let since = automation_clock().now() - Duration::minutes(SKIP_NOTICE_WINDOW_MINUTES);
        if AutomationRunStore::of(ctx)
            .recently_skipped(automation.id, reason, since, run.id)
            .await?
        {
            return Ok(());
        }
        let recipients: Vec<String> = automation
            .created_by
            .iter()
            .map(|user| user.as_str().to_owned())
            .collect();
        let event = DomainEvent::new(
            Actor::System,
            AUTOMATION_RUN_SKIPPED,
            automation_clock().now(),
        )
        .with_record(RecordRef::new(AUTOMATION_RECORD_TYPE, automation.id))
        .with_payload(json!({
            "automation_id": automation.id.to_string(),
            "name": automation.name,
            "run_id": run.id.to_string(),
            "reason": reason.as_str(),
            "depth": run.depth,
            "record_type": run.record.record_type.as_str(),
            "record_id": run.record.record_id.to_string(),
            "recipients": recipients,
        }));
        let mut tx = ctx.begin().await?;
        ctx.events().publish(&mut tx, event).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn protection(
        &self,
        ctx: &Ctx<'_>,
        automation: &Automation,
        run: &AutomationRun,
    ) -> Result<Option<SkipReason>, AppError> {
        if automation.status != AutomationStatus::Active {
            return Ok(Some(SkipReason::AutomationInactive));
        }
        if run.depth > MAX_CHAIN_DEPTH {
            return Ok(Some(SkipReason::MaxDepth));
        }
        if run.chain.contains(&automation.id) {
            return Ok(Some(SkipReason::CausationCycle));
        }
        let since = automation_clock().now() - Duration::minutes(RUNS_PER_RECORD_WINDOW_MINUTES);
        let recent = AutomationRunStore::of(ctx)
            .count_recent(automation.id, run.record.record_id, since)
            .await?;
        if recent > RUNS_PER_RECORD_LIMIT {
            return Ok(Some(SkipReason::RateLimited));
        }
        Ok(None)
    }

    async fn load_run(
        &self,
        ctx: &Ctx<'_>,
        payload: &RunJobPayload,
        record: &RecordRef,
    ) -> Result<AutomationRun, AppError> {
        let runs = AutomationRunStore::of(ctx);
        if let Some(run) = runs
            .find_by_trigger(payload.automation_id, &payload.trigger_key)
            .await?
        {
            return Ok(run);
        }
        let (depth, chain) = match &payload.causation {
            Some(causation) => (causation.depth + 1, causation.chain.clone()),
            None => (0, Vec::new()),
        };
        let run = AutomationRun {
            id: Uuid::now_v7(),
            automation_id: payload.automation_id,
            version: payload.version,
            trigger_key: payload.trigger_key.clone(),
            trigger_event_id: payload.context.event_id,
            trigger_type: payload.context.trigger_type.clone(),
            record: record.clone(),
            status: RunStatus::Running,
            depth,
            chain,
            reason: None,
            error: None,
            attempts: 0,
            started_at: automation_clock().now(),
            finished_at: None,
            resume_at: None,
            waiting_step_id: None,
            context: Some(payload.context.clone()),
        };
        match runs.insert_run(&run).await? {
            RunInsert::Created => Ok(run),
            RunInsert::Existing(existing) => Ok(*existing),
        }
    }

    async fn execute(
        &self,
        ctx: &Ctx<'_>,
        context: &JobContext,
        payload: &RunJobPayload,
    ) -> Result<(), AppError> {
        let store = AutomationStore::of(ctx);
        let Some(automation) = store.find(payload.automation_id).await? else {
            return Ok(());
        };
        let Some(version) = store.find_version(automation.id, payload.version).await? else {
            return Ok(());
        };
        let record = RecordRef::parse(&payload.record_type, &payload.record_id.to_string())?;
        let mut run = self.load_run(ctx, payload, &record).await?;
        if run.status.is_finished() || run.status == RunStatus::Waiting {
            return Ok(());
        }
        let Some(lock) = AutomationLocks::of(ctx)
            .acquire(
                &lock_name(automation.id, record.record_id),
                LOCK_WAIT_SECONDS,
            )
            .await?
        else {
            return Err(AppError::Conflict(
                "another run of this automation is processing the same record".to_owned(),
            ));
        };
        let result = self
            .locked(
                ctx,
                context,
                payload,
                &automation,
                &version,
                &record,
                &mut run,
            )
            .await;
        lock.release().await?;
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn locked(
        &self,
        ctx: &Ctx<'_>,
        context: &JobContext,
        payload: &RunJobPayload,
        automation: &Automation,
        version: &AutomationVersion,
        record: &RecordRef,
        run: &mut AutomationRun,
    ) -> Result<(), AppError> {
        let system = ctx.scope;
        let runs = AutomationRunStore::of(ctx);
        if run.attempts == 0
            && let Some(reason) = self.protection(ctx, automation, run).await?
        {
            return self.skip(ctx, automation, run, reason).await;
        }
        run.attempts += 1;
        runs.update_run(run).await?;
        let Some(scope) = publisher_scope(ctx, automation, version).await? else {
            return self
                .skip(ctx, automation, run, SkipReason::PublisherUnavailable)
                .await;
        };
        let mut chain = run.chain.clone();
        chain.push(automation.id);
        let gateway = self.0.gateways().gateway(Some(EventCausation {
            run_id: run.id,
            automation_id: automation.id,
            depth: run.depth,
            chain,
        }));
        let fields = gateway.fields(&scope, &automation.record_type).await?;
        let loaded = gateway.load_record(&scope, record).await?;
        let record_available = loaded.is_some();
        if !record_available && payload.context.trigger_type != RECORD_DELETED {
            let reason = match gateway.load_record(system, record).await? {
                Some(_) => SkipReason::RecordNotAccessible,
                None => SkipReason::RecordNotFound,
            };
            return self.skip(ctx, automation, run, reason).await;
        }
        let record_data = loaded.unwrap_or_else(|| json!({ "id": record.record_id.to_string() }));
        let templates = template_scope(
            automation,
            run.id,
            run.depth,
            &record_data,
            &payload.context,
            &fields,
            gateway.timezone().await?,
            automation_clock().now(),
        );
        let previous = runs.steps(run.id).await?;
        let run_ctx = ctx.with_scope(&scope);
        let input = EngineInput {
            ctx: &run_ctx,
            gateway: gateway.as_ref(),
            actions: &self.0.actions,
            clock: automation_clock(),
            definition: &version.definition,
            record,
            record_data: &record_data,
            record_available,
            fields: &fields,
            templates: &templates,
            automation_id: automation.id,
            run_id: run.id,
            mode: EngineMode::Live,
            previous: &previous,
        };
        let recorder = RunStepRecorder { runs: &runs };
        let (outcome, _steps) = input.run(&recorder).await?;
        match outcome {
            EngineOutcome::Completed => {
                self.finish(ctx, run, RunStatus::Succeeded, None, None)
                    .await
            }
            EngineOutcome::Waiting { step_id, resume_at } => {
                run.status = RunStatus::Waiting;
                run.resume_at = Some(resume_at);
                run.waiting_step_id = Some(step_id);
                run.error = None;
                runs.update_run(run).await
            }
            EngineOutcome::Failed { error, retry }
                if retry && context.attempt < context.max_attempts =>
            {
                run.error = Some(truncate_error(&error));
                runs.update_run(run).await?;
                Err(AppError::Persistence(format!(
                    "automation step failed, retrying: {error}"
                )))
            }
            EngineOutcome::Failed { error, .. } => {
                self.finish(ctx, run, RunStatus::Failed, None, Some(error))
                    .await
            }
        }
    }
}

#[async_trait]
impl JobHandler for AutomationRunHandler {
    fn kind(&self) -> &str {
        AUTOMATION_RUN_JOB
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let payload: RunJobPayload = serde_json::from_value(payload)
            .map_err(|error| AppError::Validation(format!("invalid automation run: {error}")))?;
        let system = AccessScope::system();
        let ctx = self.0.ctx(&system).await?;
        let result = self.execute(&ctx, context, &payload).await;
        if let Err(error) = &result
            && context.attempt >= context.max_attempts
            && let Some(mut run) = AutomationRunStore::of(&ctx)
                .find_by_trigger(payload.automation_id, &payload.trigger_key)
                .await?
                .filter(|run| !run.status.is_finished())
        {
            self.finish(
                &ctx,
                &mut run,
                RunStatus::Failed,
                None,
                Some(crate::engine::step_error(error)),
            )
            .await?;
        }
        result
    }
}
