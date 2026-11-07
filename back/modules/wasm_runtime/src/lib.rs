pub mod automation;
pub mod bridge;
mod catalog;
pub mod changes;
pub mod committed;
pub mod compiled;
pub mod conflicts;
pub mod data_subject;
mod dependents;
pub mod directory;
pub mod effects;
pub mod events;
pub mod files;
pub mod grants;
pub mod hooks;
pub mod host;
pub mod http;
pub mod manifest;
pub mod modules;
pub mod net;
pub mod notify;
pub mod package;
pub mod registry;
pub mod runtime;
pub mod schedule;
pub mod schema;
pub mod signing;
pub mod store;
pub mod takeover;
pub mod ui;
mod ui_http;
mod ui_tokens;
pub mod widgets;

use std::sync::Arc;

use aphel_contract_automation::application::{AutomationActionSource, AutomationTriggerSource};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module};
use aphel_kernel::modules::{PluginHookDispatcher, PluginOperation};
use aphel_kernel::records::{
    RecordArchiving, RecordDeleted, RecordDeleting, RecordPurged, RecordRestored, RecordSaved,
    RecordSaving, RecordValidating,
};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub const KEY: &str = "wasm_runtime";

const HOOK_PRIORITY: i32 = 50;

pub struct WasmRuntimeModule;

impl Module for WasmRuntimeModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "WebAssembly runtime", APHEL_VERSION)
            .description(
                "Runtime plugins: install .aphelplugin packages (WebAssembly backend and web bundle) without rebuilding or restarting Aphel",
            )
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create plugins",
                include_str!("../migrations/0001_create_plugins.sql"),
            ))
            .migration(Migration::new(
                2,
                "create plugins compiled",
                include_str!("../migrations/0002_create_plugins_compiled.sql"),
            ))
            .migration(Migration::new(3, "populate permission metadata", include_str!("../migrations/0003_permission_metadata.sql")))
            .migration(Migration::new(4, "durable plugin effects", include_str!("../migrations/0004_durable_effects.sql")))
            .migration(Migration::new(5, "external operation query results", include_str!("../migrations/0005_external_queries.sql")))
            .migration(Migration::new(6, "plugin change feed", include_str!("../migrations/0006_create_plugins_changes.sql")))
            .migration(Migration::new(7, "plugin module conflicts", include_str!("../migrations/0007_create_plugins_module_conflicts.sql")))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::platform::modules::RouteTakeover>(std::sync::Arc::new(
            takeover::AliasTakeover,
        ));
        app.provide::<dyn aphel_sdk::platform::infrastructure::deferred_authority::DeferredPluginPolicy>(
            std::sync::Arc::new(registry::DeferredPluginPolicy),
        );
        app.filter(RecordSaving, HOOK_PRIORITY, |ctx, saving| {
            Box::pin(hooks::saving(ctx, saving))
        })
        .on(RecordValidating, HOOK_PRIORITY, |ctx, changed| {
            Box::pin(hooks::validating(ctx, changed))
        })
        .on(RecordSaved, HOOK_PRIORITY, |ctx, changed| Box::pin(hooks::saved(ctx, changed)))
        .on(RecordDeleting, HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::deleting(ctx, removed))
        })
        .on(RecordArchiving, HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_ARCHIVING, removed))
        })
        .on(RecordDeleted, HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_DELETED, removed))
        })
        .on(RecordRestored, HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_RESTORED, removed))
        })
        .on(RecordPurged, HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_PURGED, removed))
        })
        .consumer(|env| Arc::new(events::PluginEventsConsumer::new(env.pool.clone())))
        .consumer(|env| Arc::new(changes::ChangeFeedConsumer::new(env.pool.clone())))
        .job(|env| committed::Run::shared(env.pool.clone()))
        .job(|env| events::Run::shared(env.pool.clone()))
        .job(|env| effects::Run::shared(env.pool.clone()))
        .job(|env| schedule::Tick::shared(env.pool.clone()))
        .job(|env| schedule::Run::shared(env.pool.clone()))
        .schedule(schedule::tick_schedule()?)
        .openapi_tag(
            http::TAG,
            "Runtime plugins: install, enable, disable, configure and uninstall .aphelplugin packages, and call their routes",
        )
        .routes(|state| {
            if let Ok(pool) = state.db() {
                registry::spawn_refresh(pool);
            }
            http::router().merge(ui_http::router())
        })
        .public_routes(|_| http::public_router().merge(ui_http::public_router()))
        .root_routes(|_| http::root_router());
        app.contribute::<dyn aphel_sdk::platform::modules::toggle_guard::ModuleToggleGuard>(
            Arc::new(modules::PluginModuleGuard),
        );
        app.contribute::<dyn aphel_sdk::platform::modules::runtime_catalog::RuntimeCatalogSource>(
            Arc::new(registry::PublicEvents),
        );
        app.contribute::<dyn aphel_sdk::platform::application::public_events::RuntimePublicEventSource>(
            Arc::new(registry::PublicEvents),
        );
        app.contribute::<dyn aphel_sdk::privacy::DataSubjectSource>(Arc::new(
            data_subject::PluginDataSubjects,
        ));
        app.contribute::<dyn AutomationActionSource>(Arc::new(automation::PluginActions))
            .contribute::<dyn AutomationTriggerSource>(Arc::new(automation::PluginTriggers))
            .provide::<dyn PluginHookDispatcher>(Arc::new(bridge::Dispatcher))
            .provide::<dyn aphel_sdk::platform::application::organization_profile::UiPluginDirectory>(Arc::new(
                ui_http::PluginUiDirectory,
            ));
        app.contribute::<dyn aphel_sdk::catalog::CatalogContributor>(Arc::new(
            catalog::PluginCatalog,
        ));
        for operation in bridge::record_operations() {
            app.contribute::<PluginOperation>(operation);
        }
        Ok(())
    }
}
