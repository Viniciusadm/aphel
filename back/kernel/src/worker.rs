use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::MySqlPool;

use crate::application::custom_objects::RecordTypeCatalog;
use crate::application::fields::RecordTypeRegistry;
use crate::application::jobs::{EventConsumerRegistry, JobRegistry, ScheduledJobDefinition};
use crate::application::shared::{EventConsumer, JobHandler};
use crate::config::WorkerConfig;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::MySqlCustomSchemaSource;
use crate::infrastructure::jobs::{
    EventConsumeHandler, JobCleanupHandler, JobScheduler, JobWorker, OutboxDispatchHandler,
    Shutdown, WorkerSettings, jobs_cleanup_schedule, outbox_dispatch_schedule,
};
use crate::infrastructure::noop::UnavailableSecretVault;
use crate::infrastructure::platform::SharedSecretVault;
use crate::modules::{App, WorkerEnv};

pub struct WorkerRuntime {
    pool: MySqlPool,
    settings: WorkerSettings,
    scheduler_interval: Duration,
    handlers: JobRegistry,
    consumers: EventConsumerRegistry,
    schedules: Vec<ScheduledJobDefinition>,
    secrets: SharedSecretVault,
    record_types: Arc<RecordTypeCatalog>,
}

impl WorkerRuntime {
    pub fn new(pool: MySqlPool, config: &WorkerConfig) -> Self {
        let record_types = Arc::new(RecordTypeCatalog::new(
            RecordTypeRegistry::builtin(),
            Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
        ));
        Self {
            pool,
            settings: WorkerSettings::from_config(config),
            scheduler_interval: config.scheduler_interval,
            handlers: JobRegistry::new(),
            consumers: EventConsumerRegistry::new(),
            schedules: vec![outbox_dispatch_schedule(), jobs_cleanup_schedule()],
            secrets: Arc::new(UnavailableSecretVault),
            record_types,
        }
    }

    pub fn from_app(app: &App, env: &WorkerEnv, config: &WorkerConfig) -> Self {
        crate::state::install_runtime_state(env.runtime_state());
        let parts = app.worker_parts(env);
        let mut runtime =
            Self::new(env.pool.clone(), config).with_secret_vault(env.secrets.clone());
        runtime.record_types = env.record_types.clone();
        for handler in parts.handlers {
            runtime = runtime.with_handler(handler);
        }
        for consumer in parts.consumers {
            runtime = runtime.with_consumer(consumer);
        }
        for schedule in parts.schedules {
            runtime = runtime.with_schedule(schedule);
        }
        runtime
    }

    pub fn with_handler(mut self, handler: Arc<dyn JobHandler>) -> Self {
        self.handlers.register(handler);
        self
    }

    pub fn with_consumer(mut self, consumer: Arc<dyn EventConsumer>) -> Self {
        self.consumers.register(consumer);
        self
    }

    pub fn with_schedule(mut self, schedule: ScheduledJobDefinition) -> Self {
        self.schedules
            .retain(|existing| existing.kind != schedule.kind);
        self.schedules.push(schedule);
        self
    }

    pub fn with_secret_vault(mut self, secrets: SharedSecretVault) -> Self {
        self.secrets = secrets;
        self
    }

    pub fn secrets(&self) -> SharedSecretVault {
        self.secrets.clone()
    }

    pub fn record_types(&self) -> Arc<RecordTypeCatalog> {
        self.record_types.clone()
    }

    pub fn pool(&self) -> MySqlPool {
        self.pool.clone()
    }

    pub fn schedules(&self) -> &[ScheduledJobDefinition] {
        &self.schedules
    }

    pub fn build_worker(&self) -> JobWorker {
        let consumers = Arc::new(self.consumers.clone());
        let registry = JobRegistry::new()
            .with(Arc::new(JobCleanupHandler::new(self.pool.clone())))
            .with(Arc::new(OutboxDispatchHandler::new(
                self.pool.clone(),
                consumers.clone(),
            )))
            .with(Arc::new(EventConsumeHandler::new(
                self.pool.clone(),
                consumers,
            )))
            .merge(&self.handlers);
        JobWorker::new(self.pool.clone(), registry, self.settings.clone())
    }

    pub async fn run(self, shutdown: Shutdown) -> Result<(), AppError> {
        let scheduler = JobScheduler::new(self.pool.clone());
        scheduler.ensure(&self.schedules, Utc::now()).await?;
        let worker = self.build_worker();

        let scheduler_shutdown = shutdown.clone();
        let interval = self.scheduler_interval;
        let scheduler_task =
            tokio::spawn(async move { scheduler.run(interval, scheduler_shutdown).await });
        let heartbeat_task = crate::infrastructure::system::spawn_heartbeat(
            self.pool.clone(),
            &self.settings.worker_id,
            crate::infrastructure::system::SystemSettings::from_env().heartbeat_interval,
            shutdown.clone(),
        );

        let result = worker.run(shutdown).await;
        if let Err(join_error) = scheduler_task.await {
            tracing::error!("job scheduler stopped unexpectedly: {join_error}");
        }
        if let Err(join_error) = heartbeat_task.await {
            tracing::error!("worker heartbeat stopped unexpectedly: {join_error}");
        }
        result
    }
}
