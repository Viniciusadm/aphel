pub mod builder;
pub(crate) mod catalog;
pub(crate) mod command;
pub(crate) mod enabled;
pub(crate) mod global;
pub mod hooks;
pub mod intern;
pub(crate) mod manifest;
pub mod migrations;
pub(crate) mod module;
pub mod operations;
pub mod plugin_bridge;
pub(crate) mod plugin_manifest;
pub(crate) mod points;
pub(crate) mod routes;
pub mod runtime;
mod runtime_contribution;
pub(crate) mod services;
pub(crate) mod settings;
pub mod unified_catalog;
pub mod worker;

pub use builder::{App, AppBuilder, Contribution, LoadedModule, RouteMount, RuntimeBatch};
pub use catalog::{Catalog, OpenApiTag, PermissionGrant};
pub use command::{CommandResult, ModuleCommand, connect_with_modules};
pub use enabled::EnabledModules;
pub use global::{app, install_app, try_app};
pub use hooks::{
    BoxFuture, DEFAULT_PRIORITY, Hook, HookBus, HookCtx, HookDeclaration, HookInfo, HookRef,
    HookUsage,
};
pub use manifest::{ManifestBase, Migration, SettingField, SettingKind, SettingOption};
pub use module::{Manifest, Module, ModuleCtx};
pub use operations::{
    AutomationBinding, CONTRACT_UNAVAILABLE, Effect, Exposure, OperationEntry, OperationSpec,
    ReferenceCheck, TargetSelector,
};
pub use plugin_bridge::{
    HookForwarding, OperationFuture, OperationHandler, OperationPage, PLUGIN_HOOK_PRIORITY,
    PluginHook, PluginHookArgs, PluginHookCall, PluginHookDispatcher, PluginHookFilter,
    PluginHookKind, PluginOperation, forward_plugin_hook, operation_input, operation_output,
};
pub use plugin_manifest::{
    ContributionSpec, INVALID_PLUGIN_MANIFEST, Layer, PluginManifest, ProvidedEffect,
    ProvidedOperation, Provides, Runtime, Uses,
};
pub use points::{
    FileReferenceSource, HealthSource, HttpRequestPolicy, ReferenceKeySource, ReferenceProvider,
    lookup_exists,
};
pub use routes::{MODULE_DISABLED, RouteTakeover, module_disabled};
pub use runtime_contribution::RuntimeContribution;
pub use services::Services;
pub use settings::{module_secret_key, module_settings};
pub use worker::{WorkerEnv, WorkerParts};

pub mod runtime_catalog;
pub mod toggle_guard;
