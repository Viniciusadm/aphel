pub(crate) mod coerce;
pub(crate) mod dashboard;
mod drafts;
pub mod dto;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod mapping;
pub(crate) mod model;
pub(crate) mod notifications;
pub(crate) mod runtime;
pub mod service;
pub(crate) mod source;
mod source_service;
pub mod store;
use store::privacy;
pub(crate) mod writer;

use std::sync::Arc;

use aphel_kernel::modules::Migration;
use aphel_sdk::platform::application::files::{FileReferenceColumn, FileReferenceSource};

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, DEFAULT_PRIORITY, Manifest, Module, WorkerParts};
use aphel_kernel::records::RecordSaved;
use aphel_sdk::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) use coerce::*;
pub use mapping::*;
pub use model::*;
pub use service::{
    ConfigureImportCommand, CreateImportCommand, IMPORT_IN_PROGRESS, IMPORT_NOT_EDITABLE,
    IMPORT_NOT_FOUND, IMPORT_NOT_RETRYABLE, IMPORT_NOT_SUPPORTED, IMPORT_WRITE_CONCURRENCY,
    ImportDetails, MAX_REPORTED_ROWS, RELATION_TARGET_PREFIX, RUN_TIME_BUDGET, RunImportRequest,
    RunProgress, create_permission, import_not_found, import_targets, is_importable, is_row_error,
    update_permission,
};
pub use source::*;
pub(crate) use writer::*;

pub const KEY: &str = "imports";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ImportFileReferences;

impl FileReferenceSource for ImportFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![
            FileReferenceColumn::new("import_jobs", "file_id"),
            FileReferenceColumn::new("import_jobs", "error_file_id"),
        ]
    }
}

pub struct ImportsModule;

impl Module for ImportsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Imports", APHEL_VERSION)
            .description("CSV and XLSX imports with mapping, dry run, execution and retry of failed rows over the record type handlers")
            .enabled_by_default()

            .owned_tables(&["import_jobs", "import_rows"])
            .migration(Migration::new(
                1,
                "create imports",
                include_str!("../migrations/0001_create_imports.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::imports::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::imports::ImportDrafts>(Arc::new(drafts::Drafts));
        app.provide::<dyn aphel_sdk::imports::ImportSources>(Arc::new(source_service::Sources));
        app.contribute::<dyn aphel_sdk::privacy::PrivacySource>(std::sync::Arc::new(
            privacy::ImportPrivacy,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            crate::notifications::ImportNotificationTexts,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::RecentEventExclusion>(Arc::new(
            dashboard::ImportedEventsExclusion,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationSource>(std::sync::Arc::new(
            notifications::ImportNotifications,
        ));
        app.contribute::<dyn FileReferenceSource>(Arc::new(ImportFileReferences));
        app.openapi_tag(
            http::TAG,
            "CSV and XLSX imports: upload, mapping, dry run, execution and retry of failed rows",
        )
        .routes(|_| http::router())
        .on(RecordSaved, DEFAULT_PRIORITY, |ctx, args| {
            Box::pin(hooks::mark_written_row(ctx, args))
        });
        for kind in [
            crate::notifications::IMPORT_FINISHED_NOTIFICATION,
            crate::notifications::IMPORT_FAILED_NOTIFICATION,
        ] {
            app.contribute(std::sync::Arc::new(NotificationKindDefinition {
                kind,
                channels: ALL_CHANNELS,
                email_default: false,
                push_default: false,
            }));
        }
        app.worker(|env| {
            WorkerParts::default().handler(Arc::new(runtime::ImportRunHandler::new(env.runtime())))
        });
        Ok(())
    }
}
