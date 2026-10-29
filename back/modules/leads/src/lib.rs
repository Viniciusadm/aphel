pub mod domain;
pub mod hooks;
pub(crate) mod http;
pub(crate) mod section;
pub mod service;
pub mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::SystemField;
use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::crm::seed::CrmDefaultsSeed;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::{RecordDeleted, RecordRestored, RecordValidating};

pub use hooks::{
    ConvertedLead, DealLeadConverter, DisqualifyingLead, Lead, LeadConversion, LeadConverted,
    LeadConverter, LeadConverting, LeadDisqualifying, LeadQualifying,
};

pub const KEY: &str = "leads";
pub(crate) const LIFECYCLE_STAGE_PRIORITY: i32 = 40;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "lead.qualified",
        contacts::CONTACT_RESOURCE,
        "A lead was qualified in the lead inbox (the contact moved from `lead` to `qualified`)",
    ),
    event(
        "lead.disqualified",
        contacts::CONTACT_RESOURCE,
        "A lead was disqualified in the lead inbox with a reason",
    ),
    event(
        "lead.converted",
        contacts::CONTACT_RESOURCE,
        "A lead was converted: a deal was created with the contact and the contact moved to the converted stage",
    ),
];

pub(crate) fn lifecycle_stage_field() -> Result<SystemField, AppError> {
    Ok(field(
        "lifecycle_stage",
        "Estágio do ciclo de vida",
        FieldType::Select,
    )?
    .with_option("lead", "Lead", Some("#64748b"))
    .with_option("qualified", "Qualificado", Some("#0ea5e9"))
    .with_option("opportunity", "Oportunidade", Some("#8b5cf6"))
    .with_option("customer", "Cliente", Some("#22c55e"))
    .with_option("former_customer", "Ex-cliente", Some("#f97316")))
}

pub struct LeadsModule;

impl Module for LeadsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Leads", APHEL_VERSION)
            .description(
                "Lifecycle stages of contacts and companies, automatic transitions, the lead inbox (qualify, disqualify, convert) and disqualification reasons",
            )
            .depends_on(contacts::KEY, "*")
            .enabled_by_default()
            .depends_on(companies::KEY, "*")
            .owned_tables(&["lead_disqualification_reasons", "lead_disqualifications", "lifecycle_settings", "lifecycle_stage_history"])
            .migration(Migration::new(
                1,
                "create leads",
                include_str!("../migrations/0001_create_leads.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.declare_hook::<crate::hooks::LeadQualifying>(
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a lead is qualified in the lead inbox; an error blocks the qualification",
        );
        app.declare_hook::<crate::hooks::LeadDisqualifying>(
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a lead is disqualified with a reason; an error blocks it",
        );
        app.declare_hook::<crate::hooks::LeadConverting>(
            &[aphel_kernel::modules::HookUsage::Filter],
            "Values of the deal (and optional company) created by the lead conversion; return them changed to enrich the conversion",
        );
        app.declare_hook::<crate::hooks::LeadConverted>(
            &[aphel_kernel::modules::HookUsage::Action],
            "After a lead was converted, inside the conversion transaction",
        );
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                180,
                |pool, registry| {
                    let _ = registry;
                    std::sync::Arc::new(crate::section::lifecycle_section(pool))
                },
            ),
        ));
        app.contribute(Arc::new(CrmDefaultsSeed(|connection| {
            Box::pin(store::seed_disqualification_reasons(connection))
        })));
        let stage = lifecycle_stage_field()?;
        app.extend_record_type("contact", |contact| {
            contact.system_field(stage.clone());
        })
        .extend_record_type("company", |company| {
            company.system_field(stage.clone());
        })
        .on::<RecordValidating, _>(LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::validate_stage(ctx, args))
        })
        .on::<RecordDeleted, _>(LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::close_stage(ctx, args))
        })
        .on::<RecordRestored, _>(LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::restart_stage(ctx, args))
        })
        .consumer(|env| {
            service::lifecycle_transitions_consumer(env.pool.clone(), env.record_types.clone())
        })
        .provide::<dyn LeadConverter>(Arc::new(DealLeadConverter))
        .openapi_tag(
            http::TAG,
            "Lifecycle stages of contacts and companies: automatic transitions, stage history, conversion report, lead inbox actions (qualify, disqualify, convert) and disqualification reasons",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::LIFECYCLE_MANAGE,
        ))
        .grant_to(["admin"]);
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        Ok(())
    }
}
