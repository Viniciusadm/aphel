use async_trait::async_trait;
use sqlx::types::Json;

use crate::application::system::model::SystemAuditEntry;
use crate::application::system::ports::SystemAuditLog;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone, Default)]
pub struct MySqlSystemAuditLog;

#[async_trait]
impl SystemAuditLog<MySqlTx> for MySqlSystemAuditLog {
    async fn record(&self, tx: &mut MySqlTx, entry: SystemAuditEntry) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO system_audit_logs (id, action, actor_user_id, subject_type, subject_id, metadata, ip, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.id.to_string())
        .bind(entry.action)
        .bind(entry.actor_user_id)
        .bind(entry.subject_type)
        .bind(entry.subject_id)
        .bind(Json(&entry.metadata))
        .bind(entry.ip)
        .bind(mapper::naive(entry.occurred_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
