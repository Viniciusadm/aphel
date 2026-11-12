use std::sync::Arc;

use crate::modules::manifest::table_prefix;
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySql, MySqlPool, QueryBuilder};

use crate::api::state::{AppState, SharedAuditLog, SharedEventPublisher, SharedSecretVault};
use crate::application::fields::is_sql_identifier;
use crate::application::module_admin::{
    ModuleDescriptor, ModuleLifecycle, ModulePorts, ModuleUseCases, module_not_found,
};
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::module_admin::MySqlModuleStateRepository;
use crate::infrastructure::transactions::MySqlTransactionManager;
use crate::modules::builder::{App, LoadedModule};
use crate::modules::migrations::apply_grants;
use crate::modules::module::ModuleCtx;

const MAX_PURGE_PASSES: usize = 8;

pub struct ModuleRuntime {
    pool: MySqlPool,
    app: Arc<App>,
}

impl ModuleRuntime {
    pub fn new(pool: MySqlPool, app: Arc<App>) -> Self {
        Self { pool, app }
    }

    fn entry(&self, key: &str) -> Result<&LoadedModule, AppError> {
        self.app.module(key).ok_or_else(module_not_found)
    }

    fn context<'a>(&'a self, scope: &'a AccessScope, key: &'static str) -> ModuleCtx<'a> {
        ModuleCtx {
            pool: &self.pool,
            scope,
            services: self.app.services(),
            module: key,
        }
    }

    async fn refresh(&self) {
        if let Err(error) = self.app.enabled().refresh(&self.pool).await {
            tracing::warn!("could not refresh the enabled modules: {error}");
        }
    }

    async fn data_tables(&self, key: &str) -> Result<Vec<String>, AppError> {
        let own = format!("{}%", table_prefix(key).replace('_', "\\_"));
        let owned = self
            .app
            .module(key)
            .map(|entry| entry.manifest.owned_tables.clone())
            .unwrap_or_default();
        let mut query = QueryBuilder::<MySql>::new(
            "SELECT TABLE_NAME FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' AND (TABLE_NAME LIKE ",
        );
        query.push_bind(own);
        if !owned.is_empty() {
            query.push(" OR TABLE_NAME IN (");
            let mut tables = query.separated(", ");
            for table in owned {
                tables.push_bind(table);
            }
            query.push(")");
        }
        query.push(") ORDER BY TABLE_NAME");
        let tables: Vec<String> = query.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(tables
            .into_iter()
            .filter(|table| is_sql_identifier(table))
            .collect())
    }

    async fn columns(&self, table: &str) -> Result<Vec<String>, AppError> {
        let columns: Vec<String> = sqlx::query_scalar(
            "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
        )
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        Ok(columns
            .into_iter()
            .filter(|column| is_sql_identifier(column))
            .collect())
    }

    async fn export_table(&self, table: &str) -> Result<Value, AppError> {
        let columns = self.columns(table).await?;
        if columns.is_empty() {
            return Ok(json!([]));
        }
        let object = columns
            .iter()
            .map(|column| format!("'{column}', `{column}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let rows: Option<Json<Value>> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT JSON_ARRAYAGG(JSON_OBJECT({object})) FROM `{table}`"
        )))
        .fetch_one(&self.pool)
        .await?;
        Ok(rows.map(|rows| rows.0).unwrap_or_else(|| json!([])))
    }
}

#[async_trait]
impl ModuleLifecycle for ModuleRuntime {
    type Tx = MySqlTx;

    fn descriptors(&self) -> Vec<ModuleDescriptor> {
        self.app
            .modules()
            .iter()
            .map(|entry| ModuleDescriptor {
                manifest: entry.manifest.base.clone(),
                compatibility: entry.compatibility.clone(),
                required: entry.manifest.required,
            })
            .collect()
    }

    fn validate_settings(&self, key: &str, settings: &Value) -> Vec<FieldViolation> {
        self.app
            .module(key)
            .map(|entry| entry.module.validate_settings(settings))
            .unwrap_or_default()
    }

