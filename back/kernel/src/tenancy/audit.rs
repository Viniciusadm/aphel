use serde_json::Value;
use sqlx::MySqlPool;
use uuid::Uuid;

pub struct OperatorAction<'a> {
    pub operator_id: Option<Uuid>,
    pub action: &'a str,
    pub tenant_id: Option<Uuid>,
    pub details: Value,
    pub ip: Option<&'a str>,
}

pub async fn record_operator_action(pool: &MySqlPool, action: OperatorAction<'_>) {
    let result = sqlx::query(
        "INSERT INTO operator_audit (id, operator_id, action, tenant_id, details, ip) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(action.operator_id.map(|id| id.to_string()))
    .bind(action.action)
    .bind(action.tenant_id.map(|id| id.to_string()))
    .bind(sqlx::types::Json(&action.details))
    .bind(action.ip)
    .execute(pool)
    .await;
    if let Err(error) = result {
        tracing::error!(
            action = action.action,
            "could not record the operator action: {error}"
        );
    }
}
