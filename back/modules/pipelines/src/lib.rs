pub mod domain;
pub(crate) mod guidance;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod lost_reason_section;
pub(crate) mod requirements;
pub mod rotting;
pub mod section;
pub mod seed;
pub mod service;
pub mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::automations::time::triggers::TimeTriggerPlanner;
use aphel_kernel::application::public_events::{PublicEventDefinition, PublicResource, event};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::crm::{CrmDefaultsProbe, CrmDefaultsSeed};
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::RecordValidating;

const DEAL_RESOURCE: PublicResource = PublicResource::new("deal", "DealResponse");

pub const KEY: &str = "pipelines";
pub(crate) const DEAL_RECORD_TYPE: &str = "deal";
pub(crate) const STAGE_REQUIREMENTS_PRIORITY: i32 = 5;
pub(crate) const STAGE_CHECKLIST_PRIORITY: i32 = 20;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        guidance::domain::DEAL_CHECKLIST_ITEM_CHECKED,
        DEAL_RESOURCE,
        "An item of the stage checklist of the deal was checked",
    ),
    event(
        guidance::domain::DEAL_CHECKLIST_ITEM_UNCHECKED,
        DEAL_RESOURCE,
        "An item of the stage checklist of the deal was unchecked",
    ),
];

pub struct PipelinesModule;

impl Module for PipelinesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Pipelines", APHEL_VERSION)
            .description(
                "Sales pipelines, stages (probability, rotting, required fields, guidance and checklist) and lost reasons",
            )
            .required()
            .owned_tables(&["lost_reasons", "pipelines", "pipeline_stage_checklist_items", "pipeline_stage_guidance", "pipeline_stages", "pipeline_teams"])
            .migration(Migration::new(
                1,
                "create pipelines",
                include_str!("../migrations/0001_create_pipelines.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        for (record_type, table) in [
            ("pipeline", "pipelines"),
            ("pipeline_stage", "pipeline_stages"),
            ("lost_reason", "lost_reasons"),
        ] {
            app.contribute(std::sync::Arc::new(
                aphel_kernel::infrastructure::query::aggregate::LookupTitleSource {
                    record_type,
                    table,
                    title_column: "name",
                },
            ));
        }
        app.contribute(std::sync::Arc::new(aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
            30,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::PipelinesSection::new(registry.clone(), crate::store::PipelineStore::new(pool.clone()), aphel_kernel::infrastructure::config_packages::MySqlConfigKeyRepository::new(pool.clone()), aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository::new(pool.clone())))
            },
        )));
        app.contribute(std::sync::Arc::new(aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
            40,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::lost_reason_section::LostReasonsSection::new(crate::store::PipelineStore::new(pool.clone()), aphel_kernel::infrastructure::config_packages::MySqlConfigKeyRepository::new(pool.clone())))
            },
        )));
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                200,
                |pool, registry| {
                    let _ = registry;
                    std::sync::Arc::new(crate::guidance::section::stage_guidance_section(pool))
                },
            ),
        ));
        app.contribute(std::sync::Arc::new(CrmDefaultsSeed(|connection| {
            Box::pin(seed::seed_pipeline_defaults(connection))
        })));
        for table in seed::SEEDED_TABLES {
            app.contribute(std::sync::Arc::new(CrmDefaultsProbe(table)));
        }
        let mut outcome = Ok(());
        app.extend_record_type(DEAL_RECORD_TYPE, |deal| {
            outcome = rotting::rotting_fields().and_then(|fields| {
                fields
                    .into_iter()
                    .try_for_each(|field| deal.computed(field).map(|_| ()))
            });
        });
        outcome?;
        app.trigger(std::sync::Arc::new(rotting::DealRotting));
        app.contribute(std::sync::Arc::new(TimeTriggerPlanner {
            kind: rotting::TIME_DEAL_ROTTING,
            plan: rotting::rotting_plan,
        }));
        app.on::<RecordValidating, _>(STAGE_REQUIREMENTS_PRIORITY, |ctx, args| {
            Box::pin(hooks::check_stage_requirements(ctx, args))
        })
        .on::<RecordValidating, _>(STAGE_CHECKLIST_PRIORITY, |ctx, args| {
            Box::pin(hooks::check_stage_checklist(ctx, args))
        })
        .openapi_tag(http::TAG, "Pipelines, stages and lost reasons")
        .routes(|_| http::router().merge(guidance::http::router()));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        Ok(())
    }
}
