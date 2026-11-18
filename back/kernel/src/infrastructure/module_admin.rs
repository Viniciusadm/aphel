use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::application::module_admin::{ModuleState, ModuleStateRepository};
use crate::database::transaction::Tx;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const COLUMNS: &str = "module_key, enabled, settings, installed_version, enabled_at, \
    disabled_at, data_exported_at, data_purged_at, updated_at";

pub struct MySqlModuleStateRepository {
    pool: MySqlPool,
}

impl MySqlModuleStateRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn optional_time(
    row: &MySqlRow,
    column: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    Ok(row
        .try_get::<Option<NaiveDateTime>, _>(column)?
        .map(mapper::utc))
}

fn module(row: &MySqlRow) -> Result<ModuleState, AppError> {
    let settings = match row.try_get::<Json<Value>, _>("settings")?.0 {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    Ok(ModuleState {
        key: row.try_get("module_key")?,
        enabled: row.try_get("enabled")?,
        settings,
        installed_version: row.try_get("installed_version")?,
        enabled_at: optional_time(row, "enabled_at")?,
        disabled_at: optional_time(row, "disabled_at")?,
        data_exported_at: optional_time(row, "data_exported_at")?,
        data_purged_at: optional_time(row, "data_purged_at")?,
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

#[async_trait]
impl ModuleStateRepository for MySqlModuleStateRepository {
    type Tx = Tx;

    async fn list(&self) -> Result<Vec<ModuleState>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {COLUMNS} FROM installed_modules ORDER BY module_key"
        )))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(module)
        .collect()
    }

    async fn find(&self, key: &str) -> Result<Option<ModuleState>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {COLUMNS} FROM installed_modules WHERE module_key = ?"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(module)
        .transpose()
    }

    async fn save(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        module: &ModuleState,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO installed_modules (module_key, enabled, settings, \
             installed_version, enabled_at, disabled_at, data_exported_at, data_purged_at, updated_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), settings = VALUES(settings), \
             installed_version = VALUES(installed_version), enabled_at = VALUES(enabled_at), \
             disabled_at = VALUES(disabled_at), data_exported_at = VALUES(data_exported_at), \
             data_purged_at = VALUES(data_purged_at), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(&module.key)
        .bind(module.enabled)
        .bind(Json(Value::Object(module.settings.clone())))
        .bind(&module.installed_version)
        .bind(module.enabled_at.map(mapper::naive))
        .bind(module.disabled_at.map(mapper::naive))
        .bind(module.data_exported_at.map(mapper::naive))
        .bind(module.data_purged_at.map(mapper::naive))
        .bind((!scope.is_system()).then(|| scope.user_id().as_str()))
        .bind(mapper::naive(module.updated_at))
        .bind(mapper::naive(module.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
