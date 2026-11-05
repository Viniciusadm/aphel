pub mod application;
mod cli;
pub mod dto;
mod http;
pub mod infrastructure;
mod maintenance;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "search";
pub struct SearchModule;
impl Module for SearchModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Search",
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create search index",
            include_str!("../migrations/1510_create_search_index.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            2,
            "create search index terms",
            include_str!("../migrations/1511_create_search_index_terms.sql"),
        ))
        .owned_tables(&["search_index", "search_index_terms"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_sdk::search::SearchMaintenance>(std::sync::Arc::new(
            maintenance::Maintenance,
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::infrastructure::custom_objects::RecordTypeScopedTable(
                "search_index",
            ),
        ));
        app.contribute::<dyn aphel_kernel::modules::ModuleCommand>(std::sync::Arc::new(
            cli::SearchCli,
        ));
        app.openapi_tag(http::TAG, "Search")
            .routes(|_| http::router());
        app.worker(|env| {
            aphel_kernel::modules::WorkerParts::default()
                .handler(infrastructure::search_reindex_handler(
                    env.pool.clone(),
                    env.record_types.clone(),
                ))
                .consumer(infrastructure::search_index_consumer(
                    env.pool.clone(),
                    env.record_types.clone(),
                ))
        });
        Ok(())
    }
}
