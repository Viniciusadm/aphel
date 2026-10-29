pub mod builder;
pub mod catalog;
pub mod command;
pub mod enabled;
pub mod global;
pub mod hooks;
pub mod manifest;
pub mod migrations;
pub mod module;
pub mod plugin_bridge;
pub mod routes;
pub mod runtime;
pub mod runtime_extensions;
pub mod services;
pub mod settings;
pub mod worker;

pub use builder::{App, AppBuilder, Contribution, LoadedModule, RouteMount};
pub use catalog::{Catalog, OpenApiTag, PermissionGrant};
pub use command::{CommandResult, ModuleCommand, connect_with_modules};
pub use enabled::EnabledModules;
pub use global::{app, install_app, try_app};
pub use hooks::{
    BoxFuture, DEFAULT_PRIORITY, Hook, HookBus, HookCtx, HookDeclaration, HookInfo, HookUsage,
};
pub use manifest::{ManifestBase, Migration, SettingField, SettingKind, SettingOption};
pub use module::{Manifest, Module, ModuleCtx};
pub use plugin_bridge::{
    OperationFuture, OperationPage, PLUGIN_HOOK_PRIORITY, PluginHook, PluginHookCall,
    PluginHookDispatcher, PluginHookKind, PluginOperation, forward_plugin_hook, operation_input,
    operation_output,
};
pub use routes::{MODULE_DISABLED, module_disabled};
pub use services::Services;
pub use settings::{module_secret_key, module_settings};
pub use worker::{WorkerEnv, WorkerParts};

pub mod runtime_catalog;
