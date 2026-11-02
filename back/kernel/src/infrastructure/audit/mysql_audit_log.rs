use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::types::Json;

use crate::application::shared::{AuditEntry, AuditLog, FieldChange};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const MAX_IP_LENGTH: usize = 45;

#[derive(Debug, Clone, Copy, Default)]
pub struct MySqlAuditLog;

impl MySqlAuditLog {
    pub fn new() -> Self {
        Self
    }
}

pub(crate) fn changes_to_json(changes: &[FieldChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
            .collect(),
    )
}

#[async_trait]
impl AuditLog<MySqlTx> for MySqlAuditLog {
    async fn record(&self, tx: &mut MySqlTx, mut entry: AuditEntry) -> Result<(), AppError> {
        let execution = crate::application::shared::ExecutionContext::current();
        if let Some(execution) = &execution
            && execution.executor_kind == "plugin"
        {
            entry.actor = crate::application::shared::Actor::plugin(execution.executor.clone());
        }
        let (record_type, record_id) = match &entry.record {
            Some(record) => (
                Some(record.record_type.as_str().to_owned()),
                Some(record.record_id.to_string()),
            ),
            None => (None, None),
        };
        let ip = entry
            .ip
            .as_deref()
            .map(|ip| ip.chars().take(MAX_IP_LENGTH).collect::<String>());
        let reason = entry
            .reason
            .clone()
            .or_else(crate::application::shared::current_audit_reason)
            .map(|reason| {
                reason
                    .chars()
                    .take(crate::application::shared::MAX_AUDIT_REASON_LENGTH)
                    .collect::<String>()
            });
        sqlx::query(
            "INSERT INTO audit_logs (id, action, record_type, record_id, actor_type, actor_id, changes, ip, reason, causation, execution, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.id.to_string())
        .bind(&entry.action)
        .bind(record_type)
        .bind(record_id)
        .bind(entry.actor.kind())
        .bind(entry.actor.id())
        .bind(Json(changes_to_json(&entry.changes)))
        .bind(ip)
        .bind(reason)
        .bind(entry.causation.as_ref().map(Json))
        .bind(execution.as_ref().map(Json))
        .bind(mapper::naive(entry.occurred_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
