pub(crate) mod domain;
pub(crate) mod dto;
pub(crate) mod exposed_operations;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod service;
pub(crate) mod store;
use store::config_references;
mod target_reader;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::application::query::FilterSourceResolver;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use aphel_sdk::segments::KEY;

pub struct SegmentsModule;

impl Module for SegmentsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Segments", APHEL_VERSION)
            .description(
                "Named dynamic filters of records, usable as the in_saved_filter filter condition",
            )
            .enabled_by_default()
            .owned_tables(&["segments"])
            .migration(Migration::new(
                1,
                "create segments",
                include_str!("../migrations/0001_create_segments.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::segments::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::segments::SegmentTargetReader>(Arc::new(
            target_reader::TargetReader,
        ));
        app.contribute::<dyn aphel_sdk::configuration::ConfigurationReferences>(
            std::sync::Arc::new(config_references::References),
        );
        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::infrastructure::custom_objects::RecordTypeScopedTable("segments"),
        ));
        app.openapi_tag(
            http::TAG,
            "Segments: named dynamic filters with member count, usable as the `in_saved_filter` filter condition",
        )
        .routes(|_| http::router());
        app.provide::<dyn FilterSourceResolver>(Arc::new(hooks::StoredFilterSources));
        exposed_operations::register(app);
        Ok(())
    }
}
