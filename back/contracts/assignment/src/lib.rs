use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, ReferenceKind, contract,
    extension_point, reference,
};
use serde::{Deserialize, Serialize};
pub const CONTRACT: Contract = contract!("assignment", "1.0.0");
pub const WORKLOAD: ExtensionPoint<WorkloadMapping> = extension_point!("assignment.workload");
pub const RULE: ReferenceKind = reference!("assignment.rule");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "assignment.workload",
            KeyKind::ExtensionPoint,
            "Owner-declared active workload states",
        ),
        KeyEntry::new(
            "assignment.rule",
            KeyKind::Reference,
            "A record assignment rule",
        ),
    ],
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadMapping {
    pub state_field: String,
    pub active_states: Vec<String>,
}
