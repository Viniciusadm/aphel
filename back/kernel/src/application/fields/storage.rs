use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityStorage {
    #[default]
    Table,
    Managed,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityIndex {
    pub key: String,
    pub fields: Vec<String>,
    #[serde(default)]
    pub unique: bool,
}

pub fn managed_table(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(key.as_bytes());
    format!(
        "entity_{}",
        hash[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

#[derive(Debug, Clone, Copy)]
pub struct EntityKeyAliases(pub &'static [(&'static str, &'static str)]);
