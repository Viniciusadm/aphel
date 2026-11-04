use aphel_contract::{
    Contract, ContractDescriptor, KeyEntry, KeyKind, OperationKey, ReferenceKind, contract,
    operation, reference,
};
use serde::{Deserialize, Serialize};

pub const TAGGABLE: &str = "tagging.taggable";

pub const CONTRACT: Contract = contract!("tagging", "1.0.0");

pub const APPLY: OperationKey<TagRecord, Tagging> = operation!("tagging.apply");
pub const REMOVE: OperationKey<TagRecord, Tagging> = operation!("tagging.remove");
pub const LIST: OperationKey<ListTags, TagList> = operation!("tagging.list");
pub const GET: OperationKey<GetTag, TagInfo> = operation!("tagging.get");

pub const TAG: ReferenceKind = reference!("tagging.tag");

pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            TAGGABLE,
            KeyKind::ExtensionPoint,
            "Entities that accept tags",
        ),
        KeyEntry::new(
            "tagging.apply",
            KeyKind::Operation,
            "Tag a record the actor can edit",
        ),
        KeyEntry::new(
            "tagging.remove",
            KeyKind::Operation,
            "Remove a tag from a record the actor can edit",
        ),
        KeyEntry::new(
            "tagging.list",
            KeyKind::Operation,
            "Tags of the organization with their usage, optionally only those allowed on a record type",
        ),
        KeyEntry::new("tagging.get", KeyKind::Operation, "One tag by id"),
        KeyEntry::new("tagging.tag", KeyKind::Reference, "A tag"),
    ],
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRecord {
    pub record_type: String,
    pub record_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSummary {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tagging {
    pub changed: bool,
    pub tags: Vec<TagSummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTags {
    #[serde(default)]
    pub record_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTag {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagInfo {
    pub id: String,
    pub name: String,
    pub color: String,
    pub record_types: Vec<String>,
    pub usage_count: u64,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagList {
    pub data: Vec<TagInfo>,
}
