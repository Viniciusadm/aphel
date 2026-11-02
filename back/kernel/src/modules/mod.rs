pub mod builder;
pub mod catalog;
pub mod command;
pub mod enabled;
pub mod global;
pub mod hooks;
pub mod intern;
pub mod manifest;
pub mod migrations;
pub mod module;
pub(crate) mod operation_action;
pub mod operations;
pub mod plugin_bridge;
pub mod plugin_manifest;
pub mod routes;
pub mod runtime;
pub mod services;
pub mod settings;
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
    OperationFuture, OperationHandler, OperationPage, PLUGIN_HOOK_PRIORITY, PluginHook,
    PluginHookCall, PluginHookDispatcher, PluginHookKind, PluginOperation, forward_plugin_hook,
    operation_input, operation_output,
};
pub use plugin_manifest::{
    INVALID_PLUGIN_MANIFEST, Layer, PluginManifest, Provides, Runtime, Uses,
};
pub use routes::{MODULE_DISABLED, module_disabled};
pub use services::Services;
pub use settings::{module_secret_key, module_settings};
pub use worker::{WorkerEnv, WorkerParts};

pub mod runtime_catalog;
pub mod toggle_guard;
