use chrono::NaiveDateTime;
use serde_json::Value;
use uuid::Uuid;

use crate::database::Db;
use crate::errors::AppError;

pub(crate) struct AuditEntry {
    pub id: String,
    pub operator_email: Option<String>,
    pub action: String,
    pub tenant_id: Option<String>,
    pub details: Value,
    pub ip: Option<String>,
    pub occurred_at: NaiveDateTime,
}

type AuditRow = (
    String,
    Option<String>,
    String,
    Option<String>,
    sqlx::types::Json<Value>,
    Option<String>,
    NaiveDateTime,
);

pub(crate) async fn insert(
    pool: &Db,
    operator_id: Option<Uuid>,
    action: &str,
    tenant_id: Option<Uuid>,
    details: &Value,
    ip: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO operator_audit (id, operator_id, action, tenant_id, details, ip) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(operator_id.map(|id| id.to_string()))
    .bind(action)
    .bind(tenant_id.map(|id| id.to_string()))
    .bind(sqlx::types::Json(details))
    .bind(ip)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn count(pool: &Db) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM operator_audit")
        .fetch_one(pool)
        .await?)
}

pub(crate) async fn page(pool: &Db, limit: u64, offset: u64) -> Result<Vec<AuditEntry>, AppError> {
    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT a.id, o.email, a.action, a.tenant_id, a.details, a.ip, a.occurred_at \
         FROM operator_audit a LEFT JOIN operators o ON o.id = a.operator_id \
         ORDER BY a.occurred_at DESC, a.id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, operator_email, action, tenant_id, details, ip, occurred_at)| AuditEntry {
                id,
                operator_email,
                action,
                tenant_id,
                details: details.0,
                ip,
                occurred_at,
            },
        )
        .collect())
}
