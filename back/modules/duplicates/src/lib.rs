pub(crate) mod detection;
pub(crate) mod domain;
pub(crate) mod dto;
pub(crate) mod fields;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod kinds;
pub(crate) mod model;
pub(crate) mod plugin_bridge;
pub(crate) mod scan;
pub(crate) mod service;
pub(crate) mod store;
mod timeline;

use aphel_kernel::modules::Migration;

use std::sync::Arc;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};
use aphel_kernel::records::DuplicateDetector;

pub(crate) use detection::*;
pub(crate) use fields::*;
pub(crate) use kinds::*;
pub(crate) use model::*;
pub(crate) use scan::*;

pub(crate) const KEY: &str = "duplicates";

pub struct DuplicatesModule;

impl Module for DuplicatesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Duplicates", APHEL_VERSION)
            .description("Duplicate detection rules, the duplicate queue and record merging")
            .enabled_by_default()
            .optional(leads::KEY)
            .optional(trash::KEY)
            .owned_tables(&["duplicate_candidates", "duplicate_rules", "record_merges"])
            .migration(Migration::new(
                1,
                "create duplicates",
                include_str!("../migrations/0001_create_duplicates.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::timeline::TimelineEventScope>(std::sync::Arc::new(
            timeline::MergedEvents,
        ));
        app.openapi_tag(
            http::TAG,
            "Duplicate detection rules, the duplicate queue and record merging",
        )
        .routes(|_| http::router())
        .provide::<dyn DuplicateDetector>(Arc::new(hooks::RuleDuplicateDetector))
        .contribute::<dyn trash::TrashExclusion>(Arc::new(hooks::MergedRecords));
        app.on(
            aphel_kernel::records::RecordPurged,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(hooks::forget_purged_record(ctx, removed)),
        );
        app.worker(|env| {
            WorkerParts::default()
                .handler(Arc::new(DuplicateScanHandler::new(
                    env.pool.clone(),
                    env.record_types.clone(),
                )))
                .schedule(duplicates_scan_schedule())
                .handler(Arc::new(DuplicateCleanupHandler::new(
                    env.pool.clone(),
                    env.record_types.clone(),
                )))
                .schedule(duplicates_cleanup_schedule())
        });
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
