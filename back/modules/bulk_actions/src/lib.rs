pub(crate) mod dto;
pub(crate) mod http;
pub(crate) mod model;
pub(crate) mod notifications;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod targets;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};

pub(crate) use model::*;
pub(crate) use runtime::BulkRunHandler;

pub(crate) const KEY: &str = "bulk_actions";

pub struct BulkActionsModule;

impl Module for BulkActionsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Bulk actions", APHEL_VERSION)
            .description("Bulk actions over selected records or every record of a filter, with progress and report")
            .enabled_by_default()
            .depends_on(exports::KEY, "*")
            .plugin_toml(include_str!("../plugin.toml"))
            .owned_tables(&["bulk_operation_items", "bulk_operations"])
            .migration(Migration::new(
                1,
                "create bulk operations",
                include_str!("../migrations/0001_create_bulk_operations.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::fields::EntityKeyAliases(&[(
                "bulk_operation",
                "bulk_actions.bulk_operation",
            )]),
        ));
        app.openapi_tag(http::TAG, "Bulk actions over selected records or every record of a filter, with progress and report")
            .routes(|_| http::router());
        for definition in notifications::notification_kinds() {
            app.contribute(std::sync::Arc::new(definition));
        }
        app.consumer(|env| {
            notifications::BulkNotificationConsumer::shared(
                env.pool.clone(),
                env.record_types.clone(),
            )
        });
        app.worker(|env| WorkerParts::default().handler(BulkRunHandler::shared(env.state())));
        Ok(())
    }
}
