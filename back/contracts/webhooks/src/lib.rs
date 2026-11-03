use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
pub const INCOMING_WEBHOOK: aphel_contract::ReferenceKind =
    aphel_contract::reference!("webhooks.incoming");
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookAction {
    #[serde(rename = "type")]
    pub action_type: ActionKind,
    pub record_type: String,
    #[serde(default)]
    pub match_field: Option<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(default)]
    pub assignment_rule_id: Option<String>,
}

impl WebhookAction {
    pub fn type_name(&self) -> &'static str {
        self.action_type.as_str()
    }
    pub fn kind(&self) -> ActionKind {
        self.action_type
    }
    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    CreateRecord,
    UpsertRecord,
}
impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateRecord => "create_record",
            Self::UpsertRecord => "upsert_record",
        }
    }
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("webhooks", "1.0.0"),
    entries: &[aphel_contract::KeyEntry::new(
        "webhooks.incoming",
        aphel_contract::KeyKind::Reference,
        "An incoming webhook definition",
    )],
};
