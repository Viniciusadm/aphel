pub mod application;
pub mod dto;
mod http;
pub mod infrastructure;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "validation_rules";
pub struct ValidationRulesModule;
impl Module for ValidationRulesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "ValidationRules",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create validation rules",
            include_str!("../migrations/2550_create_validation_rules.sql"),
        ))
        .owned_tables(&["validation_rules"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(http::TAG, "ValidationRules")
            .routes(|_| http::router());
        app.on(aphel_kernel::records::RecordValidating, 30, |ctx, args| {
            Box::pin(async move {
                infrastructure::validation_rule_guard(ctx.pool, &args.registry)
                    .check(ctx.tx, ctx.scope, &args.write())
                    .await
            })
        });
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(190, |pool, _| {
                std::sync::Arc::new(infrastructure::validation_rules_section(pool))
            }),
        ));
        Ok(())
    }
}
