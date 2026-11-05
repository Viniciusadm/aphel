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
    #[serde(default)]
    pub operations: Vec<ProvidedOperation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvidedEffect {
    #[default]
    Read,
    Write,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidedOperation {
    pub key: String,
    pub description: String,
    #[serde(default)]
    pub effect: ProvidedEffect,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub idempotent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionSpec {
    pub point: String,
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl Provides {
    pub fn violations(&self, contribute: &[ContributionSpec]) -> Vec<(String, String)> {
        provision_violations(self, contribute)
    }
}

pub const MAX_PROVIDED_OPERATIONS: usize = 64;
pub const MAX_CONTRIBUTIONS: usize = 128;
const MAX_OPERATION_KEY_LENGTH: usize = 96;
const MAX_DESCRIPTION_LENGTH: usize = 240;

fn provision_violations(
    provides: &Provides,
    contribute: &[ContributionSpec],
) -> Vec<(String, String)> {
    let mut violations = Vec::new();
    if provides.operations.len() > MAX_PROVIDED_OPERATIONS {
        violations.push((
            "provides.operations".to_owned(),
            format!("can list at most {MAX_PROVIDED_OPERATIONS} operations"),
        ));
    }
    for (index, operation) in provides.operations.iter().enumerate() {
        let path = format!("provides.operations[{index}]");
        if !valid_dotted_key(&operation.key) {
            violations.push((
                format!("{path}.key"),
                "must be dotted lowercase segments such as `module.verb`".to_owned(),
            ));
        }
        if provides.operations[..index]
            .iter()
            .any(|previous| previous.key == operation.key)
        {
            violations.push((format!("{path}.key"), "must be unique".to_owned()));
        }
        if operation.description.trim().is_empty()
            || operation.description.len() > MAX_DESCRIPTION_LENGTH
        {
            violations.push((
                format!("{path}.description"),
                format!("must have between 1 and {MAX_DESCRIPTION_LENGTH} characters"),
            ));
        }
        if let Some(permission) = &operation.permission
            && crate::domain::access::Permission::parse(permission.clone()).is_err()
        {
            violations.push((
                format!("{path}.permission"),
                "must be a valid permission".to_owned(),
            ));
        }
    }
    if contribute.len() > MAX_CONTRIBUTIONS {
        violations.push((
            "contribute".to_owned(),
            format!("can list at most {MAX_CONTRIBUTIONS} contributions"),
        ));
    }
    for (index, contribution) in contribute.iter().enumerate() {
        let path = format!("contribute[{index}]");
        if !valid_dotted_key(&contribution.point) {
            violations.push((
                format!("{path}.point"),
                "must be dotted lowercase segments such as `dashboard.widget`".to_owned(),
            ));
        }
        if !valid_key(&contribution.name) {
            violations.push((format!("{path}.name"), "must be a lowercase key".to_owned()));
        }
        if contribute[..index].iter().any(|previous| {
            previous.point == contribution.point && previous.name == contribution.name
        }) {
            violations.push((
                format!("{path}.name"),
                "must be unique within its point".to_owned(),
            ));
        }
    }
    violations
}

fn valid_dotted_key(key: &str) -> bool {
    key.len() <= MAX_OPERATION_KEY_LENGTH && key.contains('.') && key.split('.').all(valid_key)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uses {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

impl Uses {
    pub fn unmet(&self, provided: &[String]) -> Vec<String> {
        self.required
            .iter()
            .filter(|requirement| !provided.contains(requirement))
            .cloned()
            .collect()
    }
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
    #[serde(default)]
    pub contribute: Vec<ContributionSpec>,
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
        if let Some((path, message)) = provision_violations(&self.provides, &self.contribute)
            .into_iter()
            .next()
        {
            return Err(invalid(format!("{path} {message}")));
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
