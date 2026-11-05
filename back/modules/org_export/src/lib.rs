pub(crate) mod archive;
pub(crate) mod catalog;
pub(crate) mod cli;
pub(crate) mod dto;
pub(crate) mod http;
pub(crate) mod installer_http;
pub(crate) mod model;
pub(crate) mod notifications;
mod points;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;
use aphel_sdk::platform::application::files::{FileReferenceColumn, FileReferenceSource};

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest as ModuleManifest, Module, WorkerParts};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) use archive::*;
pub(crate) use catalog::*;
pub(crate) use model::*;
pub(crate) use service::{IdStrategy, RestoreCommand, RestoreEnv, build_archive};

pub(crate) const KEY: &str = "org_export";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OrganizationExportFileReferences;

impl FileReferenceSource for OrganizationExportFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![FileReferenceColumn::new("organization_exports", "file_id")]
    }
}

pub struct OrgExportModule;

impl Module for OrgExportModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(KEY, "Organization backup", APHEL_VERSION)
            .description("Full export of the organization (zip with records, relationships, files, configuration and manifest) and its restoration")

            .owned_tables(&["organization_exports"])
            .migration(Migration::new(
                1,
                "create organization exports",
                include_str!("../migrations/0001_create_organization_exports.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::modules::FileReferenceSource>(std::sync::Arc::new(
            points::OrganizationExportFiles,
        ));
        for kind in [
            aphel_sdk::exports::EXPORT_READY_NOTIFICATION,
            aphel_sdk::exports::EXPORT_FAILED_NOTIFICATION,
        ] {
            app.contribute(std::sync::Arc::new(
                aphel_sdk::notifications::NotificationKindDefinition::new(kind),
            ));
        }
        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            notifications::ExportNotificationTexts,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationSource>(std::sync::Arc::new(
            notifications::OrganizationExportNotifications,
        ));
        app.contribute::<dyn aphel_kernel::modules::ModuleCommand>(std::sync::Arc::new(
            cli::OrgCli,
        ));
        app.contribute::<dyn FileReferenceSource>(Arc::new(OrganizationExportFileReferences));
        app.openapi_tag(http::TAG, "Full export of the organization (zip with records, relationships, files, configuration and manifest) for backup and restoration with `aphel org import`")
            .routes(|_| http::router().merge(installer_http::router()));
        app.permission(PermissionDefinition::administrative(
            Permission::ORGANIZATION_EXPORT,
        ))
        .grant_to(["admin"]);
        app.worker(|env| {
            WorkerParts::default().handler(Arc::new(runtime::OrgExportRunHandler::new(
                env.pool.clone(),
                env.record_types.clone(),
                env.storage.clone(),
            )))
        });
        Ok(())
    }
}
