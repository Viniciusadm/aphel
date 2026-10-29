use serde::{Deserialize, Serialize};
use serde_json::Value;

use aphel_kernel::application::jobs::CronSchedule;
use aphel_kernel::domain::access::{Permission, is_known_permission};
use aphel_kernel::domain::custom_objects::{CUSTOM_OBJECT_PREFIX, object_record_type};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const API_VERSION: u32 = 1;
pub const INVALID_MANIFEST: &str = "plugin_manifest_invalid";

pub const HOOK_SAVING: &str = "record.saving";
pub const HOOK_VALIDATING: &str = "record.validating";
pub const HOOK_SAVED: &str = "record.saved";
pub const HOOK_DELETING: &str = "record.deleting";
const HOOKS: &[&str] = &[HOOK_SAVING, HOOK_VALIDATING, HOOK_SAVED, HOOK_DELETING];
const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];
pub const SETTING_SECRET: &str = "secret";
const SETTING_KINDS: &[&str] = &["number", "boolean", "text", SETTING_SECRET];
const MAX_SCHEDULES: usize = 10;
const MAX_ALLOWED_HOSTS: usize = 20;
const MAX_KEY_LENGTH: usize = 32;
const MAX_OBJECT_KEY_LENGTH: usize = 48;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub api: u32,
    #[serde(default)]
    pub requires_aphel: Option<String>,
    #[serde(default)]
    pub entities: Vec<EntitySpec>,
    #[serde(default)]
    pub fields: Vec<ExtensionField>,
    #[serde(default)]
    pub hooks: Vec<HookSubscription>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub routes: Vec<RouteSpec>,
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<PermissionSpec>,
    #[serde(default)]
    pub http: HttpSpec,
    #[serde(default)]
    pub schedules: Vec<ScheduleSpec>,
    #[serde(default)]
    pub web: Option<WebSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionSpec {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub grant_to: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HttpSpec {
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub name: String,
    pub cron: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySpec {
    pub key: String,
    pub label: String,
    pub label_plural: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default = "enabled")]
    pub owner_enabled: bool,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub options: Vec<OptionDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionDef {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionField {
    pub record_type: String,
    #[serde(flatten)]
    pub field: FieldDef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookSubscription {
    pub hook: String,
    pub record_types: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteSpec {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingSpec {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSpec {
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub style: Option<String>,
}

fn enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    50
}

fn default_entry() -> String {
    "web/index.js".to_owned()
}

fn is_key(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.starts_with(|first: char| first.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

impl PluginManifest {
    pub fn parse(text: &str) -> Result<Self, AppError> {
        toml::from_str(text).map_err(|error| {
            AppError::validation_code(INVALID_MANIFEST, format!("plugin.toml is invalid: {error}"))
        })
    }

    pub fn entity_record_type(&self, entity: &str) -> String {
        object_record_type(&self.object_key(entity))
    }

    pub fn object_key(&self, entity: &str) -> String {
        format!("{}_{entity}", self.key)
    }

    pub fn entity_record_types(&self) -> Vec<String> {
        self.entities
            .iter()
            .map(|entity| self.entity_record_type(&entity.key))
            .collect()
    }

    pub fn allows(&self, permission: &Permission) -> bool {
        let key = permission.as_str();
        self.requires.iter().any(|required| required == key)
            || self.permissions.iter().any(|declared| declared.key == key)
            || self.entity_record_types().iter().any(|record_type| {
                key.strip_prefix(record_type.as_str())
                    .is_some_and(|action| action.starts_with('.'))
            })
    }

    pub fn host_allowed(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.http
            .allowed_hosts
            .iter()
            .any(|pattern| match pattern.strip_prefix("*.") {
                Some(domain) => host.ends_with(&format!(".{domain}")),
                None => *pattern == host,
            })
    }

    pub fn secret_settings(&self) -> impl Iterator<Item = &SettingSpec> {
        self.settings
            .iter()
            .filter(|setting| setting.kind == SETTING_SECRET)
    }

    pub fn subscribes(&self, hook: &str, record_type: &str) -> Option<i32> {
        self.hooks
            .iter()
            .find(|subscription| {
                subscription.hook == hook
                    && subscription
                        .record_types
                        .iter()
                        .any(|candidate| candidate == record_type || candidate == "*")
            })
            .map(|subscription| subscription.priority)
    }

    pub fn consumes(&self, event_type: &str) -> bool {
        self.events.iter().any(|candidate| {
            candidate == event_type
                || candidate
                    .strip_suffix(".*")
                    .is_some_and(|prefix| event_type.starts_with(&format!("{prefix}.")))
        })
    }

    pub fn validate(
        &self,
        compiled_modules: &[String],
        aphel_version: &str,
    ) -> Result<(), AppError> {
        let mut violations = Vec::new();
        if !is_key(&self.key, MAX_KEY_LENGTH) {
            violations.push(FieldViolation::new(
                "key",
                format!("must have up to {MAX_KEY_LENGTH} lowercase letters, digits or underscores and start with a letter"),
            ));
        }
        if compiled_modules.iter().any(|module| module == &self.key) {
            violations.push(FieldViolation::new(
                "key",
                "is already used by a built-in module",
            ));
        }
        if self.name.trim().is_empty() || self.name.len() > 120 {
            violations.push(FieldViolation::new(
                "name",
                "must have between 1 and 120 characters",
            ));
        }
        if semver::Version::parse(&self.version).is_err() {
            violations.push(FieldViolation::new("version", "must be a semantic version"));
        }
        if self.api != API_VERSION {
            violations.push(FieldViolation::new(
                "api",
                format!("this Aphel supports plugin API {API_VERSION}"),
            ));
        }
        if let Some(requirement) = &self.requires_aphel {
            match (
                semver::VersionReq::parse(requirement),
                semver::Version::parse(aphel_version),
            ) {
                (Ok(requirement), Ok(current)) if !requirement.matches(&current) => {
                    violations.push(FieldViolation::new(
                        "requires_aphel",
                        format!("requires Aphel {requirement}, this is {current}"),
                    ));
                }
                (Err(_), _) => violations.push(FieldViolation::new(
                    "requires_aphel",
                    "must be a version requirement",
                )),
                _ => {}
            }
        }
        for (index, entity) in self.entities.iter().enumerate() {
            let path = format!("entities[{index}]");
            let object_key = self.object_key(&entity.key);
            if !is_key(&entity.key, MAX_KEY_LENGTH) || object_key.len() > MAX_OBJECT_KEY_LENGTH {
                violations.push(FieldViolation::new(
                    format!("{path}.key"),
                    "must be a short lowercase key",
                ));
            }
            for (field_index, field) in entity.fields.iter().enumerate() {
                self.validate_field(
                    &format!("{path}.fields[{field_index}]"),
                    field,
                    &mut violations,
                );
            }
        }
        for (index, extension) in self.fields.iter().enumerate() {
            let path = format!("fields[{index}]");
            if extension.record_type.starts_with(CUSTOM_OBJECT_PREFIX)
                && !self.entity_record_types().contains(&extension.record_type)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.record_type"),
                    "cannot extend custom objects of other plugins",
                ));
            }
            self.validate_field(&path, &extension.field, &mut violations);
        }
        for (index, hook) in self.hooks.iter().enumerate() {
            if !HOOKS.contains(&hook.hook.as_str()) {
                violations.push(FieldViolation::new(
                    format!("hooks[{index}].hook"),
                    format!("must be one of {}", HOOKS.join(", ")),
                ));
            }
            if hook.record_types.is_empty() {
                violations.push(FieldViolation::new(
                    format!("hooks[{index}].record_types"),
                    "must list at least one record type",
                ));
            }
        }
        for (index, route) in self.routes.iter().enumerate() {
            if !METHODS.contains(&route.method.as_str()) {
                violations.push(FieldViolation::new(
                    format!("routes[{index}].method"),
                    format!("must be one of {}", METHODS.join(", ")),
                ));
            }
            if !route.path.starts_with('/') || route.path.contains("..") {
                violations.push(FieldViolation::new(
                    format!("routes[{index}].path"),
                    "must start with /",
                ));
            }
            match (&route.permission, route.public) {
                (Some(_), true) => violations.push(FieldViolation::new(
                    format!("routes[{index}].permission"),
                    "public routes cannot require a permission",
                )),
                (Some(permission), false) => {
                    if !Permission::parse(permission.clone())
                        .is_ok_and(|permission| self.grantable(&permission))
                    {
                        violations.push(FieldViolation::new(
                            format!("routes[{index}].permission"),
                            "must be a known permission or one the plugin declares",
                        ));
                    }
                }
                (None, _) => {}
            }
        }
        for (index, required) in self.requires.iter().enumerate() {
            if !Permission::parse(required.clone()).is_ok_and(|permission| {
                is_known_permission(&permission) || required.starts_with(CUSTOM_OBJECT_PREFIX)
            }) {
                violations.push(FieldViolation::new(
                    format!("requires[{index}]"),
                    "is not a known permission",
                ));
            }
        }
        let prefix = format!("{}.", self.key);
        for (index, declared) in self.permissions.iter().enumerate() {
            let path = format!("permissions[{index}]");
            let valid = declared.key.starts_with(&prefix)
                && Permission::parse(declared.key.clone())
                    .is_ok_and(|permission| !is_known_permission(&permission));
            if !valid {
                violations.push(FieldViolation::new(
                    format!("{path}.key"),
                    format!("must be {prefix}<action> with lowercase letters and underscores"),
                ));
            }
            if declared.label.trim().is_empty() || declared.label.len() > 120 {
                violations.push(FieldViolation::new(
                    format!("{path}.label"),
                    "must have between 1 and 120 characters",
                ));
            }
            if declared.grant_to.iter().any(|role| !is_key(role, 64)) {
                violations.push(FieldViolation::new(
                    format!("{path}.grant_to"),
                    "must list role keys",
                ));
            }
        }
        if self.http.allowed_hosts.len() > MAX_ALLOWED_HOSTS {
            violations.push(FieldViolation::new(
                "http.allowed_hosts",
                format!("can list at most {MAX_ALLOWED_HOSTS} hosts"),
            ));
        }
        for (index, host) in self.http.allowed_hosts.iter().enumerate() {
            if !is_host_pattern(host) {
                violations.push(FieldViolation::new(
                    format!("http.allowed_hosts[{index}]"),
                    "must be a lowercase host name such as api.example.com or *.example.com",
                ));
            }
        }
        if self.schedules.len() > MAX_SCHEDULES {
            violations.push(FieldViolation::new(
                "schedules",
                format!("can list at most {MAX_SCHEDULES} schedules"),
            ));
        }
        for (index, schedule) in self.schedules.iter().enumerate() {
            let path = format!("schedules[{index}]");
            if !is_key(&schedule.name, 64)
                || self.schedules[..index]
                    .iter()
                    .any(|previous| previous.name == schedule.name)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.name"),
                    "must be a unique lowercase key",
                ));
            }
            if CronSchedule::parse(&schedule.cron).is_err() {
                violations.push(FieldViolation::new(
                    format!("{path}.cron"),
                    "must be a cron expression with seconds (sec min hour day month weekday)",
                ));
            }
        }
        for (index, setting) in self.settings.iter().enumerate() {
            if !is_key(&setting.key, 64) {
                violations.push(FieldViolation::new(
                    format!("settings[{index}].key"),
                    "must be a lowercase key",
                ));
            }
            if !SETTING_KINDS.contains(&setting.kind.as_str()) {
                violations.push(FieldViolation::new(
                    format!("settings[{index}].type"),
                    format!("must be one of {}", SETTING_KINDS.join(", ")),
                ));
            }
            if setting.kind == SETTING_SECRET && setting.default.is_some() {
                violations.push(FieldViolation::new(
                    format!("settings[{index}].default"),
                    "secret settings cannot have a default",
                ));
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(AppError::invalid_fields_code(
                INVALID_MANIFEST,
                "plugin.toml is invalid",
                violations,
            ))
        }
    }

    fn grantable(&self, permission: &Permission) -> bool {
        is_known_permission(permission)
            || permission.as_str().starts_with(CUSTOM_OBJECT_PREFIX)
            || self
                .permissions
                .iter()
                .any(|declared| declared.key == permission.as_str())
    }

    fn validate_field(&self, path: &str, field: &FieldDef, violations: &mut Vec<FieldViolation>) {
        if !is_key(&field.key, 64) {
            violations.push(FieldViolation::new(
                format!("{path}.key"),
                "must be a lowercase key",
            ));
        }
        match FieldType::parse(&field.field_type) {
            None => violations.push(FieldViolation::new(
                format!("{path}.type"),
                "is not a known field type",
            )),
            Some(FieldType::Record) if field.target.is_none() => violations.push(
                FieldViolation::new(format!("{path}.target"), "is required for record fields"),
            ),
            Some(FieldType::Select | FieldType::MultiSelect) if field.options.is_empty() => {
                violations.push(FieldViolation::new(
                    format!("{path}.options"),
                    "is required for select fields",
                ))
            }
            _ => {}
        }
    }

    pub fn setting_defaults(&self) -> serde_json::Map<String, Value> {
        self.settings
            .iter()
            .filter(|setting| setting.kind != SETTING_SECRET)
            .filter_map(|setting| {
                setting
                    .default
                    .clone()
                    .map(|value| (setting.key.clone(), value))
            })
            .collect()
    }
}

fn is_host_pattern(pattern: &str) -> bool {
    let host = pattern.strip_prefix("*.").unwrap_or(pattern);
    let labels: Vec<&str> = host.split('.').collect();
    host.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        })
}

pub fn match_route<'a>(
    routes: &'a [RouteSpec],
    public: bool,
    method: &str,
    path: &str,
) -> Option<(&'a RouteSpec, serde_json::Map<String, Value>)> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    routes.iter().find_map(|route| {
        if route.method != method || route.public != public {
            return None;
        }
        let pattern: Vec<&str> = route.path.trim_matches('/').split('/').collect();
        if pattern.len() != segments.len() {
            return None;
        }
        let mut params = serde_json::Map::new();
        for (expected, actual) in pattern.iter().zip(&segments) {
            match expected
                .strip_prefix('{')
                .and_then(|name| name.strip_suffix('}'))
            {
                Some(name) => {
                    params.insert(name.to_owned(), Value::String((*actual).to_owned()));
                }
                None if expected == actual => {}
                None => return None,
            }
        }
        Some((route, params))
    })
}
