pub mod package;
pub mod planning;
pub use aphel_contract::ReferenceKind as ConfigReferenceKind;
use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};
pub use package::*;
pub use planning::*;
pub const CONTRACT: Contract = contract!("configuration", "1.0.0");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "configuration.catalog",
            KeyKind::ExtensionPoint,
            "Scoped configuration template and installation references",
        ),
        KeyEntry::new(
            "configuration.section",
            KeyKind::ExtensionPoint,
            "Owner-provided portable configuration sections",
        ),
        KeyEntry::new(
            "configuration.fragments",
            KeyKind::ExtensionPoint,
            "Owner-provided version fragment metadata",
        ),
    ],
};
