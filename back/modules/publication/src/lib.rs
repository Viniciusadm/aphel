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

pub async fn probe(failures: &mut Vec<String>) {
    use aphel_sdk::platform::application::fields::permissions::check_managed_writes;
    use aphel_sdk::platform::application::shared::{Actor, ExecutionContext, FieldChange};
    let app = aphel_sdk::platform::modules::app();
    let registry = app.record_types();
    for (_, definition) in
        app.all_contributions::<aphel_contract_publication::PublishableDefinition>()
    {
        if let Err(error) = definitions::validate(registry, &definition) {
            failures.push(format!(
                "publication: invalid native definition {}: {error}",
                definition.entity
            ));
            continue;
        }
        let entity = registry
            .require(&definition.entity)
            .expect("validated publication entity");
        if entity.system_field_by_key("updated_at").is_none() {
            failures.push(format!(
                "publication: {} needs a readable saved draft version",
                definition.entity
            ));
        }
        for key in definition.projected_fields() {
            let changes = [FieldChange::new(
                key,
                serde_json::Value::Null,
                serde_json::json!("probe"),
            )];
            if check_managed_writes(entity, &[key], &changes).is_ok() {
                failures.push(format!(
                    "publication: generic writes must reject {}.{key}",
                    definition.entity
                ));
            }
            let execution = ExecutionContext::module(MODULE, "publication.activate", None)
                .expect("publication probe context");
            if execution
                .scope(async { check_managed_writes(entity, &[key], &changes) })
                .await
                .is_err()
            {
                failures.push(format!(
                    "publication: projection writer cannot update {}.{key}",
                    definition.entity
                ));
            }
            let other = ExecutionContext::module("probe_other", "records.import", None)
                .expect("unrelated module probe context");
            if other
                .scope(async { check_managed_writes(entity, &[key], &changes) })
                .await
                .is_ok()
            {
                failures.push(format!(
                    "publication: an unrelated writer can update {}.{key}",
                    definition.entity
                ));
            }
        }
    }
    for scope in [runtime::module_scope(), activation::module_scope()] {
        if !matches!(Actor::from_scope(&scope), Actor::Service { kind, id } if kind == "module" && id == MODULE)
        {
            failures
                .push("publication: background work must use its technical module actor".into());
        }
    }
}
