use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletePolicy {
    #[default]
    Restrict,
    Clear,
    Cascade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReferenceTarget {
    Entity {
        entity: String,
    },
    Trait {
        trait_key: String,
        type_field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftReference {
    pub field: String,
    pub target: ReferenceTarget,
    #[serde(default)]
    pub on_delete: DeletePolicy,
}
