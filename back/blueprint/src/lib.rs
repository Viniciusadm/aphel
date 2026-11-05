use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "blueprint.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Plugins {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub recommended: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Label {
    Text(String),
    Localized(BTreeMap<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NavigationSection {
    pub key: String,
    pub label: Label,
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Navigation {
    #[serde(default)]
    pub home: Option<String>,
    #[serde(default)]
    pub sections: Vec<NavigationSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Role {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Blueprint {
    pub key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub plugins: Plugins,
    #[serde(default)]
    pub navigation: Navigation,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub configuration: Configuration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedBlueprint {
    pub key: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub lineage: Vec<String>,
    pub required: Vec<String>,
    pub recommended: Vec<String>,
    pub navigation: Navigation,
    pub roles: Vec<Role>,
    pub configuration: Configuration,
}

impl ResolvedBlueprint {
    pub fn plugins(&self) -> impl Iterator<Item = &str> {
        self.required
            .iter()
            .chain(self.recommended.iter())
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintError {
    Parse { source: String, message: String },
    InvalidKey(String),
    DuplicateKey(String),
    UnknownParent { key: String, parent: String },
    Cycle(String),
    UnknownBlueprint(String),
}

impl fmt::Display for BlueprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { source, message } => write!(formatter, "{source}: {message}"),
            Self::InvalidKey(key) => write!(formatter, "invalid blueprint key: {key}"),
            Self::DuplicateKey(key) => write!(formatter, "duplicate blueprint key: {key}"),
            Self::UnknownParent { key, parent } => {
                write!(
                    formatter,
                    "blueprint {key} extends unknown blueprint {parent}"
                )
            }
            Self::Cycle(key) => write!(formatter, "blueprint {key} extends itself"),
            Self::UnknownBlueprint(key) => write!(formatter, "unknown blueprint: {key}"),
        }
    }
}

impl std::error::Error for BlueprintError {}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    blueprints: BTreeMap<String, Blueprint>,
}

impl Catalog {
    pub fn parse<'a>(
        sources: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, BlueprintError> {
        let mut parsed = Vec::new();
        for (source, text) in sources {
            let blueprint =
                toml::from_str::<Blueprint>(text).map_err(|error| BlueprintError::Parse {
                    source: source.to_owned(),
                    message: error.to_string(),
                })?;
            parsed.push(blueprint);
        }
        Self::new(parsed)
    }

    pub fn new(blueprints: Vec<Blueprint>) -> Result<Self, BlueprintError> {
        let mut by_key = BTreeMap::new();
        for blueprint in blueprints {
            if !valid_key(&blueprint.key) {
                return Err(BlueprintError::InvalidKey(blueprint.key));
            }
            let key = blueprint.key.clone();
            if by_key.insert(key.clone(), blueprint).is_some() {
                return Err(BlueprintError::DuplicateKey(key));
            }
        }
        let catalog = Self { blueprints: by_key };
        for key in catalog.blueprints.keys() {
            catalog.lineage(key)?;
        }
        Ok(catalog)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.blueprints.keys().map(String::as_str)
    }

    pub fn get(&self, key: &str) -> Option<&Blueprint> {
        self.blueprints.get(key)
    }

    fn lineage(&self, key: &str) -> Result<Vec<&Blueprint>, BlueprintError> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = self
            .blueprints
            .get(key)
            .ok_or_else(|| BlueprintError::UnknownBlueprint(key.to_owned()))?;
        loop {
            if !seen.insert(current.key.as_str()) {
                return Err(BlueprintError::Cycle(key.to_owned()));
            }
            chain.push(current);
            let Some(parent) = current.extends.as_deref() else {
                break;
            };
            current = self
                .blueprints
                .get(parent)
                .ok_or_else(|| BlueprintError::UnknownParent {
                    key: current.key.clone(),
                    parent: parent.to_owned(),
                })?;
        }
        chain.reverse();
        Ok(chain)
    }

    pub fn resolve(&self, key: &str) -> Result<ResolvedBlueprint, BlueprintError> {
        let chain = self.lineage(key)?;
        let leaf = chain.last().expect("lineage always holds the blueprint");
        let mut required: Vec<String> = Vec::new();
        let mut recommended: Vec<String> = Vec::new();
        let mut navigation = Navigation::default();
        let mut roles: Vec<Role> = Vec::new();
        let mut configuration = Configuration::default();
        for blueprint in &chain {
            let excluded: BTreeSet<&str> = blueprint
                .plugins
                .exclude
                .iter()
                .map(String::as_str)
                .collect();
            required.retain(|plugin| !excluded.contains(plugin.as_str()));
            recommended.retain(|plugin| !excluded.contains(plugin.as_str()));
            push_unique(&mut required, &blueprint.plugins.required);
            push_unique(&mut recommended, &blueprint.plugins.recommended);
            if blueprint.navigation.home.is_some() {
                navigation.home.clone_from(&blueprint.navigation.home);
            }
            for section in &blueprint.navigation.sections {
                match navigation
                    .sections
                    .iter_mut()
                    .find(|existing| existing.key == section.key)
                {
                    Some(existing) => {
                        existing.label.clone_from(&section.label);
                        push_unique(&mut existing.items, &section.items);
                    }
                    None => navigation.sections.push(section.clone()),
                }
            }
            for role in &blueprint.roles {
                match roles.iter_mut().find(|existing| existing.key == role.key) {
                    Some(existing) => {
                        existing.label.clone_from(&role.label);
                        push_unique(&mut existing.permissions, &role.permissions);
                    }
                    None => roles.push(role.clone()),
                }
            }
            push_unique(
                &mut configuration.packages,
                &blueprint.configuration.packages,
            );
            if blueprint.configuration.theme.is_some() {
                configuration
                    .theme
                    .clone_from(&blueprint.configuration.theme);
            }
        }
        recommended.retain(|plugin| !required.contains(plugin));
        Ok(ResolvedBlueprint {
            key: leaf.key.clone(),
            name: leaf.name.clone(),
            version: leaf.version.clone(),
            description: leaf.description.clone(),
            lineage: chain
                .iter()
                .map(|blueprint| blueprint.key.clone())
                .collect(),
            required,
            recommended,
            navigation,
            roles,
            configuration,
        })
    }

    pub fn resolve_all(&self) -> Result<Vec<ResolvedBlueprint>, BlueprintError> {
        self.keys().map(|key| self.resolve(key)).collect()
    }
}

fn push_unique(target: &mut Vec<String>, additions: &[String]) {
    for addition in additions {
        if !target.contains(addition) {
            target.push(addition.clone());
        }
    }
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../../blueprints/"]
#[include = "*/blueprint.toml"]
#[include = "*/themes/*.json"]
struct EmbeddedFiles;

pub fn embedded_file(path: &str) -> Option<String> {
    let file = EmbeddedFiles::get(path)?;
    String::from_utf8(file.data.into_owned()).ok()
}

pub fn embedded() -> Result<Catalog, BlueprintError> {
    let files: Vec<(String, String)> = EmbeddedFiles::iter()
        .filter(|path| path.ends_with("/blueprint.toml"))
        .filter_map(|path| {
            let file = EmbeddedFiles::get(&path)?;
            let text = String::from_utf8(file.data.into_owned()).ok()?;
            Some((path.into_owned(), text))
        })
        .collect();
    Catalog::parse(
        files
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str())),
    )
}
