pub(crate) mod contract;
pub(crate) mod domain;
pub mod dto;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod search;
pub mod section;
pub mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

pub use service::*;
pub use store::{TagStore, Tagging};

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "tags";

pub struct TagsModule;

impl Module for TagsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Tags", APHEL_VERSION)
            .description("Tags of the organization and tagging of contacts, companies, deals and custom objects")
            .required()
            .plugin_toml(include_str!("../plugin.toml"))
            .owned_tables(&["tags", "taggings"])
            .migration(Migration::new(
                1,
                "create tags and taggings",
                include_str!("../migrations/0001_create_tags.sql"),
            ))
            .migration(Migration::new(2, "namespace_config_references", include_str!("../migrations/0002_namespace_config_references.sql")))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        contract::register(app);
        app.contribute(std::sync::Arc::new(aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
            100,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::TagsSection::new(crate::TagStore::new(pool.clone()), aphel_kernel::infrastructure::config_packages::MySqlConfigKeyRepository::new(pool.clone())))
            },
        )));
        app.openapi_tag(
            http::TAG,
            "Tags of the organization and tagging of contacts, companies and deals",
        )
        .routes(|_| http::router());
        app.on(
            aphel_kernel::records::RecordPurged,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(crate::hooks::purge_record_taggings(ctx, removed)),
        );
        app.contribute::<dyn aphel_kernel::application::search::SearchContentSource>(Arc::new(
            search::TagSearchContent,
        ));
        app.contribute(Arc::new(
            aphel_kernel::infrastructure::query::compiler::TagStorage {
                taggings: "taggings",
                tag_column: "tag_id",
                tags: "tags",
            },
        ));
        Ok(())
    }
}
