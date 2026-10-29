pub mod automation;
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
use aphel_kernel::records::{RecordDeleting, RecordSaved, RecordSaving, RecordValidating};

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
        .consumer(|env| Arc::new(events::PluginEventsConsumer::new(env.pool.clone())))
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
        app.contribute::<dyn AutomationActionSource>(Arc::new(automation::PluginActions))
            .contribute::<dyn AutomationTriggerSource>(Arc::new(automation::PluginTriggers));
        Ok(())
    }
}
