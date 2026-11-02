use std::collections::{BTreeSet, HashMap};

use uuid::Uuid;

use aphel_kernel::application::fields::option_value_from_label;
use aphel_kernel::domain::fields::is_identifier;

pub const MAX_CONFIG_KEY_LENGTH: usize = 64;

pub fn is_config_key(value: &str) -> bool {
    is_identifier(value, MAX_CONFIG_KEY_LENGTH)
}

pub fn key_from_name(name: &str) -> String {
    let key = option_value_from_label(name);
    let starts_with_digit = name
        .trim()
        .starts_with(|character: char| character.is_ascii_digit());
    match key.strip_prefix("option_") {
        _ if key.is_empty() => "item".to_owned(),
        Some(rest) if starts_with_digit => format!("item_{rest}"),
        _ => key,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedItem {
    pub id: Uuid,
    pub key: String,
    pub explicit: bool,
}

pub fn assign_keys<'a>(
    items: impl IntoIterator<Item = (Uuid, &'a str)>,
    stored: &HashMap<Uuid, String>,
) -> Vec<KeyedItem> {
    let items: Vec<(Uuid, &str)> = items.into_iter().collect();
    let mut taken: BTreeSet<String> = items
        .iter()
        .filter_map(|(id, _)| stored.get(id).cloned())
        .collect();
    items
        .into_iter()
        .map(|(id, name)| match stored.get(&id) {
            Some(key) => KeyedItem {
                id,
                key: key.clone(),
                explicit: true,
            },
            None => {
                let base = key_from_name(name);
                let mut candidate = base.clone();
                let mut suffix = 2;
                while taken.contains(&candidate) {
                    candidate = format!("{base}_{suffix}");
                    suffix += 1;
                }
                taken.insert(candidate.clone());
                KeyedItem {
                    id,
                    key: candidate,
                    explicit: false,
                }
            }
        })
        .collect()
}
