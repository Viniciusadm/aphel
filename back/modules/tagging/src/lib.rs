mod classifiable;
mod config_references;
pub(crate) mod contract;
pub(crate) mod domain;
pub mod dto;
pub(crate) mod hooks;
pub mod http;
mod includes;
mod merge_references;
mod provision;
mod resolver;
pub(crate) mod search;
pub mod section;
pub mod service;
pub(crate) mod store;
mod vocabularies;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

pub use service::*;
pub use store::{TagStore, Tagging};

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "tagging";

pub struct TaggingModule;

impl Module for TaggingModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Tags", APHEL_VERSION)
            .description("Tags of the organization and tagging of opted-in records")
            .plugin_toml(include_str!("../plugin.toml"))
            .owned_tables(&["tags", "taggings", "tag_vocabularies"])
            .migration(Migration::new(
                1,
                "create tags and taggings",
                include_str!("../migrations/0001_create_tags.sql"),
            ))
            .migration(Migration::new(
                2,
                "create tag vocabularies",
                include_str!("../migrations/0002_create_tag_vocabularies.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::merges::MergeReferences>(std::sync::Arc::new(
            merge_references::References,
        ));
        app.contribute::<dyn aphel_sdk::configuration::ConfigurationReferences>(
            std::sync::Arc::new(config_references::References),
        );
        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::application::query::ReferenceSetField {
                key: "tags",
                label: "Tags",
            },
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::versioning::fragments::SectionFragmentLayout {
                section: "tags",
                collections: vec![
                    aphel_sdk::configuration::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "tag",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        contract::register(app);
        app.contribute::<dyn aphel_sdk::records::RecordIncludeSource>(Arc::new(includes::Tags));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(100, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::TagsSection::new(
                    crate::TagStore::new(pool.clone()),
                    aphel_sdk::configuration::ConfigKeyClient::new(pool.clone()),
                ))
            }),
        ));
        app.openapi_tag(
            http::TAG,
            "Tags of the organization and tagging of opted-in records",
        )
        .routes(|_| http::router().merge(vocabularies::router()));
        app.filter(
            aphel_kernel::records::RecordSaving,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, saving| Box::pin(classifiable::saving(ctx, saving)),
        )
        .on(
            aphel_kernel::records::RecordValidating,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, changed| Box::pin(classifiable::validating(ctx, changed)),
        );
        app.contribute::<dyn aphel_contract_tagging::TermResolver>(Arc::new(resolver::PublicTerms));
        app.contribute::<dyn aphel_sdk::provisioning::Provisioner>(Arc::new(
            provision::VocabularyProvisioner,
        ));
        app.on(
            aphel_kernel::records::RecordPurged,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(crate::hooks::purge_record_taggings(ctx, removed)),
        );
        app.contribute::<dyn aphel_sdk::search::SearchContentSource>(Arc::new(
            search::TagSearchContent,
        ));
        app.contribute(Arc::new(
            aphel_sdk::platform::infrastructure::query::compiler::ReferenceSetStorage {
                links_table: "taggings",
                value_column: "tag_id",
                labels_table: "tags",
            },
        ));
        Ok(())
    }
}
