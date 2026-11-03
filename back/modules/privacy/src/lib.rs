pub(crate) mod domain;
pub(crate) mod dto;
pub(crate) mod http;
pub mod mysql;
pub(crate) mod notifications;
mod points;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::application::files::FileReferenceSource;
use aphel_kernel::modules::Migration;

use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};

pub(crate) use domain::*;
pub use service::subject::{
    Redaction, SubjectMatcher, mask_digits, mask_email, mask_text, parse_subject,
};

pub(crate) const KEY: &str = "privacy";

pub struct PrivacyModule;

impl Module for PrivacyModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Privacy", APHEL_VERSION)
            .description("Privacy (LGPD): personal data subjects, data subject requests and retention policies")
            .enabled_by_default()
            .depends_on(exports::KEY, "*")
            .owned_tables(&["retention_policies", "retention_runs"])
            .migration(Migration::new(
                1,
                "create privacy",
                include_str!("../migrations/0001_create_privacy.sql"),
            ))
            .migration(Migration::new(
                2,
                "privacy progress",
                include_str!("../migrations/0002_privacy_progress.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::privacy::DESCRIPTOR);
        app.contribute::<dyn aphel_kernel::modules::FileReferenceSource>(std::sync::Arc::new(
            points::PrivacyFiles,
        ));
        app.contribute::<dyn ::notifications::application::NotificationSource>(
            std::sync::Arc::new(notifications::PrivacyExportNotifications),
        );
        app.openapi_tag(http::TAG, "Privacy (LGPD): personal data subjects, data subject requests (export, anonymization, deletion) and retention policies")
            .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::PRIVACY_MANAGE,
        ))
        .grant_to(["admin"]);
        app.contribute::<dyn FileReferenceSource>(Arc::new(mysql::sql::PrivacyFileReferences));
        app.worker(|env| {
            let runtime = runtime::PrivacyRuntime::new(
                env.pool.clone(),
                env.record_types.clone(),
                env.storage.clone(),
            );
            WorkerParts::default()
                .handlers(runtime.handlers())
                .schedule(runtime::privacy_retention_schedule())
                .schedule(runtime::privacy_retention_sweep_schedule())
        });
        Ok(())
    }
}
