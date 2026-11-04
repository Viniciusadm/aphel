use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const KEY: &str = "organizations";
pub const RECORD_TYPE: &str = "organizations.organization";
pub const READ: &str = "organizations.read";
pub const CREATE: &str = "organizations.create";
pub const UPDATE: &str = "organizations.update";
pub const DELETE: &str = "organizations.delete";
pub const EXPORT: &str = "organizations.export";
pub const ARCHIVE: &str = "organizations.archive";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("organizations", "1.0.0"),
    entries: &[KeyEntry::new(
        RECORD_TYPE,
        KeyKind::Reference,
        "An organization record",
    )],
};
