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
    let result = super::store::audit::insert(
        pool,
        actor.operator_id,
        action,
        tenant_id,
        &details,
        actor.ip.as_deref(),
    )
    .await;
    if let Err(error) = result {
        tracing::error!(action, "could not record the operator action: {error}");
    }
}
