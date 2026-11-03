use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FIELD_POLICY: &str = "imports.fields";
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImportFieldPolicy {
    pub excluded_fields: Vec<String>,
    pub aliases: BTreeMap<String, Vec<String>>,
    pub satisfied_by: BTreeMap<String, String>,
    pub label_creation: bool,
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("imports", "1.0.0"),
    entries: &[aphel_contract::KeyEntry::new(
        "imports.fields",
        aphel_contract::KeyKind::ExtensionPoint,
        "Owner-declared import fields, aliases and reference creation policy",
    )],
};
