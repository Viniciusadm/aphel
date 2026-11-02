pub mod dto;
pub(crate) mod encode;
pub(crate) mod format;
pub(crate) mod generated;
pub(crate) mod http;
pub(crate) mod model;
pub mod notifications;
pub(crate) mod plugin_bridge;
mod points;
pub(crate) mod runtime;
pub mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::application::files::{FileReferenceColumn, FileReferenceSource};
use aphel_kernel::modules::Migration;

use aphel_kernel::domain::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};

pub use encode::*;
pub(crate) use format::*;
pub use generated::*;
pub use model::*;
pub use runtime::{ExportRunHandler, system_scope};
pub use service::{EXPORT_NOT_FOUND, EXPORT_NOT_SUPPORTED, EXPORT_TOO_LARGE, export_not_found};
pub use store::{LabelSource, config_label_source};

pub const KEY: &str = "exports";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ExportFileReferences;

impl FileReferenceSource for ExportFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![FileReferenceColumn::new("export_jobs", "file_id")]
    }
}

pub struct ExportsModule;

impl Module for ExportsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Exports", APHEL_VERSION)
            .description("CSV and XLSX exports of filtered records, the generated file helpers and the worker state shared by the data modules")
            .required()
            .owned_tables(&["export_jobs"])
            .migration(Migration::new(
                1,
                "create export jobs",
                include_str!("../migrations/0001_create_export_jobs.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::modules::FileReferenceSource>(std::sync::Arc::new(
            points::ExportFiles,
        ));
        app.contribute::<dyn aphel_kernel::application::notifications::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::ExportNotificationTexts),
        );
        app.contribute::<dyn aphel_kernel::application::notifications::NotificationSource>(
            std::sync::Arc::new(notifications::ExportNotifications),
        );
        app.openapi_tag(http::TAG, "CSV and XLSX exports of filtered records")
            .routes(|_| http::router());
        app.contribute::<dyn FileReferenceSource>(Arc::new(ExportFileReferences));
        for kind in [
            crate::notifications::EXPORT_READY_NOTIFICATION,
            crate::notifications::EXPORT_FAILED_NOTIFICATION,
        ] {
            app.notification_kind(NotificationKindDefinition {
                kind,
                channels: ALL_CHANNELS,
                email_default: false,
                push_default: false,
            });
        }
        app.worker(|env| {
            WorkerParts::default().handler(Arc::new(ExportRunHandler::new(env.state())))
        });
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
