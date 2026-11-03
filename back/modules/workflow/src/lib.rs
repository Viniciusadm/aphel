pub mod domain;
pub(crate) mod guidance;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod lost_reason_section;
pub(crate) mod plugin_bridge;
pub(crate) mod requirements;
pub mod rotting;
pub mod section;
pub mod seed;
pub mod service;
pub mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::public_events::{PublicEventDefinition, PublicResource, event};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::defaults::{DefaultsProbe, DefaultsSeed};
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::RecordValidating;

const CHECKLIST_RESOURCE: PublicResource = PublicResource::payload("workflow.record");

pub const KEY: &str = "workflow";
pub(crate) const STAGE_REQUIREMENTS_PRIORITY: i32 = 5;
pub(crate) const STAGE_CHECKLIST_PRIORITY: i32 = 20;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        guidance::domain::RECORD_CHECKLIST_ITEM_CHECKED,
        CHECKLIST_RESOURCE,
        "An item of the stage checklist of the record was checked",
    ),
    event(
        guidance::domain::RECORD_CHECKLIST_ITEM_UNCHECKED,
        CHECKLIST_RESOURCE,
        "An item of the stage checklist of the record was unchecked",
    ),
];

pub struct WorkflowModule;

impl Module for WorkflowModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Workflow", APHEL_VERSION)
            .description(
                "Workflow stages (probability, rotting, required fields, guidance and checklist) and lost reasons",
            )
            .owned_tables(&["lost_reasons", "workflow", "workflow_stage_checklist_items", "workflow_stage_guidance", "workflow_stages", "workflow_teams", "workflow_checklist_progress"])
            .migration(Migration::new(
                1,
                "create workflow",
                include_str!("../migrations/0001_create_pipelines.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::workflow::DESCRIPTOR);
        app.permission(
            aphel_kernel::domain::access::PermissionDefinition::administrative(
                aphel_kernel::domain::access::Permission::from_static("workflow.read"),
            ),
        )
        .grant_to(["admin", "manager", "member"]);
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable(
                "workflow_checklist_progress",
            ),
        ));
        app.on(aphel_kernel::records::RecordPurged, 10, |ctx,args| Box::pin(async move {
            sqlx::query("DELETE FROM workflow_checklist_progress WHERE record_type = ? AND record_id = ?").bind(args.record_type.as_str()).bind(args.record.record_id.to_string()).execute(&mut **ctx.tx).await?;
            Ok(())
        }));

        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "lost_reasons",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "lost_reason",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        app.entity(aphel_kernel::application::fields::builtin::with_fields(
            aphel_kernel::application::fields::EntityDefinition::new(
                aphel_kernel::application::shared::RecordType::from_static("workflow.pipeline"),
                "workflow",
                aphel_kernel::domain::access::Permission::from_static("workflow.read"),
            )
            .capabilities(
                aphel_kernel::application::fields::EntityTraits::default()
                    .with_trait("records.global"),
            )
            .title_field("name")
            .system_field(aphel_kernel::application::fields::builtin::field(
                "name",
                "Name",
                aphel_kernel::domain::fields::FieldType::Text,
            )?)
            .system_field(aphel_kernel::application::fields::builtin::field(
                "is_active",
                "Active",
                aphel_kernel::domain::fields::FieldType::Checkbox,
            )?),
            aphel_kernel::application::fields::builtin::timestamps()?,
        ));

        app.entity(aphel_kernel::application::fields::builtin::with_fields(
            aphel_kernel::application::fields::EntityDefinition::new(
                aphel_kernel::application::shared::RecordType::from_static("workflow.stage"),
                "workflow_stages",
                aphel_kernel::domain::access::Permission::from_static("workflow.read"),
            )
            .capabilities(
                aphel_kernel::application::fields::EntityTraits::default()
                    .with_trait("records.global"),
            )
            .title_field("name")
            .system_field(aphel_kernel::application::fields::builtin::field(
                "name",
                "Name",
                aphel_kernel::domain::fields::FieldType::Text,
            )?)
            .system_field(aphel_kernel::application::fields::builtin::field(
                "is_active",
                "Active",
                aphel_kernel::domain::fields::FieldType::Checkbox,
            )?),
            aphel_kernel::application::fields::builtin::timestamps()?,
        ));

        app.entity(aphel_kernel::application::fields::builtin::with_fields(
            aphel_kernel::application::fields::EntityDefinition::new(
                aphel_kernel::application::shared::RecordType::from_static("workflow.lost_reason"),
                "lost_reasons",
                aphel_kernel::domain::access::Permission::from_static("workflow.read"),
            )
            .capabilities(
                aphel_kernel::application::fields::EntityTraits::default()
                    .with_trait("records.global"),
            )
            .title_field("name")
            .system_field(aphel_kernel::application::fields::builtin::field(
                "name",
                "Name",
                aphel_kernel::domain::fields::FieldType::Text,
            )?)
            .system_field(aphel_kernel::application::fields::builtin::field(
                "is_active",
                "Active",
                aphel_kernel::domain::fields::FieldType::Checkbox,
            )?),
            aphel_kernel::application::fields::builtin::timestamps()?,
        ));

        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::NestedFragmentLayout {
                section: "workflow",
                parent_kind: "pipeline",
                child_kind: "stage",
                children_field: "stages",
            },
        ));
        for (record_type, table) in [
            ("workflow.pipeline", "workflow"),
            ("workflow.stage", "workflow_stages"),
            ("workflow.lost_reason", "lost_reasons"),
        ] {
            app.contribute(std::sync::Arc::new(
                aphel_kernel::infrastructure::query::aggregate::LookupTitleSource {
                    record_type,
                    table,
                    title_column: "name",
                },
            ));
        }
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(30, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::PipelinesSection::new(
                    registry.clone(),
                    crate::store::PipelineStore::new(pool.clone()),
                    ::config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone()),
                    aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                        pool.clone(),
                    ),
                ))
            }),
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(40, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::lost_reason_section::LostReasonsSection::new(
                    crate::store::PipelineStore::new(pool.clone()),
                    ::config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone()),
                ))
            }),
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(200, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::guidance::section::stage_guidance_section(pool))
            }),
        ));
        app.contribute(std::sync::Arc::new(DefaultsSeed(|connection| {
            Box::pin(seed::seed_pipeline_defaults(connection))
        })));
        for table in seed::SEEDED_TABLES {
            app.contribute(std::sync::Arc::new(DefaultsProbe(table)));
        }
        app.on(
            RecordValidating,
            STAGE_REQUIREMENTS_PRIORITY,
            |ctx, args| Box::pin(hooks::check_stage_requirements(ctx, args)),
        )
        .on(RecordValidating, STAGE_CHECKLIST_PRIORITY, |ctx, args| {
            Box::pin(hooks::check_stage_checklist(ctx, args))
        })
        .openapi_tag(http::TAG, "Pipelines, stages and lost reasons")
        .routes(|_| http::router().merge(guidance::http::router()));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
