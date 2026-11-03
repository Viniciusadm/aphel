use serde_json::Value;

use crate::errors::AppError;

pub const MAX_KEY_LENGTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDependency {
    pub key: &'static str,
    pub version_req: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDeclaration {
    pub permission: &'static str,
    pub label: &'static str,
    pub record_scoped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDeclaration {
    pub event_type: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub const fn new(version: u32, description: &'static str, sql: &'static str) -> Self {
        Self {
            version,
            description,
            sql,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingKind {
    Text,
    LongText,
    Number {
        min: Option<i64>,
        max: Option<i64>,
    },
    Boolean,
    Select {
        options: Vec<SettingOption>,
    },
    Email,
    Url,
    Secret,
    Field {
        record_type: &'static str,
        field_types: Vec<&'static str>,
    },
}

impl SettingKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::LongText => "long_text",
            Self::Number { .. } => "number",
            Self::Boolean => "boolean",
            Self::Select { .. } => "select",
            Self::Email => "email",
            Self::Url => "url",
            Self::Secret => "secret",
            Self::Field { .. } => "field",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingOption {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingField {
    pub key: &'static str,
    pub label: &'static str,
    pub help: Option<&'static str>,
    pub kind: SettingKind,
    pub required: bool,
    pub default: Option<Value>,
}

impl SettingField {
    pub fn new(key: &'static str, label: &'static str, kind: SettingKind) -> Self {
        Self {
            key,
            label,
            help: None,
            kind,
            required: false,
            default: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn is_secret(&self) -> bool {
        self.kind == SettingKind::Secret
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestBase {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub min_aphel_version: &'static str,
    pub dependencies: Vec<ModuleDependency>,
    pub permissions: Vec<PermissionDeclaration>,
    pub events: Vec<EventDeclaration>,
    pub settings: Vec<SettingField>,
    pub migrations: Vec<Migration>,
    pub owned_tables: Vec<&'static str>,
}

impl ManifestBase {
    pub fn new(key: &'static str, name: &'static str, version: &'static str) -> Self {
        Self {
            key,
            name,
            description: "",
            version,
            min_aphel_version: "0.1.0",
            dependencies: Vec::new(),
            permissions: Vec::new(),
            events: Vec::new(),
            settings: Vec::new(),
            migrations: Vec::new(),
            owned_tables: Vec::new(),
        }
    }

    pub fn description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    pub fn min_aphel_version(mut self, version: &'static str) -> Self {
        self.min_aphel_version = version;
        self
    }

    pub fn depends_on(mut self, key: &'static str, version_req: &'static str) -> Self {
        self.dependencies
            .push(ModuleDependency { key, version_req });
        self
    }

    pub fn permission(mut self, permission: &'static str, label: &'static str) -> Self {
        self.permissions.push(PermissionDeclaration {
            permission,
            label,
            record_scoped: false,
        });
        self
    }

    pub fn record_permission(mut self, permission: &'static str, label: &'static str) -> Self {
        self.permissions.push(PermissionDeclaration {
            permission,
            label,
            record_scoped: true,
        });
        self
    }

    pub fn event(mut self, event_type: &'static str, description: &'static str) -> Self {
        self.events.push(EventDeclaration {
            event_type,
            description,
        });
        self
    }

    pub fn setting(mut self, field: SettingField) -> Self {
        self.settings.push(field);
        self
    }

    pub fn migration(mut self, migration: Migration) -> Self {
        self.migrations.push(migration);
        self
    }

    pub fn owned_tables(mut self, tables: &[&'static str]) -> Self {
        for table in tables {
            if !self.owned_tables.contains(table) {
                self.owned_tables.push(table);
            }
        }
        self
    }

    pub fn table_prefix(&self) -> String {
        table_prefix(self.key)
    }

    pub fn semver(&self) -> Result<semver::Version, AppError> {
        parse_version(self.key, "version", self.version)
    }

    pub fn min_aphel_semver(&self) -> Result<semver::Version, AppError> {
        parse_version(self.key, "min_aphel_version", self.min_aphel_version)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        let invalid =
            |message: String| AppError::Internal(format!("module {}: {message}", self.key));
        if !is_module_key(self.key) {
            return Err(invalid(format!(
                "key must be 2 to {MAX_KEY_LENGTH} lowercase letters or underscores"
            )));
        }
        if self.name.trim().is_empty() {
            return Err(invalid("name is required".to_owned()));
        }
        self.semver()?;
        self.min_aphel_semver()?;
        for dependency in &self.dependencies {
            if dependency.key == self.key || !is_module_key(dependency.key) {
                return Err(invalid(format!("invalid dependency {}", dependency.key)));
            }
            semver::VersionReq::parse(dependency.version_req).map_err(|error| {
                invalid(format!(
                    "dependency {} has an invalid version requirement: {error}",
                    dependency.key
                ))
            })?;
        }
        let prefix = format!("{}.", self.key);
        for declaration in &self.permissions {
            let action = declaration.permission.strip_prefix(&prefix);
            if !action.is_some_and(is_identifier) {
                return Err(invalid(format!(
                    "permission {} must look like {}.<action>",
                    declaration.permission, self.key
                )));
            }
        }
        for declaration in &self.events {
            let valid = declaration.event_type.split('.').count() >= 2
                && declaration.event_type.split('.').all(is_identifier);
            if !valid {
                return Err(invalid(format!(
                    "event type {} must look like <resource>.<action>",
                    declaration.event_type
                )));
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        for field in &self.settings {
            if !is_identifier(field.key) || !keys.insert(field.key) {
                return Err(invalid(format!(
                    "invalid or duplicated setting {}",
                    field.key
                )));
            }
        }
        for table in &self.owned_tables {
            if !is_table_name(table) {
                return Err(invalid(format!("invalid owned table {table}")));
            }
        }
        let mut previous = 0;
        for migration in &self.migrations {
            if migration.version <= previous {
                return Err(invalid(
                    "migrations must have increasing versions starting at 1".to_owned(),
                ));
            }
            previous = migration.version;
        }
        Ok(())
    }
}

pub fn table_prefix(key: &str) -> String {
    format!("{key}_")
}

pub fn is_module_key(value: &str) -> bool {
    (2..=MAX_KEY_LENGTH).contains(&value.len())
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && !value.ends_with('_')
        && value
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
}

pub fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
}

fn is_table_name(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn parse_version(key: &str, what: &str, value: &str) -> Result<semver::Version, AppError> {
    semver::Version::parse(value).map_err(|error| {
        AppError::Internal(format!("module {key}: invalid {what} {value:?}: {error}"))
    })
}
