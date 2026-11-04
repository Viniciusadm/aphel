mod config_references;
pub(crate) mod domain;
pub(crate) mod dto;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::query::FilterSourceResolver;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "segments";

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
        app.contribute::<dyn aphel_sdk::configuration::ConfigurationReferences>(
            std::sync::Arc::new(config_references::References),
        );
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable("segments"),
        ));
        app.openapi_tag(
            http::TAG,
            "Segments: named dynamic filters with member count, usable as the `in_saved_filter` filter condition",
        )
        .routes(|_| http::router());
        app.provide::<dyn FilterSourceResolver>(Arc::new(hooks::StoredFilterSources));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
