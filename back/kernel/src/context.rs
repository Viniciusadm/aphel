use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::application::custom_objects::RecordTypeCatalog;
use crate::application::fields::RecordTypeRegistry;
use crate::application::shared::SharedFileStorage;
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::fields::MySqlCustomFieldPorts;
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::platform::{
    SharedAuditLog, SharedEventPublisher, SharedJobQueue, SharedSecretVault,
};
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::modules::{App, HookCtx, HookRef, Services};
use crate::records::MySqlRecordPipeline;
use crate::state::AppState;

#[derive(Clone)]
pub struct Ctx<'a> {
    pub scope: &'a AccessScope,
    pool: Db,
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
        pool: Db,
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

    pub fn runtime(&self) -> crate::state::Runtime {
        self.state().into()
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

    fn ensure_record_enabled(&self, entity: &str) -> Result<(), AppError> {
        let definition = self.registry.require(entity)?;
        if self
            .app
            .record_contributions()
            .iter()
            .find(|entry| entry.registers.as_ref() == Some(&definition.record_type))
            .is_some_and(|entry| !self.app.is_enabled(entry.module))
        {
            return Err(crate::modules::module_disabled());
        }
        Ok(())
    }

    fn record_handler(
        &self,
        entity: &str,
    ) -> Result<Arc<dyn crate::records::RecordTypeHandler>, AppError> {
        self.ensure_record_enabled(entity)?;
        self.registry.handler(entity)
    }

    pub async fn can_access_record(
        &self,
        record: &crate::application::shared::RecordRef,
        permission: &crate::domain::access::Permission,
    ) -> Result<bool, AppError> {
        self.ensure_record_enabled(record.record_type.as_str())?;
        use crate::application::files::RecordAccessChecker;
        crate::infrastructure::records::MySqlRecordAccessChecker::new(
            self.pool.clone(),
            self.registry.clone(),
        )
        .can_with(self.scope, record, permission)
        .await
    }

    pub async fn load_record(
        &self,
        record_type: &str,
        id: uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let state = self.state();
        self.record_handler(record_type)?
            .load(&crate::records::RecordCtx::new(self.scope, &state), id)
            .await
    }

    pub async fn list_records(
        &self,
        record_type: &str,
        query: crate::application::query::RecordQuery,
    ) -> Result<crate::application::organization_admin::Listing<serde_json::Value>, AppError> {
        let state = self.state();
        let definition = self.registry.require(record_type)?;
        let ports = self.query();
        let listing = crate::application::query::RecordQueryEngine::new(&ports)
            .query(self.scope, definition.record_type.as_str(), query)
            .await?;
        let items = self
            .record_handler(record_type)?
            .load_listed(
                &crate::records::RecordCtx::new(self.scope, &state),
                &listing.items,
            )
            .await?;
        Ok(crate::application::organization_admin::Listing {
            items,
            total: listing.total,
        })
    }

    pub async fn create_record(
        &self,
        record_type: &str,
        values: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let state = self.state();
        self.record_handler(record_type)?
            .create(&crate::records::RecordCtx::new(self.scope, &state), values)
            .await
    }

    pub async fn update_record(
        &self,
        record_type: &str,
        id: uuid::Uuid,
        values: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let state = self.state();
        self.record_handler(record_type)?
            .update(
                &crate::records::RecordCtx::new(self.scope, &state),
                id,
                values,
            )
            .await
    }

    pub async fn create_record_in_tx(
        &self,
        tx: &mut Tx,
        entity: &str,
        values: serde_json::Value,
    ) -> Result<crate::records::PendingCommit, AppError> {
        let state = self.state();
        self.record_handler(entity)?
            .create_in_tx(
                &crate::records::RecordCtx::new(self.scope, &state),
                tx,
                values,
            )
            .await
    }

    pub async fn update_record_in_tx(
        &self,
        tx: &mut Tx,
        entity: &str,
        id: uuid::Uuid,
        values: serde_json::Value,
    ) -> Result<Option<crate::records::PendingCommit>, AppError> {
        let state = self.state();
        self.record_handler(entity)?
            .update_in_tx(
                &crate::records::RecordCtx::new(self.scope, &state),
                tx,
                id,
                values,
            )
            .await
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

    pub fn pool(&self) -> &Db {
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

    pub async fn begin(&self) -> Result<Tx, AppError> {
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

    pub async fn run_hook<K: HookRef>(
        &self,
        hook: K,
        tx: &mut Tx,
        module: &'static str,
        args: &K::Args,
    ) -> Result<(), AppError> {
        let hooks = self.app.hooks();
        if !hooks.has(hook) {
            return Ok(());
        }
        let mut hook_ctx = HookCtx::new(tx, &self.pool, self.scope, self.app.services(), module);
        hooks.run(hook, &mut hook_ctx, args).await
    }

    pub async fn apply_hook<K: HookRef>(
        &self,
        hook: K,
        tx: &mut Tx,
        module: &'static str,
        args: K::Args,
    ) -> Result<K::Args, AppError> {
        let hooks = self.app.hooks();
        if !hooks.has(hook) {
            return Ok(args);
        }
        let mut hook_ctx = HookCtx::new(tx, &self.pool, self.scope, self.app.services(), module);
        hooks.apply(hook, &mut hook_ctx, args).await
    }
}
