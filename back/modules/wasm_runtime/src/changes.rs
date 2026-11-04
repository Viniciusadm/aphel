use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{MySqlPool, Row, types::Json};
use utoipa::ToSchema;

use crate::manifest::PLUGIN_EVENT_PREFIX;
use crate::registry;
use aphel_kernel::application::public_events::custom_event_record_type;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::runtime_state;

pub(crate) const CHANGES_CONSUMER: &str = "wasm_runtime.changes";
pub(crate) const MAX_PAGE: u32 = 500;
pub(crate) const RETENTION_HOURS: i64 = 24;
pub(crate) const STREAM_POLL: Duration = Duration::from_secs(1);
pub(crate) const STREAM_LIFETIME: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ChangeResponse {
    pub seq: u64,
    pub event_type: String,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub fields: Vec<String>,
    pub occurred_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ChangesPage {
    pub data: Vec<ChangeResponse>,
    pub next: u64,
    pub reset: bool,
}

pub(crate) struct ChangeFeedConsumer {
    pool: MySqlPool,
}

impl ChangeFeedConsumer {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn changed_fields(payload: &Value) -> Vec<String> {
    payload
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|change| change.get("field").and_then(Value::as_str))
        .map(str::to_owned)
        .take(100)
        .collect()
}

#[async_trait]
impl EventConsumer for ChangeFeedConsumer {
    fn name(&self) -> &str {
        CHANGES_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        custom_event_record_type(event_type).is_some()
            || event_type.starts_with(PLUGIN_EVENT_PREFIX)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let snapshot = registry::current(&self.pool).await?;
        let owner = match event.event_type.strip_prefix(PLUGIN_EVENT_PREFIX) {
            Some(rest) => rest
                .split('.')
                .next()
                .and_then(|key| snapshot.plugin(key))
                .map(|plugin| plugin.key.clone()),
            None => event.record.as_ref().and_then(|record| {
                snapshot
                    .plugins
                    .iter()
                    .find(|plugin| {
                        plugin
                            .manifest
                            .entity_record_types()
                            .iter()
                            .any(|record_type| record_type == record.record_type.as_str())
                    })
                    .map(|plugin| plugin.key.clone())
            }),
        };
        let Some(owner) = owner else {
            return Ok(());
        };
        sqlx::query(
            "INSERT IGNORE INTO plugins_changes (plugin_key, event_id, event_type, record_type, record_id, fields, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&owner)
        .bind(event.id.to_string())
        .bind(&event.event_type)
        .bind(event.record.as_ref().map(|record| record.record_type.as_str().to_owned()))
        .bind(event.record.as_ref().map(|record| record.record_id.to_string()))
        .bind(Json(changed_fields(&event.payload)))
        .bind(event.occurred_at.naive_utc())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

pub(crate) async fn prune(pool: &MySqlPool) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM plugins_changes WHERE created_at < UTC_TIMESTAMP(6) - INTERVAL ? HOUR LIMIT 10000",
    )
    .bind(RETENTION_HOURS)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn page(
    scope: &AccessScope,
    plugin: &str,
    since: u64,
    limit: u32,
) -> Result<ChangesPage, AppError> {
    let state = runtime_state()?;
    let pool = state.db()?;
    let registry = state.registry().await?;
    let oldest: Option<u64> =
        sqlx::query_scalar("SELECT MIN(seq) FROM plugins_changes WHERE plugin_key = ?")
            .bind(plugin)
            .fetch_one(&pool)
            .await?;
    let reset = since > 0 && oldest.is_some_and(|oldest| oldest > since + 1);
    let rows = sqlx::query(
        "SELECT seq, event_type, record_type, record_id, fields, occurred_at FROM plugins_changes \
         WHERE plugin_key = ? AND seq > ? ORDER BY seq LIMIT ?",
    )
    .bind(plugin)
    .bind(since)
    .bind(limit.clamp(1, MAX_PAGE))
    .fetch_all(&pool)
    .await?;
    let mut next = since;
    let mut data = Vec::with_capacity(rows.len());
    for row in &rows {
        let seq: u64 = row.try_get("seq")?;
        next = seq;
        let record_type: Option<String> = row.try_get("record_type")?;
        if let Some(record_type) = &record_type {
            let readable = registry.require(record_type).is_ok_and(|definition| {
                scope
                    .permissions()
                    .scope_for(&definition.read_permission)
                    .is_some()
            });
            if !readable {
                continue;
            }
        }
        let Json(fields): Json<Vec<String>> = row.try_get("fields")?;
        let occurred_at: chrono::NaiveDateTime = row.try_get("occurred_at")?;
        data.push(ChangeResponse {
            seq,
            event_type: row.try_get("event_type")?,
            record_id: row.try_get("record_id")?,
            record_type,
            fields,
            occurred_at: occurred_at.and_utc().to_rfc3339(),
        });
    }
    Ok(ChangesPage { data, next, reset })
}

pub(crate) fn as_event(change: &ChangeResponse) -> Value {
    json!(change)
}
