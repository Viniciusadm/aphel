use crate::{AppError, fields::EntityTraits};
pub use aphel_contract_assignment::*;
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
