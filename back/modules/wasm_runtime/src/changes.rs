use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::manifest::PLUGIN_EVENT_PREFIX;
use crate::registry;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::public_events::custom_event_record_type;
use aphel_sdk::platform::application::shared::{DomainEvent, EventConsumer};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;

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
    pool: Db,
}

impl ChangeFeedConsumer {
    pub(crate) fn new(pool: Db) -> Self {
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
        crate::store::changes::insert(
            &self.pool,
            crate::store::changes::NewChange {
                plugin_key: &owner,
                event_id: event.id.to_string(),
                event_type: &event.event_type,
                record_type: event
                    .record
                    .as_ref()
                    .map(|record| record.record_type.as_str().to_owned()),
                record_id: event
                    .record
                    .as_ref()
                    .map(|record| record.record_id.to_string()),
                fields: changed_fields(&event.payload),
                occurred_at: event.occurred_at.naive_utc(),
            },
        )
        .await?;
        Ok(())
    }
}

pub(crate) async fn prune(pool: &Db) -> Result<(), AppError> {
    crate::store::changes::prune(pool, RETENTION_HOURS).await
}

pub(crate) async fn page(
    scope: &AccessScope,
    plugin: &str,
    since: u64,
    limit: u32,
) -> Result<ChangesPage, AppError> {
    let state = aphel_sdk::runtime()?;
    let pool = state.db()?;
    let registry = state.registry().await?;
    let oldest = crate::store::changes::oldest_seq(&pool, plugin).await?;
    let reset = since > 0 && oldest.is_some_and(|oldest| oldest > since + 1);
    let rows = crate::store::changes::after(&pool, plugin, since, limit.clamp(1, MAX_PAGE)).await?;
    let mut next = since;
    let mut data = Vec::with_capacity(rows.len());
    for row in rows {
        let seq = row.seq;
        next = seq;
        let record_type = row.record_type;
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
        data.push(ChangeResponse {
            seq,
            event_type: row.event_type,
            record_id: row.record_id,
            record_type,
            fields: row.fields,
            occurred_at: row.occurred_at.and_utc().to_rfc3339(),
        });
    }
    Ok(ChangesPage { data, next, reset })
}

pub(crate) fn as_event(change: &ChangeResponse) -> Value {
    json!(change)
}
