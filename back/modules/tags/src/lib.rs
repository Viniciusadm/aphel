pub(crate) mod automation;
pub(crate) mod domain;
pub mod dto;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod operations;
pub(crate) mod plugin_bridge;
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
            .owned_tables(&["tags", "taggings"])
            .migration(Migration::new(
                1,
                "create tags and taggings",
                include_str!("../migrations/0001_create_tags.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_kernel::application::automations::TagOperations>(
            std::sync::Arc::new(operations::AutomationTags),
        );
        app.contribute(std::sync::Arc::new(aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
            100,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::TagsSection::new(crate::TagStore::new(pool.clone()), aphel_kernel::infrastructure::config_packages::MySqlConfigKeyRepository::new(pool.clone())))
            },
        )));
        use crate::automation::Tag;
        app.openapi_tag(
            http::TAG,
            "Tags of the organization and tagging of contacts, companies and deals",
        )
        .routes(|_| http::router());
        app.action(Arc::new(Tag { present: true }))
            .action(Arc::new(Tag { present: false }));
        app.on::<aphel_kernel::records::RecordPurged, _>(
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(crate::hooks::purge_record_taggings(ctx, removed)),
        );
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
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
