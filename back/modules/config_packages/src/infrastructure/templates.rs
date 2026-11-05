use std::collections::BTreeSet;

use rust_embed::RustEmbed;
use serde_json::Value;

use crate::application::{PackageTemplate, TemplateCatalog};
use aphel_sdk::platform::application::organization_profile::SUPPORTED_LOCALES;

#[derive(RustEmbed)]
#[folder = "../../../blueprints/templates/"]
struct TemplateFiles;

#[derive(Debug, Clone, Copy, Default)]
pub struct EmbeddedTemplates;

impl EmbeddedTemplates {
    fn load(locale: &str, key: &str) -> Option<PackageTemplate> {
        let file = TemplateFiles::get(&format!("{locale}/{key}.json"))?;
        let package = serde_json::from_slice::<Value>(&file.data).ok()?;
        Some(PackageTemplate {
            key: key.to_owned(),
            package,
        })
    }

    fn keys() -> BTreeSet<String> {
        TemplateFiles::iter()
            .filter_map(|path| {
                let (_, file) = path.split_once('/')?;
                file.strip_suffix(".json").map(str::to_owned)
            })
            .collect()
    }
}

fn is_valid_key(key: &str) -> bool {
    key.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '_'
            || character == '-'
    })
}

impl TemplateCatalog for EmbeddedTemplates {
    fn templates(&self, locale: &str) -> Vec<PackageTemplate> {
        Self::keys()
            .iter()
            .filter_map(|key| self.template(key, locale))
            .collect()
    }

    fn template(&self, key: &str, locale: &str) -> Option<PackageTemplate> {
        if !is_valid_key(key) {
            return None;
        }
        let preferred = SUPPORTED_LOCALES
            .iter()
            .find(|supported| **supported == locale)
            .into_iter();
        preferred
            .chain(SUPPORTED_LOCALES.iter())
            .find_map(|candidate| Self::load(candidate, key))
    }
}
