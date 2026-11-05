use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;

use crate::application::auth::AuthSettings;
use crate::application::custom_objects::RecordTypeCatalog;
use crate::application::jobs::ScheduledJobDefinition;
use crate::application::mail::SharedMailer;
use crate::application::organization_profile::PublicBaseUrl;
use crate::application::shared::{
    DomainEvent, EventConsumer, JobContext, JobHandler, SharedFileStorage,
};
use crate::config::OutboundHttpConfig;
use crate::errors::AppError;
use crate::infrastructure::platform::SharedSecretVault;
use crate::modules::enabled::EnabledModules;
use crate::state::AppState;

#[derive(Clone)]
pub struct WorkerEnv {
    pub pool: MySqlPool,
    pub secrets: SharedSecretVault,
    pub mailer: SharedMailer,
    pub storage: SharedFileStorage,
    pub public_url: PublicBaseUrl,
    pub outbound_http: OutboundHttpConfig,
    pub record_types: Arc<RecordTypeCatalog>,
}

impl WorkerEnv {
    pub fn state(&self) -> AppState {
        AppState::new(Some(self.pool.clone()))
            .with_secrets(self.secrets.clone())
            .with_outbound_http(self.outbound_http.clone())
            .with_mysql_platform()
            .with_storage(self.storage.clone())
            .with_record_catalog(self.record_types.clone())
    }

    pub fn runtime(&self) -> crate::state::Runtime {
        self.runtime_state().into()
    }

    pub fn runtime_state(&self) -> AppState {
        self.state().with_auth_settings(AuthSettings {
            app_base_url: self.public_url.clone(),
            ..AuthSettings::default()
        })
    }
}

#[derive(Default)]
pub struct WorkerParts {
    pub handlers: Vec<Arc<dyn JobHandler>>,
    pub consumers: Vec<Arc<dyn EventConsumer>>,
    pub schedules: Vec<ScheduledJobDefinition>,
}

impl WorkerParts {
    pub fn handler(mut self, handler: Arc<dyn JobHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn handlers(mut self, handlers: impl IntoIterator<Item = Arc<dyn JobHandler>>) -> Self {
        self.handlers.extend(handlers);
        self
    }

    pub fn consumer(mut self, consumer: Arc<dyn EventConsumer>) -> Self {
        self.consumers.push(consumer);
        self
    }

    pub fn consumers(
        mut self,
        consumers: impl IntoIterator<Item = Arc<dyn EventConsumer>>,
    ) -> Self {
        self.consumers.extend(consumers);
        self
    }

    pub fn schedule(mut self, schedule: ScheduledJobDefinition) -> Self {
        self.schedules.push(schedule);
        self
    }

    pub fn schedules(
        mut self,
        schedules: impl IntoIterator<Item = ScheduledJobDefinition>,
    ) -> Self {
        self.schedules.extend(schedules);
        self
    }

    pub fn merge(mut self, other: WorkerParts) -> Self {
        self.handlers.extend(other.handlers);
        self.consumers.extend(other.consumers);
        self.schedules.extend(other.schedules);
        self
    }
}

pub type WorkerFactory = Arc<dyn Fn(&WorkerEnv) -> WorkerParts + Send + Sync>;

pub struct GatedHandler {
    pub module: &'static str,
    pub enabled: Arc<EnabledModules>,
    pub inner: Arc<dyn JobHandler>,
}

#[async_trait]
impl JobHandler for GatedHandler {
    fn kind(&self) -> &str {
        self.inner.kind()
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        if !self.enabled.is_enabled(self.module) {
            tracing::debug!(
                module = self.module,
                kind = self.inner.kind(),
                "job skipped: module disabled"
            );
            return Ok(());
        }
        self.inner.handle(context, payload).await
    }
}

pub struct GatedConsumer {
    pub module: &'static str,
    pub enabled: Arc<EnabledModules>,
    pub inner: Arc<dyn EventConsumer>,
}

#[async_trait]
impl EventConsumer for GatedConsumer {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn accepts(&self, event_type: &str) -> bool {
        self.enabled.is_enabled(self.module) && self.inner.accepts(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        if !self.enabled.is_enabled(self.module) {
            return Ok(());
        }
        self.inner.consume(event).await
    }
}

pub fn lazy_pool(database_url: Option<&str>) -> Result<MySqlPool, AppError> {
    let url = database_url.unwrap_or("mysql://aphel@localhost/aphel");
    MySqlPool::connect_lazy(url).map_err(|error| AppError::Configuration(error.to_string()))
}

impl crate::modules::builder::App {
    pub fn worker_parts(&self, env: &WorkerEnv) -> WorkerParts {
        let mut parts = WorkerParts::default();
        for contribution in self.worker_contributions() {
            let module = contribution.module;
            let produced = (contribution.item)(env);
            if self.enabled().is_required(module) {
                parts = parts.merge(produced);
                continue;
            }
            let enabled = self.enabled().clone();
            parts
                .handlers
                .extend(produced.handlers.into_iter().map(|inner| {
                    Arc::new(GatedHandler {
                        module,
                        enabled: enabled.clone(),
                        inner,
                    }) as Arc<dyn JobHandler>
                }));
            parts
                .consumers
                .extend(produced.consumers.into_iter().map(|inner| {
                    Arc::new(GatedConsumer {
                        module,
                        enabled: enabled.clone(),
                        inner,
                    }) as Arc<dyn EventConsumer>
                }));
            parts.schedules.extend(produced.schedules);
        }
        parts
    }
}
