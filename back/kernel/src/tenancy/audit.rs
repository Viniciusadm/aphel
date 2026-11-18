use crate::database::Db;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperatorActor {
    pub operator_id: Option<Uuid>,
    pub ip: Option<String>,
}

impl OperatorActor {
    pub fn command_line() -> Self {
        Self::default()
    }

    pub fn operator(operator_id: Uuid, ip: Option<String>) -> Self {
        Self {
            operator_id: Some(operator_id),
            ip,
        }
    }
}

pub async fn record_operator_action(
    pool: &Db,
    actor: &OperatorActor,
    action: &str,
    tenant_id: Option<Uuid>,
    details: Value,
) {
    let result = sqlx::query(
        "INSERT INTO operator_audit (id, operator_id, action, tenant_id, details, ip) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(actor.operator_id.map(|id| id.to_string()))
    .bind(action)
    .bind(tenant_id.map(|id| id.to_string()))
    .bind(sqlx::types::Json(&details))
    .bind(actor.ip.as_deref())
    .execute(pool)
    .await;
    if let Err(error) = result {
        tracing::error!(action, "could not record the operator action: {error}");
    }
}
