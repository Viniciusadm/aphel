use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::access::permission_catalog;
use crate::modules::builder::App;
use crate::modules::hooks::HookUsage;
use crate::modules::operations::OperationEntry;
use crate::modules::plugin_bridge::{PluginHook, PluginOperation};

pub trait CatalogContributor: Send + Sync {
    fn hooks(&self) -> Vec<HookEntry>;
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ContractInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub provider: &'static str,
    pub keys: Vec<ContractKeyInfo>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ContractKeyInfo {
    pub key: &'static str,
    pub kind: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PluginEntry {
    pub key: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub layer: Option<&'static str>,
    pub runtime: &'static str,
    pub required: bool,
    pub enabled: bool,
    pub provides: Vec<String>,
    pub requires: Vec<String>,
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OperationInfo {
    pub key: &'static str,
    pub module: &'static str,
    pub description: &'static str,
    pub effect: &'static str,
    pub permission: Option<&'static str>,
    pub idempotent: bool,
    pub durable: bool,
    pub expose_automation: bool,
    pub expose_bulk: bool,
    pub expose_ai: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HookEntry {
    pub key: String,
    pub module: &'static str,
    pub kind: String,
    pub record_type: Option<String>,
    pub args: Option<String>,
    pub description: String,
    pub listeners: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct EventEntry {
    pub event_type: &'static str,
    pub resource: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PermissionEntry {
    pub permission: String,
    pub record_scoped: bool,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct FieldTypeEntry {
    pub key: &'static str,
    pub provider: &'static str,
    pub storage: &'static str,
    pub config: serde_json::Value,
    pub operators: Vec<&'static str>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReferenceKindEntry {
    pub key: &'static str,
    pub provider: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CatalogDocument {
    pub version: &'static str,
    pub field_types: Vec<FieldTypeEntry>,
    pub reference_kinds: Vec<ReferenceKindEntry>,
    pub contracts: Vec<ContractInfo>,
    pub plugins: Vec<PluginEntry>,
    pub operations: Vec<OperationInfo>,
    pub hooks: Vec<HookEntry>,
    pub events: Vec<EventEntry>,
    pub permissions: Vec<PermissionEntry>,
    #[serde(flatten)]
    pub extensions: std::collections::BTreeMap<String, serde_json::Value>,
}

pub fn build(app: &App) -> CatalogDocument {
    CatalogDocument {
        version: crate::modules::builder::APHEL_VERSION,
        field_types: app
            .all_contributions::<crate::application::fields::FieldTypeDefinition>()
            .into_iter()
            .map(|(provider, definition)| FieldTypeEntry {
                key: definition.key,
                provider,
                storage: definition.storage().column_name(),
                config: definition.config.clone(),
                operators: definition
                    .operators
                    .iter()
                    .map(|operator| operator.as_str())
                    .collect(),
                enabled: app.is_enabled(provider),
            })
            .collect(),
        reference_kinds: app
            .all_contributions::<dyn crate::modules::ReferenceProvider>()
            .into_iter()
            .map(|(provider, resolver)| ReferenceKindEntry {
                key: resolver.kind().key(),
                provider,
                enabled: app.is_enabled(provider),
            })
            .collect(),
        contracts: contracts(app),
        plugins: plugins(app),
        operations: operations(app),
        hooks: hooks(app),
        events: events(),
        permissions: permissions(),
        extensions: app
            .contributions::<dyn CatalogExtension>()
            .into_iter()
            .map(|source| source.project(app))
            .collect(),
    }
}

fn contracts(app: &App) -> Vec<ContractInfo> {
    let mut entries: Vec<ContractInfo> = app
        .all_contributions::<aphel_contract::ContractDescriptor>()
        .into_iter()
        .map(|(provider, descriptor)| ContractInfo {
            name: descriptor.contract.name(),
            version: descriptor.contract.version(),
            provider,
            keys: descriptor
                .entries
                .iter()
                .map(|entry| ContractKeyInfo {
                    key: entry.key,
                    kind: entry.kind.label(),
                    description: entry.description,
                })
                .collect(),
        })
        .collect();
    entries.sort_by_key(|entry| entry.name);
    entries
}

fn plugins(app: &App) -> Vec<PluginEntry> {
    let mut entries: Vec<PluginEntry> = app
        .modules()
        .iter()
        .map(|loaded| {
            let manifest = &loaded.manifest;
            PluginEntry {
                key: manifest.key(),
                name: manifest.name,
                version: manifest.version,
                layer: manifest.layer.map(|layer| layer.as_str()),
                runtime: manifest.runtime.as_str(),
                required: manifest.required,
                enabled: app.is_enabled(manifest.key()),
                provides: manifest.provides.contracts.clone(),
                requires: manifest.uses.required.clone(),
                optional: manifest.uses.optional.clone(),
            }
        })
        .collect();
    entries.sort_by_key(|entry| entry.key);
    entries
}

fn operations(app: &App) -> Vec<OperationInfo> {
    let mut by_key: BTreeMap<&'static str, OperationInfo> = BTreeMap::new();
    for (module, operation) in app.all_contributions::<PluginOperation>() {
        by_key.insert(
            operation.key,
            OperationInfo {
                key: operation.key,
                module,
                description: operation.description,
                effect: if operation.external_query {
                    "external"
                } else if operation.writes {
                    "write"
                } else {
                    "read"
                },
                permission: None,
                idempotent: false,
                durable: operation.durable,
                expose_automation: false,
                expose_bulk: false,
                expose_ai: false,
                enabled: app.is_enabled(module),
            },
        );
    }
    for (module, entry) in app.all_contributions::<OperationEntry>() {
        by_key.insert(
            entry.key,
            OperationInfo {
                key: entry.key,
                module,
                description: entry.spec.description,
                effect: entry.spec.effect.as_str(),
                permission: entry.spec.permission,
                idempotent: entry.spec.idempotent,
                durable: entry.spec.durable,
                expose_automation: entry.spec.automation.is_some(),
                expose_bulk: entry.spec.expose.bulk,
                expose_ai: entry.spec.expose.ai,
                enabled: app.is_enabled(module),
            },
        );
    }
    by_key.into_values().collect()
}

fn hooks(app: &App) -> Vec<HookEntry> {
    let bus = app.hooks();
    let mut entries: Vec<HookEntry> = bus
        .declarations()
        .iter()
        .map(|declaration| {
            let mut listeners: Vec<_> = bus
                .describe()
                .iter()
                .filter(|info| info.hook == declaration.hook)
                .collect();
            listeners.sort_by_key(|info| (info.priority, info.module));
            HookEntry {
                key: declaration.hook.to_owned(),
                module: declaration.module,
                kind: declaration
                    .usages
                    .iter()
                    .map(HookUsage::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
                record_type: None,
                args: Some(declaration.args.to_owned()),
                description: declaration.description.to_owned(),
                listeners: listeners
                    .into_iter()
                    .map(|info| {
                        format!("{} ({} {})", info.module, info.kind.as_str(), info.priority)
                    })
                    .collect(),
                enabled: app.is_enabled(declaration.module),
            }
        })
        .collect();
    for (module, hook) in app.all_contributions::<PluginHook>() {
        if entries.iter().any(|entry| entry.key == hook.key) {
            continue;
        }
        entries.push(HookEntry {
            key: hook.key.to_owned(),
            module,
            kind: hook.kind.as_str().to_owned(),
            record_type: Some(hook.record_type.to_owned()),
            args: None,
            description: hook.description.to_owned(),
            listeners: Vec::new(),
            enabled: app.is_enabled(module),
        });
    }
    for (_, contributor) in app.all_contributions::<dyn CatalogContributor>() {
        entries.extend(contributor.hooks());
    }
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    entries
}

fn events() -> Vec<EventEntry> {
    let mut entries: Vec<EventEntry> = crate::application::public_events::public_event_catalog()
        .map(|definition| EventEntry {
            event_type: definition.event_type,
            resource: definition.resource.key(),
            description: definition.description,
        })
        .collect();
    entries.sort_by_key(|entry| entry.event_type);
    entries
}

fn permissions() -> Vec<PermissionEntry> {
    let mut entries: Vec<PermissionEntry> = permission_catalog()
        .into_iter()
        .map(|definition| PermissionEntry {
            permission: definition.permission.as_str().to_owned(),
            record_scoped: definition.record_scoped,
            label: definition.label,
        })
        .collect();
    entries.sort_by(|left, right| left.permission.cmp(&right.permission));
    entries
}

pub trait CatalogExtension: Send + Sync {
    fn project(&self, app: &App) -> (String, serde_json::Value);
}
