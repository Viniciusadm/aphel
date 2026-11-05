use serde::{Deserialize, Serialize};
use serde_json::Value;

use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::jobs::CronSchedule;
use aphel_sdk::platform::domain::access::{Permission, RecordScope, is_known_permission};
use aphel_sdk::platform::domain::custom_objects::is_custom_record_type;
use aphel_sdk::platform::domain::fields::FieldType;

pub const API_VERSION: u32 = 1;
pub const INVALID_MANIFEST: &str = "plugin_manifest_invalid";

pub const HOOK_SAVING: &str = "record.saving";
pub const HOOK_VALIDATING: &str = "record.validating";
pub const HOOK_SAVED: &str = "record.saved";
pub const HOOK_DELETING: &str = "record.deleting";
pub const HOOK_ARCHIVING: &str = "record.archiving";
pub const HOOK_COMMITTED: &str = "record.committed";
pub const HOOK_DELETED: &str = "record.deleted";
pub const HOOK_RESTORED: &str = "record.restored";
pub const HOOK_PURGED: &str = "record.purged";
pub const RECORD_HOOKS: &[&str] = &[
    HOOK_SAVING,
    HOOK_VALIDATING,
    HOOK_SAVED,
    HOOK_DELETING,
    HOOK_ARCHIVING,
    HOOK_COMMITTED,
    HOOK_DELETED,
    HOOK_RESTORED,
    HOOK_PURGED,
];
const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];
pub const SETTING_SECRET: &str = "secret";
pub const PLUGIN_EVENT_PREFIX: &str = "plugin.";
const SETTING_KINDS: &[&str] = &["number", "boolean", "text", "select", SETTING_SECRET];
pub const SETTING_SELECT: &str = "select";
const MAX_SCHEDULES: usize = 10;
const MAX_ACTIONS: usize = 20;
const MAX_EMITS: usize = 50;
const MAX_INPUTS: usize = 20;
pub const INPUT_KINDS: &[&str] = &["text", "number", "boolean", "select"];
pub const NOTIFICATION_CHANNELS: &[&str] = &["in_app", "email", "push"];
const WIDGET_SIZES: &[&str] = &["small", "medium", "large"];
const MAX_NOTIFICATIONS: usize = 20;
const MAX_WIDGETS: usize = 20;
const MAX_ALLOWED_HOSTS: usize = 20;
const MAX_OPERATIONS: usize = 100;
const MAX_KEY_LENGTH: usize = 32;
const MAX_OBJECT_KEY_LENGTH: usize = 48;
const MAX_ROLE_PRESETS: usize = 20;
pub const ENTITY_ACTIONS: &[&str] = &["read", "create", "update", "delete", "archive", "export"];
const RESERVED_ROLES: &[&str] = &["admin"];
const REMOVED_MANIFEST_FIELDS: &[(&str, &str)] = &[(
    "conflicts_with",
    "provide the contract of the module you replace through provides.contracts",
)];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub api: u32,
    #[serde(default)]
    pub layer: Option<aphel_kernel::modules::Layer>,
    #[serde(default)]
    pub runtime: Option<aphel_kernel::modules::Runtime>,
    #[serde(default)]
    pub provides: aphel_kernel::modules::Provides,
    #[serde(default)]
    pub uses: aphel_kernel::modules::Uses,
    #[serde(default)]
    pub contribute: Vec<aphel_kernel::modules::ContributionSpec>,
    #[serde(default)]
    pub requires_aphel: Option<String>,
    #[serde(default)]
    pub entities: Vec<EntityDeclaration>,
    #[serde(default)]
    pub fields: Vec<ExtensionField>,
    #[serde(default)]
    pub hooks: HooksSpec,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub routes: Vec<RouteSpec>,
    #[serde(default)]
    pub aliases: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub optional_requires: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<PermissionSpec>,
    #[serde(default)]
    pub http: HttpSpec,
    #[serde(default)]
    pub schedules: Vec<ScheduleSpec>,
    #[serde(default)]
    pub actions: Vec<ActionSpec>,
    #[serde(default)]
    pub emits: Vec<EmitSpec>,
    #[serde(default)]
    pub notifications: Vec<NotificationSpec>,
    #[serde(default)]
    pub widgets: Vec<WidgetSpec>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub role_presets: Vec<RolePresetSpec>,
    #[serde(default)]
    pub privacy: PrivacySpec,
    #[serde(default)]
    pub web: Option<WebSpec>,
    #[serde(default)]
    pub menu: MenuSpec,
    #[serde(default)]
    pub settings_permission: Option<String>,
}

