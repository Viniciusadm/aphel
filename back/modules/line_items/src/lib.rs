mod dataset_seed;
pub mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod hooks;
pub mod http;
mod port;
pub(crate) mod recurring;
pub(crate) mod section;
pub mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::RecordValidating;
use aphel_sdk::configuration::ConfigSectionFactory;

pub const KEY: &str = "line_items";

pub struct LineItemsModule;

impl Module for LineItemsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Line items", APHEL_VERSION)
            .description("Quantity, pricing, discounts and totals attached to records")
            .enabled_by_default()
            .owned_tables(&["line_item_settings", "line_items"])
            .migration(Migration::new(
                1,
                "create line items",
                include_str!("../migrations/0001_create_line_items.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::LineItemsSeed,
        ));
        app.contract(aphel_sdk::line_items::DESCRIPTOR);
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable("line_items"),
        ));
        app.on(aphel_kernel::records::RecordPurged, 10, |ctx, args| {
            Box::pin(async move {
                sqlx::query("DELETE FROM line_items WHERE record_type = ? AND record_id = ?")
                    .bind(args.record_type.as_str())
                    .bind(args.record.record_id.to_string())
                    .execute(&mut **ctx.tx)
                    .await?;
                Ok(())
            })
        });
        app.permission(
            aphel_kernel::domain::access::PermissionDefinition::administrative(
                aphel_kernel::domain::access::Permission::from_static("line_items.read"),
            ),
        )
        .grant_to(["admin", "manager", "member"]);
        app.provide::<dyn aphel_sdk::line_items::LineItemsPort>(Arc::new(port::Port))
            .openapi_tag(http::TAG, "Scoped record line items and total policy")
            .routes(|_| http::router())
            .on(
                RecordValidating,
                hooks::CURRENCY_LOCK_PRIORITY,
                |ctx, args| Box::pin(hooks::block_currency_change_with_items(ctx, args)),
            );
        app.contribute(Arc::new(ConfigSectionFactory::new(
            55,
            |pool, _registry| Arc::new(section::LineItemsSection::new(pool.clone())),
        )));
        exposed_operations::register(app);
        Ok(())
    }
}
