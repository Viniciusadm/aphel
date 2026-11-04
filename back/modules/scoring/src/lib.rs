use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub(crate) mod directory;
pub(crate) mod domain;
pub(crate) mod engine;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod runtime;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use crate::domain::SCORE_FIELD;
use crate::runtime::{
    ScoringDecayHandler, ScoringEventsConsumer, ScoringRecalculateHandler, ScoringRuntime,
    scoring_decay_schedule,
};
use aphel_kernel::application::fields::{COMPUTED_RECORD_ALIAS, ComputedColumn, SystemField};
use aphel_kernel::application::public_events::{PreviousValues, PublicEventDefinition, changed};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};

pub(crate) const KEY: &str = "scoring";

const SCORED_TYPES: [RecordType; 2] = [RecordType::CONTACT, RecordType::COMPANY];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    changed(
        domain::SCORE_BAND_CHANGED,
        aphel_sdk::people::RESOURCE,
        "The score of the contact crossed into another band (cold, warm, hot) of a scoring model",
        PreviousValues::Changes,
    ),
    changed(
        domain::COMPANY_SCORE_BAND_CHANGED,
        aphel_sdk::organizations::RESOURCE,
        "The score of the company crossed into another band (cold, warm, hot) of a scoring model",
        PreviousValues::Changes,
    ),
];

pub(crate) fn score_field(record_type: &RecordType) -> Result<SystemField, AppError> {
    Ok(SystemField::new(
        SCORE_FIELD,
        crate::labels("Score"),
        FieldType::Number,
        SCORE_FIELD,
    )?
    .computed(primary_score(record_type)))
}

fn primary_score(record_type: &RecordType) -> ComputedColumn {
    ComputedColumn::expression(format!(
        "SELECT COALESCE(sc_rs.score, 0) FROM scoring_models sc_m \
         LEFT JOIN record_scores sc_rs ON sc_rs.model_id = sc_m.id AND sc_rs.record_id = {COMPUTED_RECORD_ALIAS}.id \
         WHERE sc_m.primary_record_type = '{}' \
         AND sc_m.is_active = TRUE LIMIT 1",
        record_type.as_str().replace('\'', "")
    ))
}

pub struct ScoringModule;

impl Module for ScoringModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Scoring", APHEL_VERSION)
            .description(
                "Lead scoring models (profile and behavior criteria, decay, bands) and the scores of a record with their explanation",
            )
            .depends_on(aphel_sdk::people::KEY, "*")
            .enabled_by_default()
            .owned_tables(&["record_scores"])
            .depends_on(aphel_sdk::organizations::KEY, "*")
            .migration(Migration::new(
                1,
                "create scoring",
                include_str!("../migrations/0001_create_scoring.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(220, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::scoring_models_section(pool))
            }),
        ));
        app.triggers(automation::scoring_triggers());
        for record_type in SCORED_TYPES {
            let field = score_field(&record_type)?;
            let mut outcome = Ok(());
            app.extend(record_type.as_str(), |extension| {
                outcome = extension.computed(field).map(|_| ());
            });
            outcome?;
        }
        app.worker(|env| {
            let runtime = ScoringRuntime::new(env.pool.clone(), env.record_types.clone());
            WorkerParts::default()
                .consumer(Arc::new(ScoringEventsConsumer(runtime.clone())))
                .handler(Arc::new(ScoringRecalculateHandler(runtime.clone())))
                .handler(Arc::new(ScoringDecayHandler(runtime)))
                .schedule(scoring_decay_schedule())
        })
        .openapi_tag(
            http::TAG,
            "Lead scoring models (profile and behavior criteria, decay, bands) and the scores of a record with their explanation",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::SCORING_MANAGE,
        ))
        .grant_to(["admin"]);
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
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
