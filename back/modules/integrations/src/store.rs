use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::engine::{Integration, IntegrationError, IntegrationStatus};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const INTEGRATION_COLUMNS: &str = "id, provider_key, config, status, recent_errors, last_tested_at, \
     last_test_ok, created_at, updated_at";

#[derive(Clone)]
pub(crate) struct IntegrationStore {
    pool: MySqlPool,
}

impl IntegrationStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn integration_from_row(row: &MySqlRow) -> Result<Integration, AppError> {
    let id: String = row.try_get("id")?;
    let status: String = row.try_get("status")?;
    let Json(config): Json<Value> = row.try_get("config")?;
    let Json(recent_errors): Json<Value> = row.try_get("recent_errors")?;
    let config = match config {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    let recent_errors: Vec<IntegrationError> =
        serde_json::from_value(recent_errors).unwrap_or_default();
    Ok(Integration {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("integrations.id", &id))?,
        provider_key: row.try_get("provider_key")?,
        config,
        status: IntegrationStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("integrations.status", &status))?,
        recent_errors,
        last_tested_at: row
            .try_get::<Option<NaiveDateTime>, _>("last_tested_at")?
            .map(mapper::utc),
        last_test_ok: row.try_get::<Option<bool>, _>("last_test_ok")?,
        created_at: mapper::utc(row.try_get("created_at")?),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

pub(crate) fn errors_json(errors: &[IntegrationError]) -> Value {
    serde_json::to_value(errors).unwrap_or_else(|_| Value::Array(Vec::new()))
}

fn optional(value: Option<DateTime<Utc>>) -> Option<NaiveDateTime> {
    value.map(mapper::naive)
}

impl IntegrationStore {
    pub(crate) async fn list(&self) -> Result<Vec<Integration>, AppError> {
        let sql = format!("SELECT {INTEGRATION_COLUMNS} FROM integrations ORDER BY provider_key");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(integration_from_row)
            .collect()
    }

    pub(crate) async fn find(&self, provider_key: &str) -> Result<Option<Integration>, AppError> {
        let sql = format!("SELECT {INTEGRATION_COLUMNS} FROM integrations WHERE provider_key = ?");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(provider_key)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(integration_from_row)
            .transpose()
    }

    pub(crate) async fn find_for_update(
        &self,
        tx: &mut Tx,
        provider_key: &str,
    ) -> Result<Option<Integration>, AppError> {
        let sql = format!(
            "SELECT {INTEGRATION_COLUMNS} FROM integrations WHERE provider_key = ? FOR UPDATE"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(provider_key)
            .fetch_optional(&mut **tx)
            .await?
            .as_ref()
            .map(integration_from_row)
            .transpose()
    }

    pub(crate) async fn save(
        &self,
        tx: &mut Tx,
        integration: &Integration,
    ) -> Result<(), AppError> {
        let last_error = integration.last_error();
        sqlx::query(
            "INSERT INTO integrations (id, provider_key, config, status, last_error, \
             last_error_at, recent_errors, last_tested_at, last_test_ok, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE config = VALUES(config), status = VALUES(status), \
             last_error = VALUES(last_error), last_error_at = VALUES(last_error_at), \
             recent_errors = VALUES(recent_errors), last_tested_at = VALUES(last_tested_at), \
             last_test_ok = VALUES(last_test_ok), updated_at = VALUES(updated_at)",
        )
        .bind(integration.id.to_string())
        .bind(&integration.provider_key)
        .bind(Json(Value::Object(integration.config.clone())))
        .bind(integration.status.as_str())
        .bind(last_error.map(|error| error.message.as_str()))
        .bind(optional(last_error.map(|error| error.at)))
        .bind(Json(errors_json(&integration.recent_errors)))
        .bind(optional(integration.last_tested_at))
        .bind(integration.last_test_ok)
        .bind(mapper::naive(integration.created_at))
        .bind(mapper::naive(integration.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut Tx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM integrations WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
