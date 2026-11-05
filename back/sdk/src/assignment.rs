use crate::{AppError, fields::EntityTraits};
pub use aphel_contract_assignment::*;
pub struct AssignedCreate<'a> {
    pub rule: &'a str,
    pub record_type: &'a str,
    pub values: &'a serde_json::Map<String, serde_json::Value>,
    pub body: serde_json::Value,
    pub owner_field: &'a str,
    pub source: &'a str,
    pub idempotency_key: String,
}
pub trait AssignmentTraits {
    fn workload(self, mapping: WorkloadMapping) -> Self;
    fn workload_mapping(&self) -> Result<Option<WorkloadMapping>, AppError>;
}
impl AssignmentTraits for EntityTraits {
    fn workload(self, mapping: WorkloadMapping) -> Self {
        self.with_trait_data(
            WORKLOAD.key(),
            serde_json::to_value(mapping).expect("workload mapping"),
        )
    }
    fn workload_mapping(&self) -> Result<Option<WorkloadMapping>, AppError> {
        self.trait_data(WORKLOAD.key())
            .map(|value| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    AppError::Configuration(format!("invalid assignment workload mapping: {error}"))
                })
            })
            .transpose()
    }
}

pub trait AssignmentPermissions {
    const ASSIGNMENT_MANAGE: crate::access::Permission;
}
impl AssignmentPermissions for crate::access::Permission {
    const ASSIGNMENT_MANAGE: crate::access::Permission =
        crate::access::Permission::from_static("assignment.manage");
}

#[async_trait::async_trait]
pub trait AssignmentSelector: Send + Sync {
    async fn options(
        &self,
        ctx: &crate::Ctx<'_>,
        record_type: &str,
    ) -> Result<Vec<AssignmentRuleOption>, AppError>;

    async fn usable(
        &self,
        ctx: &crate::Ctx<'_>,
        rule: &str,
        record_type: &str,
    ) -> Result<bool, AppError>;
    async fn select(
        &self,
        ctx: &crate::Ctx<'_>,
        rule: &str,
        record_type: &str,
        values: &serde_json::Map<String, serde_json::Value>,
        source: &str,
        idempotency_key: String,
    ) -> Result<Option<aphel_kernel::domain::UserId>, AppError>;

    async fn create_assigned(
        &self,
        ctx: &crate::Ctx<'_>,
        request: AssignedCreate<'_>,
    ) -> Result<Option<crate::platform::application::shared::RecordRef>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentRuleOption {
    pub id: uuid::Uuid,
    pub name: String,
}
pub fn selector(
    ctx: &crate::Ctx<'_>,
) -> Result<Option<std::sync::Arc<dyn AssignmentSelector>>, AppError> {
    let sources = ctx.app().contributions::<dyn AssignmentSelector>();
    if sources.len() > 1 {
        return Err(AppError::Configuration(
            "multiple assignment selectors".into(),
        ));
    }
    Ok(sources.into_iter().next())
}

#[derive(Debug, Clone)]
pub struct HostRuleMember {
    pub user_id: aphel_kernel::domain::UserId,
    pub away: bool,
    pub away_from: Option<chrono::DateTime<chrono::Utc>>,
    pub away_until: Option<chrono::DateTime<chrono::Utc>>,
}
#[derive(Debug, Clone)]
pub struct HostRule {
    pub id: uuid::Uuid,
    pub name: String,
    pub is_active: bool,
    pub members: Vec<HostRuleMember>,
}
pub enum HostPick {
    Untracked,
    Unavailable,
    Picked(aphel_kernel::domain::UserId),
}
pub struct WeightedHost {
    pub user_id: aphel_kernel::domain::UserId,
    pub weight: u32,
}
pub struct HostPool {
    pub rule: Option<HostRule>,
    pub members: Vec<WeightedHost>,
}
#[async_trait::async_trait]
pub trait HostAssignment: Send + Sync {
    async fn rule_names(
        &self,
        pool: &sqlx::MySqlPool,
        ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<uuid::Uuid, String>, AppError>;

    async fn find_rule(
        &self,
        ctx: &crate::Ctx<'_>,
        id: uuid::Uuid,
    ) -> Result<Option<HostRule>, AppError>;
    async fn active_rules(&self, ctx: &crate::Ctx<'_>) -> Result<Vec<HostRule>, AppError>;
    async fn rule_pool(
        &self,
        ctx: &crate::Ctx<'_>,
        id: uuid::Uuid,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<HostPool, AppError>;
    async fn pick_user(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        id: uuid::Uuid,
        hosts: &[WeightedHost],
    ) -> Result<HostPick, AppError>;
}
pub fn host_assignment() -> Option<std::sync::Arc<dyn HostAssignment>> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn HostAssignment>()
}
