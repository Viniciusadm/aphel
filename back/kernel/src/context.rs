use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

use crate::application::custom_objects::RecordTypeCatalog;
use crate::application::fields::RecordTypeRegistry;
use crate::application::shared::SharedFileStorage;
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::fields::MySqlCustomFieldPorts;
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::platform::{
    SharedAuditLog, SharedEventPublisher, SharedJobQueue, SharedSecretVault,
};
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::modules::{App, Hook, HookCtx, Services};
use crate::records::MySqlRecordPipeline;
use crate::state::AppState;

#[derive(Clone)]
pub struct Ctx<'a> {
    pub scope: &'a AccessScope,
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    jobs: SharedJobQueue,
    app: Arc<App>,
    storage: Option<SharedFileStorage>,
    secrets: Option<SharedSecretVault>,
    state: Option<AppState>,
}

impl<'a> Ctx<'a> {
    pub async fn from_state(state: &AppState, scope: &'a AccessScope) -> Result<Self, AppError> {
        crate::modules::runtime_catalog::ensure_ready(&state.db()?).await?;
        Ok(Self {
            scope,
            pool: state.db()?,
            registry: state.registry().await?,
            events: state.events.clone(),
            audit: state.audit.clone(),
            jobs: state.jobs.clone(),
            app: state.app.clone(),
            storage: Some(state.storage.clone()),
            secrets: Some(state.secrets.clone()),
            state: Some(state.clone()),
        })
    }

    pub fn from_parts(
        scope: &'a AccessScope,
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            scope,
            jobs: Arc::new(MySqlJobQueue::new(pool.clone())),
            pool,
            registry,
            events,
            audit,
            app: crate::modules::app().clone(),
            storage: None,
            secrets: None,
            state: None,
        }
    }

    pub fn with_storage(mut self, storage: SharedFileStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn with_secrets(mut self, secrets: SharedSecretVault) -> Self {
        self.secrets = Some(secrets);
        self
    }

    pub fn with_scope<'b>(&self, scope: &'b AccessScope) -> Ctx<'b> {
        Ctx {
            scope,
            pool: self.pool.clone(),
            registry: self.registry.clone(),
            events: self.events.clone(),
            audit: self.audit.clone(),
            jobs: self.jobs.clone(),
            app: self.app.clone(),
            storage: self.storage.clone(),
            secrets: self.secrets.clone(),
            state: self.state.clone(),
        }
    }

    pub fn state(&self) -> AppState {
        self.state.clone().unwrap_or_else(|| {
            let state = AppState::new(Some(self.pool.clone()))
                .with_mysql_platform()
                .with_record_types(RecordTypeCatalog::builtin_only(
                    self.registry.as_ref().clone(),
                ))
                .with_events(self.events.clone())
                .with_audit(self.audit.clone());
            let state = match &self.storage {
                Some(storage) => state.with_storage(storage.clone()),
                None => state,
            };
            match &self.secrets {
                Some(secrets) => state.with_secrets(secrets.clone()),
                None => state,
            }
        })
    }

    pub fn storage(&self) -> Result<SharedFileStorage, AppError> {
        self.storage.clone().ok_or_else(|| {
            AppError::Configuration("file storage is not available in this context".to_owned())
        })
    }

    pub fn secrets(&self) -> Result<SharedSecretVault, AppError> {
        self.secrets.clone().ok_or_else(|| {
            AppError::Configuration("the secret vault is not available in this context".to_owned())
        })
    }

    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub fn registry(&self) -> &Arc<RecordTypeRegistry> {
        &self.registry
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn services(&self) -> &Services {
        self.app.services()
    }

    pub fn events(&self) -> &SharedEventPublisher {
        &self.events
    }

    pub fn audit(&self) -> &SharedAuditLog {
        &self.audit
    }

    pub fn jobs(&self) -> &SharedJobQueue {
        &self.jobs
    }

    pub fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    pub async fn begin(&self) -> Result<MySqlTx, AppError> {
        crate::database::receipt::begin(&self.pool).await
    }

    pub fn fields(&self) -> MySqlCustomFieldPorts {
        MySqlCustomFieldPorts::new(self.pool.clone(), self.registry.clone())
    }

    pub fn query(&self) -> MySqlRecordQueryPorts {
        MySqlRecordQueryPorts::new(self.pool.clone(), self.registry.clone())
    }

    pub fn pipeline(&self) -> MySqlRecordPipeline {
        MySqlRecordPipeline::new(self.pool.clone(), self.registry.clone())
    }

    pub async fn run_hook<H: Hook>(
        &self,
        tx: &mut MySqlTx,
        module: &'static str,
        args: &H::Args,
    ) -> Result<(), AppError> {
        let hooks = self.app.hooks();
        if !hooks.has::<H>() {
            return Ok(());
        }
        let mut hook_ctx = HookCtx::new(tx, &self.pool, self.scope, self.app.services(), module);
        hooks.run::<H>(&mut hook_ctx, args).await
    }

    pub async fn apply_hook<H: Hook>(
        &self,
        tx: &mut MySqlTx,
        module: &'static str,
        args: H::Args,
    ) -> Result<H::Args, AppError> {
        let hooks = self.app.hooks();
        if !hooks.has::<H>() {
            return Ok(args);
        }
        let mut hook_ctx = HookCtx::new(tx, &self.pool, self.scope, self.app.services(), module);
        hooks.apply::<H>(&mut hook_ctx, args).await
    }
}
