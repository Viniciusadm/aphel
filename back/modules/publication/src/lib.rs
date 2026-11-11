pub(crate) mod activation;
pub(crate) mod definitions;
pub(crate) mod domain;
pub(crate) mod http;
pub(crate) mod operations;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_contract_publication::{ACTIVATE_MOMENT, MODULE};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, WorkerParts};
use aphel_sdk::approvals::{
    ApprovalMoment, ApprovalMomentDefinition, ApprovalMomentSource, MomentTarget,
};
use aphel_sdk::platform::application::files::{FileReferenceColumn, FileReferenceSource};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub struct PublicationModule;

struct PublicationMoments;

impl ApprovalMomentSource for PublicationMoments {
    fn moments(&self) -> Vec<ApprovalMomentDefinition> {
        vec![ApprovalMomentDefinition {
            moment: ApprovalMoment::from_static(ACTIVATE_MOMENT),
            label: "Publication".to_owned(),
            target: MomentTarget::Subject,
            record_types: Some(definitions::entities()),
        }]
    }
}

struct RevisionFiles;

impl FileReferenceSource for RevisionFiles {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![FileReferenceColumn::new(
            "publication_revision_files",
            "file_id",
        )]
    }
}

impl Module for PublicationModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(MODULE, "Publication", APHEL_VERSION)
            .description(
                "Immutable revisions of publishable records, immediate and scheduled publication with approval policies, withdrawal and retention",
            )
            .optional("approvals")
            .plugin_toml(include_str!("../plugin.toml"))
            .owned_tables(&[
                "publication_requests",
                "publication_revision_files",
                "publication_revisions",
                "publication_subjects",
            ])
            .migration(Migration::new(
                1,
                "create publication",
                include_str!("../migrations/0001_create_publication.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_contract_publication::DESCRIPTOR);
        app.contribute::<dyn ApprovalMomentSource>(Arc::new(PublicationMoments));
        app.contribute::<dyn FileReferenceSource>(Arc::new(RevisionFiles));
        app.openapi_tag(
            http::TAG,
            "Immutable revisions of publishable records, publication requests (immediate, scheduled or waiting for approval) and withdrawal",
        )
        .routes(|_| http::router())
        .worker(|_| {
            WorkerParts::default()
                .handler(runtime::ActivateHandler::shared())
                .handler(runtime::CollectHandler::shared())
                .consumer(runtime::ApprovalConsumer::shared())
                .schedule(runtime::collect_schedule())
        });
        operations::register(app);
        Ok(())
    }
}
