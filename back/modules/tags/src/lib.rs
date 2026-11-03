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
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::query::ReferenceSetField {
                key: "tags",
                label: "Tags",
            },
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "tags",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "tag",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        contract::register(app);
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(100, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::TagsSection::new(
                    crate::TagStore::new(pool.clone()),
                    ::config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone()),
                ))
            }),
        ));
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
        app.contribute::<dyn ::search::application::SearchContentSource>(Arc::new(
            search::TagSearchContent,
        ));
        app.contribute(Arc::new(
            aphel_kernel::infrastructure::query::compiler::ReferenceSetStorage {
                links_table: "taggings",
                value_column: "tag_id",
                labels_table: "tags",
            },
        ));
        Ok(())
    }
}