pub const OWN_ENTITIES_VISIBILITY: &[&str] = &["visible", "hidden"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MenuSpec {
    #[serde(default)]
    pub flat: bool,
    #[serde(default)]
    pub own_entities: Option<String>,
}

impl MenuSpec {
    pub fn hides_own_entities(&self) -> bool {
        self.own_entities.as_deref() == Some("hidden")
    }

    pub fn is_empty(&self) -> bool {
        !self.flat && !self.hides_own_entities()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrivacySpec {
    #[serde(default)]
    pub subject_data: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolePresetSpec {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "organization_scope")]
    pub scope: String,
    #[serde(default)]
    pub home: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlannedGrant {
    pub role: String,
    pub permission: String,
    pub scope: RecordScope,
}

#[derive(Debug, Clone, Copy)]
pub struct HostCatalog<'a> {
    pub hooks: &'a [&'a str],
    pub operations: &'a [&'a str],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSpec {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "all_channels")]
    pub channels: Vec<String>,
    #[serde(default)]
    pub email_default: bool,
    #[serde(default)]
    pub push_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSpec {
    pub name: String,
    pub title: String,
    #[serde(default = "default_widget_category")]
    pub category: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default = "default_widget_size")]
    pub size: String,
    #[serde(default = "enabled")]
    pub uses_period: bool,
    #[serde(default = "empty_object")]
    pub default_settings: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSpec {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub record_types: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<InputSpec>,
    #[serde(default)]
    pub retry_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSpec {
    pub key: String,
    pub label: String,
    #[serde(rename = "type", default = "default_input_kind")]
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitSpec {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub record_types: Vec<String>,
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

pub use aphel_sdk::platform::application::fields::{EntityDeclaration, FieldDef, OptionDef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionField {
    pub record_type: String,
    #[serde(flatten)]
    pub field: FieldDef,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HooksSpec {
    #[serde(default)]
    pub subscribe: Vec<HookSubscription>,
    #[serde(default)]
    pub declare: Vec<HookDeclarationSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookDeclarationSpec {
    pub key: String,
    pub description: String,
    #[serde(default = "default_hook_kind")]
    pub kind: String,
    #[serde(default = "default_hook_record_type")]
    pub record_type: String,
}

fn default_hook_kind() -> String {
    "action".to_owned()
}

fn default_hook_record_type() -> String {
    "*".to_owned()
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
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub options: Vec<SettingOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SettingOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSpec {
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub public: Option<String>,
}

fn enabled() -> bool {
    true
}

fn organization_scope() -> String {
    RecordScope::Organization.as_str().to_owned()
}

fn default_priority() -> i32 {
    50
}

fn all_channels() -> Vec<String> {
    NOTIFICATION_CHANNELS
        .iter()
        .map(|channel| (*channel).to_owned())
        .collect()
}

fn default_widget_category() -> String {
    "plugins".to_owned()
}

fn default_widget_size() -> String {
    "medium".to_owned()
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

fn default_input_kind() -> String {
    "text".to_owned()
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
    pub fn needs_backend(&self) -> bool {
        !self.hooks.subscribe.is_empty()
            || !self.events.is_empty()
            || !self.routes.is_empty()
            || !self.schedules.is_empty()
            || !self.actions.is_empty()
            || !self.widgets.is_empty()
            || self.privacy.subject_data
    }

    pub fn parse(text: &str) -> Result<Self, AppError> {
        if let Ok(table) = text.parse::<toml::Table>() {
            for (field, hint) in REMOVED_MANIFEST_FIELDS {
                if table.contains_key(*field) {
                    return Err(AppError::validation_code(
                        INVALID_MANIFEST,
                        format!("plugin.toml is invalid: `{field}` was removed; {hint}"),
                    ));
                }
            }
        }
        toml::from_str(text).map_err(|error| {
            AppError::validation_code(INVALID_MANIFEST, format!("plugin.toml is invalid: {error}"))
        })
    }

    pub fn entity_record_type(&self, entity: &str) -> String {
        format!("{}.{entity}", self.key)
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
            || self
                .optional_requires
                .iter()
                .any(|optional| optional == key)
            || self.permissions.iter().any(|declared| declared.key == key)
            || self.entity_record_types().iter().any(|record_type| {
                key.strip_prefix(record_type.as_str())
                    .is_some_and(|action| action.starts_with('.'))
            })
    }

    pub fn entity_scope(&self, entity: &EntityDeclaration) -> Option<RecordScope> {
        entity
            .permission_scope
            .as_deref()
            .and_then(RecordScope::parse)
    }

    pub fn entity_permissions(&self) -> Vec<String> {
        self.entity_record_types()
            .iter()
            .flat_map(|record_type| {
                ENTITY_ACTIONS
                    .iter()
                    .map(move |action| format!("{record_type}.{action}"))
            })
            .collect()
    }

    fn presettable(&self, permission: &str) -> bool {
        self.permissions
            .iter()
            .any(|declared| declared.key == permission)
            || self.requires.iter().any(|required| required == permission)
            || self
                .optional_requires
                .iter()
                .any(|optional| optional == permission)
            || self
                .entity_permissions()
                .iter()
                .any(|key| key == permission)
    }

    pub fn planned_grants(&self) -> Vec<PlannedGrant> {
        let mut grants: Vec<PlannedGrant> = Vec::new();
        for declared in &self.permissions {
            for role in &declared.grant_to {
                grants.push(PlannedGrant {
                    role: role.clone(),
                    permission: declared.key.clone(),
                    scope: RecordScope::Organization,
                });
            }
        }
        for preset in &self.role_presets {
            let scope = RecordScope::parse(&preset.scope).unwrap_or(RecordScope::Organization);
            for permission in &preset.permissions {
                grants.push(PlannedGrant {
                    role: preset.key.clone(),
                    permission: permission.clone(),
                    scope,
                });
            }
        }
        grants.sort();
        grants
            .dedup_by(|left, right| left.role == right.role && left.permission == right.permission);
        grants
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

    pub fn event_type(&self, name: &str) -> String {
        format!("{PLUGIN_EVENT_PREFIX}{}.{name}", self.key)
    }

    pub fn action_key(&self, action: &str) -> String {
        format!("{PLUGIN_EVENT_PREFIX}{}.{action}", self.key)
    }

    pub fn secret_settings(&self) -> impl Iterator<Item = &SettingSpec> {
        self.settings
            .iter()
            .filter(|setting| setting.kind == SETTING_SECRET)
    }

    pub fn subscribes(&self, hook: &str, record_type: &str) -> Option<i32> {
        self.hooks
            .subscribe
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

    pub fn uses_operation(&self, operation: &str) -> bool {
        self.operations.iter().any(|declared| declared == operation)
    }

    pub fn validate(
        &self,
        compiled_modules: &[String],
        aphel_version: &str,
        catalog: HostCatalog<'_>,
    ) -> Result<(), AppError> {
        let mut violations = Vec::new();
        if self.key == "custom" || !is_key(&self.key, MAX_KEY_LENGTH) {
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
            if let Some(scope) = &entity.permission_scope
                && RecordScope::parse(scope).is_none()
            {
                violations.push(FieldViolation::new(
                    format!("{path}.permission_scope"),
                    "must be own, team or organization",
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
            if is_custom_record_type(&extension.record_type)
                && !self.entity_record_types().contains(&extension.record_type)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.record_type"),
                    "cannot extend custom objects of other plugins",
                ));
            }
            self.validate_field(&path, &extension.field, &mut violations);
        }
        for (index, declaration) in self.hooks.declare.iter().enumerate() {
            let path = format!("hooks.declare[{index}]");
            if !declaration.key.starts_with(&format!("{}.", self.key))
                || !declaration
                    .key
                    .split('.')
                    .all(|segment| is_key(segment, MAX_KEY_LENGTH))
                || self.hooks.declare[..index]
                    .iter()
                    .any(|previous| previous.key == declaration.key)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.key"),
                    "must be a unique dotted key starting with `<plugin key>.`",
                ));
            }
            if !["veto", "filter", "action"].contains(&declaration.kind.as_str()) {
                violations.push(FieldViolation::new(
                    format!("{path}.kind"),
                    "must be veto, filter or action",
                ));
            }
            if declaration.description.trim().is_empty() || declaration.description.len() > 240 {
                violations.push(FieldViolation::new(
                    format!("{path}.description"),
                    "must have between 1 and 240 characters",
                ));
            }
        }
        for (index, hook) in self.hooks.subscribe.iter().enumerate() {
            if !catalog.hooks.contains(&hook.hook.as_str())
                && !self
                    .hooks
                    .declare
                    .iter()
                    .any(|declared| declared.key == hook.hook)
            {
                violations.push(FieldViolation::new(
                    format!("hooks.subscribe[{index}].hook"),
                    "must be a hook offered by this Aphel (see GET /api/v1/wasm-runtime/host-catalog)",
                ));
            }
            if hook.record_types.is_empty() {
                violations.push(FieldViolation::new(
                    format!("hooks.subscribe[{index}].record_types"),
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
                is_known_permission(&permission)
                    || required
                        .rsplit_once('.')
                        .is_some_and(|(entity, _)| is_custom_record_type(entity))
            }) {
                violations.push(FieldViolation::new(
                    format!("requires[{index}]"),
                    "is not a known permission",
                ));
            }
        }
        for (index, optional) in self.optional_requires.iter().enumerate() {
            if Permission::parse(optional.clone()).is_err() {
                violations.push(FieldViolation::new(
                    format!("optional_requires[{index}]"),
                    "is not a valid permission",
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
        if self.role_presets.len() > MAX_ROLE_PRESETS {
            violations.push(FieldViolation::new(
                "role_presets",
                format!("can list at most {MAX_ROLE_PRESETS} role presets"),
            ));
        }
        for (index, preset) in self.role_presets.iter().enumerate() {
            let path = format!("role_presets[{index}]");
            if !is_key(&preset.key, 64)
                || RESERVED_ROLES.contains(&preset.key.as_str())
                || self.role_presets[..index]
                    .iter()
                    .any(|previous| previous.key == preset.key)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.key"),
                    "must be a unique lowercase role key other than admin",
                ));
            }
            if preset.label.trim().is_empty() || preset.label.len() > 140 {
                violations.push(FieldViolation::new(
                    format!("{path}.label"),
                    "must have between 1 and 140 characters",
                ));
            }
            if RecordScope::parse(&preset.scope).is_none() {
                violations.push(FieldViolation::new(
                    format!("{path}.scope"),
                    "must be own, team or organization",
                ));
            }
            if let Some(home) = &preset.home
                && (!home.starts_with('/') || home.starts_with("//") || home.len() > 200)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.home"),
                    "must be an application path starting with a single slash",
                ));
            }
            if preset.permissions.is_empty() {
                violations.push(FieldViolation::new(
                    format!("{path}.permissions"),
                    "must list at least one permission",
                ));
            }
            for (permission_index, permission) in preset.permissions.iter().enumerate() {
                if !self.presettable(permission) {
                    violations.push(FieldViolation::new(
                        format!("{path}.permissions[{permission_index}]"),
                        "must be a declared permission, a required permission or a permission of the plugin's entities",
                    ));
                }
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
        if self.actions.len() > MAX_ACTIONS {
            violations.push(FieldViolation::new(
                "actions",
                format!("can list at most {MAX_ACTIONS} actions"),
            ));
        }
        for (index, action) in self.actions.iter().enumerate() {
            let path = format!("actions[{index}]");
            if !is_key(&action.key, 64)
                || self.actions[..index]
                    .iter()
                    .any(|previous| previous.key == action.key)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.key"),
                    "must be a unique lowercase key",
                ));
            }
            if action.label.trim().is_empty() || action.label.len() > 120 {
                violations.push(FieldViolation::new(
                    format!("{path}.label"),
                    "must have between 1 and 120 characters",
                ));
            }
            if action.inputs.len() > MAX_INPUTS {
                violations.push(FieldViolation::new(
                    format!("{path}.inputs"),
                    format!("can list at most {MAX_INPUTS} inputs"),
                ));
            }
            for (input_index, input) in action.inputs.iter().enumerate() {
                let input_path = format!("{path}.inputs[{input_index}]");
                if !is_key(&input.key, 64)
                    || action.inputs[..input_index]
                        .iter()
                        .any(|previous| previous.key == input.key)
                {
                    violations.push(FieldViolation::new(
                        format!("{input_path}.key"),
                        "must be a unique lowercase key",
                    ));
                }
                if !INPUT_KINDS.contains(&input.kind.as_str()) {
                    violations.push(FieldViolation::new(
                        format!("{input_path}.type"),
                        format!("must be one of {}", INPUT_KINDS.join(", ")),
                    ));
                }
                if input.kind == "select" && input.options.is_empty() {
                    violations.push(FieldViolation::new(
                        format!("{input_path}.options"),
                        "is required for select inputs",
                    ));
                }
            }
        }
        if self.emits.len() > MAX_EMITS {
            violations.push(FieldViolation::new(
                "emits",
                format!("can list at most {MAX_EMITS} events"),
            ));
        }
        for (native, plugin_path) in &self.aliases {
            if !native.starts_with('/') || !plugin_path.starts_with('/') {
                violations.push(FieldViolation::new(
                    format!("aliases.{native}"),
                    "paths must start with /",
                ));
            }
        }
        if !self.aliases.is_empty() && crate::modules::held_modules(self).is_empty() {
            violations.push(FieldViolation::new(
                "aliases",
                "aliases need `provides.contracts` that holds a native module",
            ));
        }
        for (path, message) in self.provides.violations(&self.contribute) {
            violations.push(FieldViolation::new(path, message));
        }
        for (index, operation) in self.provides.operations.iter().enumerate() {
            let held = crate::modules::held_modules(self);
            if aphel_sdk::platform::modules::try_app().is_some_and(|app| {
                app.native_operation_owner(&operation.key)
                    .is_some_and(|owner| !held.iter().any(|module| module == owner))
            }) {
                violations.push(FieldViolation::new(
                    format!("provides.operations[{index}].key"),
                    "is already provided by a built-in module",
                ));
            }
        }
        let other_plugins = crate::registry::cached();
        for (index, operation) in self.provides.operations.iter().enumerate() {
            if other_plugins.plugins.iter().any(|other| {
                other.key != self.key
                    && other
                        .manifest
                        .provides
                        .operations
                        .iter()
                        .any(|provided| provided.key == operation.key)
            }) {
                violations.push(FieldViolation::new(
                    format!("provides.operations[{index}].key"),
                    "is already provided by another installed plugin",
                ));
            }
        }
        if self.operations.len() > MAX_OPERATIONS {
            violations.push(FieldViolation::new(
                "operations",
                format!("can list at most {MAX_OPERATIONS} operations"),
            ));
        }
        for (index, operation) in self.operations.iter().enumerate() {
            if !catalog.operations.contains(&operation.as_str())
                || self.operations[..index].contains(operation)
            {
                violations.push(FieldViolation::new(
                    format!("operations[{index}]"),
                    "must be a unique operation offered by this Aphel (see GET /api/v1/wasm-runtime/host-catalog)",
                ));
            }
        }
        for (index, emit) in self.emits.iter().enumerate() {
            if !is_key(&emit.name, 64)
                || self.emits[..index]
                    .iter()
                    .any(|previous| previous.name == emit.name)
            {
                violations.push(FieldViolation::new(
                    format!("emits[{index}].name"),
                    "must be a unique lowercase key",
                ));
            }
            if emit.label.trim().is_empty() || emit.label.len() > 120 {
                violations.push(FieldViolation::new(
                    format!("emits[{index}].label"),
                    "must have between 1 and 120 characters",
                ));
            }
        }
        if self.notifications.len() > MAX_NOTIFICATIONS {
            violations.push(FieldViolation::new(
                "notifications",
                format!("can list at most {MAX_NOTIFICATIONS} notifications"),
            ));
        }
        for (index, notification) in self.notifications.iter().enumerate() {
            let path = format!("notifications[{index}]");
            if !is_key(&notification.name, 48)
                || self.notifications[..index]
                    .iter()
                    .any(|previous| previous.name == notification.name)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.name"),
                    "must be a unique lowercase key",
                ));
            }
            if notification.label.trim().is_empty() || notification.label.len() > 120 {
                violations.push(FieldViolation::new(
                    format!("{path}.label"),
                    "must have between 1 and 120 characters",
                ));
            }
            if notification.channels.is_empty()
                || notification
                    .channels
                    .iter()
                    .any(|channel| !NOTIFICATION_CHANNELS.contains(&channel.as_str()))
            {
                violations.push(FieldViolation::new(
                    format!("{path}.channels"),
                    format!("must list some of {}", NOTIFICATION_CHANNELS.join(", ")),
                ));
            }
        }
        if self.widgets.len() > MAX_WIDGETS {
            violations.push(FieldViolation::new(
                "widgets",
                format!("can list at most {MAX_WIDGETS} widgets"),
            ));
        }
        for (index, widget) in self.widgets.iter().enumerate() {
            let path = format!("widgets[{index}]");
            if !is_key(&widget.name, 48)
                || self.widgets[..index]
                    .iter()
                    .any(|previous| previous.name == widget.name)
            {
                violations.push(FieldViolation::new(
                    format!("{path}.name"),
                    "must be a unique lowercase key",
                ));
            }
            if widget.title.trim().is_empty() || widget.title.len() > 120 {
                violations.push(FieldViolation::new(
                    format!("{path}.title"),
                    "must have between 1 and 120 characters",
                ));
            }
            if !WIDGET_SIZES.contains(&widget.size.as_str()) {
                violations.push(FieldViolation::new(
                    format!("{path}.size"),
                    format!("must be one of {}", WIDGET_SIZES.join(", ")),
                ));
            }
            if !is_key(&widget.category, 64) {
                violations.push(FieldViolation::new(
                    format!("{path}.category"),
                    "must be a lowercase key",
                ));
            }
            if !widget.default_settings.is_object() {
                violations.push(FieldViolation::new(
                    format!("{path}.default_settings"),
                    "must be a table",
                ));
            }
            if let Some(permission) = &widget.permission
                && !Permission::parse(permission.clone())
                    .is_ok_and(|permission| self.allows(&permission))
            {
                violations.push(FieldViolation::new(
                    format!("{path}.permission"),
                    "must be one of the plugin's capabilities (requires, declared permissions or its entities)",
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
            if setting.kind == SETTING_SELECT {
                if setting.options.is_empty() {
                    violations.push(FieldViolation::new(
                        format!("settings[{index}].options"),
                        "is required for select settings",
                    ));
                }
                if let Some(default) = setting.default.as_ref().and_then(Value::as_str)
                    && !setting.options.iter().any(|option| option.value == default)
                {
                    violations.push(FieldViolation::new(
                        format!("settings[{index}].default"),
                        "must be one of the options",
                    ));
                }
            } else if !setting.options.is_empty() {
                violations.push(FieldViolation::new(
                    format!("settings[{index}].options"),
                    "is only allowed for select settings",
                ));
            }
            if setting.kind == SETTING_SECRET && setting.default.is_some() {
                violations.push(FieldViolation::new(
                    format!("settings[{index}].default"),
                    "secret settings cannot have a default",
                ));
            }
        }
        if let Some(permission) = &self.settings_permission
            && !self
                .permissions
                .iter()
                .any(|declared| &declared.key == permission)
        {
            violations.push(FieldViolation::new(
                "settings_permission",
                "must be a permission declared in permissions",
            ));
        }
        self.validate_menu(&mut violations);
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

    fn validate_menu(&self, violations: &mut Vec<FieldViolation>) {
        if let Some(visibility) = &self.menu.own_entities
            && !OWN_ENTITIES_VISIBILITY.contains(&visibility.as_str())
        {
            violations.push(FieldViolation::new(
                "menu.own_entities",
                format!("must be one of {}", OWN_ENTITIES_VISIBILITY.join(", ")),
            ));
        }
    }

    fn grantable(&self, permission: &Permission) -> bool {
        self.entity_permissions()
            .iter()
            .any(|key| key == permission.as_str())
            || is_known_permission(permission)
            || permission
                .as_str()
                .rsplit_once('.')
                .is_some_and(|(entity, _)| is_custom_record_type(entity))
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
        if let Some(kind) = FieldType::parse(&field.field_type) {
            match field.config() {
                Ok(config) => {
                    for violation in config.validate(kind) {
                        violations.push(FieldViolation::new(
                            format!("{path}.{}", violation.property.as_str()),
                            violation.message,
                        ));
                    }
                }
                Err(errors) => {
                    for error in errors {
                        violations.push(FieldViolation::new(
                            format!("{path}.{}", error.property),
                            error.message,
                        ));
                    }
                }
            }
        }
        if field.unique
            && FieldType::parse(&field.field_type).is_some_and(|kind| !kind.supports_unique())
        {
            violations.push(FieldViolation::new(
                format!("{path}.unique"),
                "is only supported by text, number, url, email and phone fields",
            ));
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
