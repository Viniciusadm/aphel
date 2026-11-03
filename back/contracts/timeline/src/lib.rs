use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const KEY: &str = "timeline";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindDefinition {
    pub key: String,
    pub labels: BTreeMap<String, String>,
    pub event_types: Vec<String>,
    pub event_prefixes: Vec<String>,
    pub hidden_event_prefixes: Vec<String>,
}
impl KindDefinition {
    pub fn new(key: &str, en: &str, pt_br: &str) -> Self {
        Self {
            key: key.to_owned(),
            labels: BTreeMap::from([
                ("en".to_owned(), en.to_owned()),
                ("pt-BR".to_owned(), pt_br.to_owned()),
            ]),
            event_types: Vec::new(),
            event_prefixes: Vec::new(),
            hidden_event_prefixes: Vec::new(),
        }
    }
    pub fn events(mut self, events: &[&str]) -> Self {
        self.event_types = events.iter().map(|value| (*value).to_owned()).collect();
        self
    }
    pub fn prefixes(mut self, prefixes: &[&str]) -> Self {
        self.event_prefixes = prefixes.iter().map(|value| (*value).to_owned()).collect();
        self
    }
    pub fn hide(mut self, prefixes: &[&str]) -> Self {
        self.hidden_event_prefixes = prefixes.iter().map(|value| (*value).to_owned()).collect();
        self
    }
}
pub fn valid_kind(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("timeline", "1.0.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "timeline.source",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-authorized timeline sources",
        ),
        aphel_contract::KeyEntry::new(
            "timeline.kind",
            aphel_contract::KeyKind::ExtensionPoint,
            "Open timeline kinds and localized labels",
        ),
    ],
};
