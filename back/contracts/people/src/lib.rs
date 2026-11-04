use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const KEY: &str = "people";
pub const RECORD_TYPE: &str = "people.person";
pub const READ: &str = "people.read";
pub const CREATE: &str = "people.create";
pub const UPDATE: &str = "people.update";
pub const DELETE: &str = "people.delete";
pub const EXPORT: &str = "people.export";
pub const ARCHIVE: &str = "people.archive";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("people", "1.0.0"),
    entries: &[
        KeyEntry::new(RECORD_TYPE, KeyKind::Reference, "A person record"),
        KeyEntry::new(
            "people.primary_organizations",
            KeyKind::ExtensionPoint,
            "Bounded primary organization maintenance snapshots",
        ),
    ],
};
