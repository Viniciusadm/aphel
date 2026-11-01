use crate::modules::manifest::ManifestBase;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::{Builder, Uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    RequiresAphel(String),
    MissingDependency(String),
    IncompatibleDependency { key: String, requirement: String },
}

impl Compatibility {
    pub fn is_compatible(&self) -> bool {
        *self == Self::Compatible
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::RequiresAphel(_) => "requires_aphel",
            Self::MissingDependency(_) => "missing_dependency",
            Self::IncompatibleDependency { .. } => "incompatible_dependency",
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Compatible => None,
            Self::RequiresAphel(version) => Some(version.clone()),
            Self::MissingDependency(key) => Some(key.clone()),
            Self::IncompatibleDependency { key, requirement } => {
                Some(format!("{key} {requirement}"))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDescriptor {
    pub manifest: ManifestBase,
    pub compatibility: Compatibility,
    pub required: bool,
}

impl ModuleDescriptor {
    pub fn key(&self) -> &'static str {
        self.manifest.key
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleState {
    pub key: String,
    pub enabled: bool,
    pub settings: Map<String, Value>,
    pub installed_version: String,
    pub enabled_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub data_exported_at: Option<DateTime<Utc>>,
    pub data_purged_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl ModuleState {
    pub fn new(key: &str, version: &str, now: DateTime<Utc>) -> Self {
        Self {
            key: key.to_owned(),
            enabled: false,
            settings: Map::new(),
            installed_version: version.to_owned(),
            enabled_at: None,
            disabled_at: None,
            data_exported_at: None,
            data_purged_at: None,
            updated_at: now,
        }
    }

    pub fn record_id(&self) -> Uuid {
        let digest = Sha256::digest(format!("module:{}", self.key).as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        Builder::from_custom_bytes(bytes).into_uuid()
    }

    pub fn export_is_current(&self) -> bool {
        match (self.data_exported_at, self.disabled_at) {
            (Some(exported), Some(disabled)) => exported >= disabled,
            (Some(_), None) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyStatus {
    pub key: String,
    pub requirement: String,
    pub installed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleView {
    pub descriptor: ModuleDescriptor,
    pub state: Option<ModuleState>,
    pub settings: Map<String, Value>,
    pub configured_secrets: Vec<String>,
    pub dependencies: Vec<DependencyStatus>,
    pub enabled_dependents: Vec<String>,
    pub held_by: Option<String>,
}

impl ModuleView {
    pub fn enabled(&self) -> bool {
        self.state.as_ref().is_some_and(|state| state.enabled)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDataExport {
    pub key: String,
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub data: Value,
}