    async fn held_modules(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        crate::modules::toggle_guard::held_modules(&self.pool).await
    }

    async fn extension_dependents(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, AppError> {
        crate::modules::toggle_guard::dependents(&self.pool).await
    }

    async fn on_enable(&self, scope: &AccessScope, key: &str) -> Result<(), AppError> {
        let entry = self.entry(key)?;
        self.refresh().await;
        let mut connection = self.pool.acquire().await?;
        apply_grants(&mut connection, &self.app, entry.key()).await?;
        entry
            .module
            .on_enable(&mut self.context(scope, entry.key()))
            .await
    }

    async fn on_disable(&self, scope: &AccessScope, key: &str) -> Result<(), AppError> {
        let entry = self.entry(key)?;
        self.refresh().await;
        entry
            .module
            .on_disable(&mut self.context(scope, entry.key()))
            .await
    }

    async fn export_data(&self, scope: &AccessScope, key: &str) -> Result<Value, AppError> {
        let entry = self.entry(key)?;
        let mut tables = Map::new();
        for table in self.data_tables(key).await? {
            let rows = self.export_table(&table).await?;
            tables.insert(table, rows);
        }
        let extra = entry
            .module
            .export_data(&mut self.context(scope, entry.key()))
            .await?;
        Ok(json!({ "tables": tables, "module": extra }))
    }

    async fn purge_data(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        key: &str,
    ) -> Result<u64, AppError> {
        let entry = self.entry(key)?;
        entry
            .module
            .purge_data(&mut self.context(scope, entry.key()), tx)
            .await?;
        let mut removed = 0;
        for record_type in self
            .app
            .record_contributions()
            .iter()
            .filter(|contribution| contribution.module == entry.key())
            .filter_map(|contribution| contribution.registers.as_ref())
        {
            removed += crate::records::purge_record_type_data(tx, record_type).await?;
        }
        let mut pending = self.data_tables(key).await?;
        for _ in 0..MAX_PURGE_PASSES {
            if pending.is_empty() {
                break;
            }
            let before = pending.len();
            let mut blocked = Vec::new();
            for table in pending {
                let result = sqlx::query(AssertSqlSafe(format!("DELETE FROM `{table}`")))
                    .execute(&mut **tx)
                    .await;
                match result {
                    Ok(done) => removed += done.rows_affected(),
                    Err(error) => {
                        tracing::debug!(module = key, table = %table, "purge retried: {error}");
                        blocked.push(table);
                    }
                }
            }
            let stuck = blocked.len() == before;
            pending = blocked;
            if stuck {
                break;
            }
        }
        if !pending.is_empty() {
            return Err(AppError::Internal(format!(
                "could not remove the data of {key} from {}",
                pending.join(", ")
            )));
        }
        Ok(removed)
    }
}

pub struct MySqlModulePorts {
    transactions: MySqlTransactionManager,
    modules: MySqlModuleStateRepository,
    runtime: ModuleRuntime,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    secrets: SharedSecretVault,
    clock: SystemClock,
}

impl ModulePorts for MySqlModulePorts {
    type Transactions = MySqlTransactionManager;
    type Modules = MySqlModuleStateRepository;
    type Runtime = ModuleRuntime;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Secrets = SharedSecretVault;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn modules(&self) -> &Self::Modules {
        &self.modules
    }

    fn runtime(&self) -> &Self::Runtime {
        &self.runtime
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn secrets(&self) -> &Self::Secrets {
        &self.secrets
    }

    fn clock(&self) -> &Self::Clock {
        &self.clock
    }
}

pub fn module_use_cases(state: &AppState) -> Result<ModuleUseCases<MySqlModulePorts>, AppError> {
    let pool = state.db()?;
    Ok(ModuleUseCases::new(MySqlModulePorts {
        transactions: MySqlTransactionManager::new(pool.clone()),
        modules: MySqlModuleStateRepository::new(pool.clone()),
        runtime: ModuleRuntime::new(pool, state.app.clone()),
        events: state.events.clone(),
        audit: state.audit.clone(),
        secrets: state.secrets.clone(),
        clock: SystemClock,
    }))
}
