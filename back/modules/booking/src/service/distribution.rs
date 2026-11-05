pub(crate) use aphel_sdk::assignment::{HostPick, WeightedHost};
use aphel_sdk::assignment::{HostPool, HostRule, host_assignment};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::{AppError, Ctx};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(crate) async fn find_rule(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<HostRule>, AppError> {
    match host_assignment() {
        Some(source) => source.find_rule(ctx, id).await,
        None => Ok(None),
    }
}
pub(crate) async fn active_rules(ctx: &Ctx<'_>) -> Result<Vec<HostRule>, AppError> {
    match host_assignment() {
        Some(source) => source.active_rules(ctx).await,
        None => Ok(Vec::new()),
    }
}
pub(crate) async fn rule_pool(
    ctx: &Ctx<'_>,
    rule: &HostRule,
    at: DateTime<Utc>,
) -> Result<HostPool, AppError> {
    match host_assignment() {
        Some(source) => source.rule_pool(ctx, rule.id, at).await,
        None => Ok(HostPool {
            rule: None,
            members: Vec::new(),
        }),
    }
}
pub(crate) async fn pick_user(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    id: Uuid,
    hosts: &[WeightedHost],
) -> Result<HostPick, AppError> {
    match host_assignment() {
        Some(source) => source.pick_user(ctx, tx, id, hosts).await,
        None => Ok(HostPick::Unavailable),
    }
}
