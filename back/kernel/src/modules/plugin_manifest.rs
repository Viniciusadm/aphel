use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::modules::manifest::MAX_KEY_LENGTH;

pub const INVALID_PLUGIN_MANIFEST: &str = "invalid_plugin_manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Capability,
    Domain,
}

impl Layer {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Domain => "domain",
        }
    }

    pub const fn may_depend_on(&self, other: Layer) -> bool {
        (*self as u8) >= (other as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    Native,
    Wasm,
}

impl Runtime {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provides {
    #[serde(default)]
    pub contracts: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uses {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub layer: Layer,
    pub runtime: Runtime,
    #[serde(default)]
    pub provides: Provides,
    #[serde(default)]
    pub uses: Uses,
}

impl PluginManifest {
    pub fn parse(text: &str) -> Result<Self, AppError> {
        let manifest: Self = toml::from_str(text)
            .map_err(|error| invalid(format!("plugin.toml is invalid: {error}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if !valid_key(&self.key) {
            return Err(invalid(format!(
                "`{}` is not a valid plugin key (a-z, 0-9 and _, at most {MAX_KEY_LENGTH})",
                self.key
            )));
        }
        if self.name.trim().is_empty() {
            return Err(invalid("name is required".to_owned()));
        }
        semver::Version::parse(&self.version).map_err(|error| {
            invalid(format!("version `{}` is not semver: {error}", self.version))
        })?;
        for requirement in self
            .provides
            .contracts
            .iter()
            .chain(&self.uses.required)
            .chain(&self.uses.optional)
        {
            if !valid_requirement(requirement) {
                return Err(invalid(format!(
                    "`{requirement}` is not a contract requirement (use `name@major`)"
                )));
            }
        }
        if let Some(shared) = self
            .uses
            .required
            .iter()
            .find(|requirement| self.uses.optional.contains(requirement))
        {
            return Err(invalid(format!("`{shared}` is both required and optional")));
        }
        Ok(())
    }
}

fn invalid(message: String) -> AppError {
    AppError::validation_code(INVALID_PLUGIN_MANIFEST, message)
}

fn valid_key(key: &str) -> bool {
    let mut characters = key.chars();
    key.len() <= MAX_KEY_LENGTH
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn valid_requirement(requirement: &str) -> bool {
    requirement.split_once('@').is_some_and(|(name, major)| {
        valid_key(name.replace('-', "_").as_str())
            && !major.is_empty()
            && major.chars().all(|character| character.is_ascii_digit())
    })
}
