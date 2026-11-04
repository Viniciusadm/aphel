use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const KEY: &str = "catalog_items";
pub const RECORD_TYPE: &str = "catalog_items.item";
pub const API_TAG: &str = "catalog-items";
pub const PRODUCT_NOT_FOUND: &str = "product_not_found";
pub const READ: &str = "catalog_items.read";
pub const CREATE: &str = "catalog_items.create";
pub const UPDATE: &str = "catalog_items.update";
pub const DELETE: &str = "catalog_items.delete";
pub const EXPORT: &str = "catalog_items.export";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("catalog_items", "1.0.0"),
    entries: &[
        KeyEntry::new(RECORD_TYPE, KeyKind::Reference, "A catalog item record"),
        KeyEntry::new(
            "catalog_items.references",
            KeyKind::ExtensionPoint,
            "Authorized catalog identities and portable code references",
        ),
    ],
};
