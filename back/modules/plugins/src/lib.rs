pub mod automation;
pub mod bridge;
pub mod committed;
pub mod compiled;
pub mod effects;
pub mod events;
pub mod hooks;
pub mod host;
pub mod http;
pub mod manifest;
pub mod net;
pub mod notify;
pub mod package;
pub mod registry;
pub mod runtime;
pub mod schedule;
pub mod schema;
pub mod store;
pub mod widgets;

use std::sync::Arc;

use aphel_kernel::application::automations::{AutomationActionSource, AutomationTriggerSource};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module};
use aphel_kernel::modules::{PluginHookDispatcher, PluginOperation};
use aphel_kernel::records::{
    RecordDeleted, RecordDeleting, RecordPurged, RecordRestored, RecordSaved, RecordSaving,
    RecordValidating,
};

pub const KEY: &str = "plugins";

const HOOK_PRIORITY: i32 = 50;

pub struct PluginsModule;

impl Module for PluginsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Plugins", APHEL_VERSION)
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
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.filter::<RecordSaving, _>(HOOK_PRIORITY, |ctx, saving| {
            Box::pin(hooks::saving(ctx, saving))
        })
        .on::<RecordValidating, _>(HOOK_PRIORITY, |ctx, changed| {
            Box::pin(hooks::validating(ctx, changed))
        })
        .on::<RecordSaved, _>(HOOK_PRIORITY, |ctx, changed| Box::pin(hooks::saved(ctx, changed)))
        .on::<RecordDeleting, _>(HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::deleting(ctx, removed))
        })
        .on::<RecordDeleted, _>(HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_DELETED, removed))
        })
        .on::<RecordRestored, _>(HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_RESTORED, removed))
        })
        .on::<RecordPurged, _>(HOOK_PRIORITY, |ctx, removed| {
            Box::pin(hooks::removed(ctx, manifest::HOOK_PURGED, removed))
        })
        .consumer(|env| Arc::new(events::PluginEventsConsumer::new(env.pool.clone())))
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
            http::router()
        })
        .public_routes(|_| http::public_router());
        app.contribute::<dyn aphel_kernel::modules::runtime_catalog::RuntimeCatalogSource>(
            Arc::new(registry::PublicEvents),
        );
        app.contribute::<dyn aphel_kernel::application::public_events::RuntimePublicEventSource>(
            Arc::new(registry::PublicEvents),
        );
        app.contribute::<dyn AutomationActionSource>(Arc::new(automation::PluginActions))
            .contribute::<dyn AutomationTriggerSource>(Arc::new(automation::PluginTriggers))
            .provide::<dyn PluginHookDispatcher>(Arc::new(bridge::Dispatcher));
        for operation in bridge::record_operations() {
            app.contribute::<PluginOperation>(operation);
        }
        Ok(())
    }
}
