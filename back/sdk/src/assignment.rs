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
