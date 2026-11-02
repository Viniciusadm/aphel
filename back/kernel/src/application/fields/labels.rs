use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LocalizedLabels(pub BTreeMap<String, String>);

impl LocalizedLabels {
    pub fn from_catalog(catalog: &str, key: &str) -> Self {
        let mut catalog: BTreeMap<String, Self> =
            serde_json::from_str(catalog).expect("valid label catalog");
        catalog.remove(key).expect("declared label key")
    }

    pub fn get(&self, locale: &str) -> &str {
        self.0
            .get(locale)
            .or_else(|| self.0.get(locale.split('-').next().unwrap_or(locale)))
            .or_else(|| self.0.get("en"))
            .or_else(|| self.0.values().next())
            .map(String::as_str)
            .unwrap_or_default()
    }

    pub fn default_text(&self) -> &str {
        self.get("pt-BR")
    }
}

impl From<String> for LocalizedLabels {
    fn from(label: String) -> Self {
        Self(BTreeMap::from([("en".to_owned(), label)]))
    }
}

impl From<&str> for LocalizedLabels {
    fn from(label: &str) -> Self {
        label.to_owned().into()
    }
}
