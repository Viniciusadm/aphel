use crate::modules::manifest::{SettingField, SettingKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::api::dto::common::timestamp as instant;
use crate::application::module_admin::{DependencyStatus, ModuleDataExport, ModuleView};

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleCompatibilityResponse {
    #[schema(example = "compatible")]
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModulePermissionResponse {
    pub permission: String,
    pub label: String,
    pub record_scoped: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleEventResponse {
    pub event_type: String,
    pub description: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleSettingOptionResponse {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleSettingFieldResponse {
    pub key: String,
    pub label: String,
    pub help: Option<String>,
    #[serde(rename = "type")]
    #[schema(example = "text")]
    pub field_type: String,
    pub required: bool,
    #[schema(value_type = Option<Object>)]
    pub default: Option<Value>,
    pub options: Vec<ModuleSettingOptionResponse>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub record_type: Option<String>,
    pub field_types: Vec<String>,
}

impl From<&SettingField> for ModuleSettingFieldResponse {
    fn from(field: &SettingField) -> Self {
        let (options, min, max, record_type, field_types) = match &field.kind {
            SettingKind::Select { options } => (
                options
                    .iter()
                    .map(|option| ModuleSettingOptionResponse {
                        value: option.value.to_owned(),
                        label: option.label.to_owned(),
                    })
                    .collect(),
                None,
                None,
                None,
                Vec::new(),
            ),
            SettingKind::Number { min, max } => (Vec::new(), *min, *max, None, Vec::new()),
            SettingKind::Field {
                record_type,
                field_types,
            } => (
                Vec::new(),
                None,
                None,
                Some((*record_type).to_owned()),
                field_types
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
            ),
            _ => (Vec::new(), None, None, None, Vec::new()),
        };
        Self {
            key: field.key.to_owned(),
            label: field.label.to_owned(),
            help: field.help.map(str::to_owned),
            field_type: field.kind.as_str().to_owned(),
            required: field.required,
            default: field.default.clone(),
            options,
            min,
            max,
            record_type,
            field_types,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleDependencyResponse {
    pub key: String,
    pub requirement: String,
    pub installed: bool,
    pub enabled: bool,
}

impl From<DependencyStatus> for ModuleDependencyResponse {
    fn from(status: DependencyStatus) -> Self {
        Self {
            key: status.key,
            requirement: status.requirement,
            installed: status.installed,
            enabled: status.enabled,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicModuleResponse {
    #[schema(example = "demo")]
    pub key: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicPlatformResponse {
    pub plugin_management: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleResponse {
    #[schema(example = "proposals")]
    pub key: String,
    pub name: String,
    pub description: String,
    #[schema(example = "0.1.0")]
    pub version: String,
    pub min_aphel_version: String,
    pub compatibility: ModuleCompatibilityResponse,
    pub required: bool,
    pub enabled: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
    pub enabled_at: Option<String>,
    pub disabled_at: Option<String>,
    pub data_exported_at: Option<String>,
    pub data_purged_at: Option<String>,
    pub permissions: Vec<ModulePermissionResponse>,
    pub events: Vec<ModuleEventResponse>,
    pub settings_schema: Vec<ModuleSettingFieldResponse>,
    #[schema(value_type = Object)]
    pub settings: Map<String, Value>,
    pub configured_secrets: Vec<String>,
    pub dependencies: Vec<ModuleDependencyResponse>,
    pub enabled_dependents: Vec<String>,
    pub held_by: Option<String>,
}

impl From<ModuleView> for ModuleResponse {
    fn from(view: ModuleView) -> Self {
        let enabled = view.enabled();
        let manifest = &view.descriptor.manifest;
        let state = view.state.as_ref();
        let installed_version = state.map(|state| state.installed_version.clone());
        Self {
            key: manifest.key.to_owned(),
            name: manifest.name.to_owned(),
            description: manifest.description.to_owned(),
            version: manifest.version.to_owned(),
            min_aphel_version: manifest.min_aphel_version.to_owned(),
            compatibility: ModuleCompatibilityResponse {
                status: view.descriptor.compatibility.code().to_owned(),
                detail: view.descriptor.compatibility.detail(),
            },
            required: view.descriptor.required,
            enabled,
            update_available: enabled
                && installed_version
                    .as_deref()
                    .is_some_and(|installed| installed != manifest.version),
            installed_version,
            enabled_at: state.and_then(|state| state.enabled_at).map(instant),
            disabled_at: state.and_then(|state| state.disabled_at).map(instant),
            data_exported_at: state.and_then(|state| state.data_exported_at).map(instant),
            data_purged_at: state.and_then(|state| state.data_purged_at).map(instant),
            permissions: manifest
                .permissions
                .iter()
                .map(|declaration| ModulePermissionResponse {
                    permission: declaration.permission.to_owned(),
                    label: declaration.label.to_owned(),
                    record_scoped: declaration.record_scoped,
                })
                .collect(),
            events: manifest
                .events
                .iter()
                .map(|declaration| ModuleEventResponse {
                    event_type: declaration.event_type.to_owned(),
                    description: declaration.description.to_owned(),
                })
                .collect(),
            settings_schema: manifest.settings.iter().map(Into::into).collect(),
            settings: view.settings.clone(),
            configured_secrets: view.configured_secrets.clone(),
            dependencies: view.dependencies.into_iter().map(Into::into).collect(),
            enabled_dependents: view.enabled_dependents,
            held_by: view.held_by,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateModuleSettingsRequest {
    #[schema(value_type = Object)]
    pub settings: Map<String, Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgeModuleDataRequest {
    #[schema(example = "proposals")]
    pub confirm: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleDataExportResponse {
    pub key: String,
    pub version: String,
    pub exported_at: String,
    #[schema(value_type = Object)]
    pub data: Value,
}

impl From<ModuleDataExport> for ModuleDataExportResponse {
    fn from(export: ModuleDataExport) -> Self {
        Self {
            key: export.key,
            version: export.version,
            exported_at: instant(export.exported_at),
            data: export.data,
        }
    }
}
