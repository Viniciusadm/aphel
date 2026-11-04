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

pub fn normalize_label(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        let folded = fold_accent(character);
        if folded.is_ascii_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            pending_space = false;
            normalized.push(folded);
        } else {
            pending_space = true;
        }
    }
    normalized
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}
