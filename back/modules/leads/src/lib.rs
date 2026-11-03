pub mod domain;
pub mod hooks;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod section;
pub mod service;
pub mod store;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::SystemField;
use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::defaults::seed::DefaultsSeed;
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
        people::CONTACT_RESOURCE,
        "A lead was qualified in the lead inbox (the contact moved from `lead` to `qualified`)",
    ),
    event(
        "lead.disqualified",
        people::CONTACT_RESOURCE,
        "A lead was disqualified in the lead inbox with a reason",
    ),
    event(
        "lead.converted",
        people::CONTACT_RESOURCE,
        "A lead was converted: a deal was created with the contact and the contact moved to the converted stage",
    ),
];

pub(crate) fn lifecycle_stage_field() -> Result<SystemField, AppError> {
    Ok(field(
        "lifecycle_stage",
        crate::labels("Lifecycle stage"),
        FieldType::Select,
    )?
    .with_option("lead", crate::labels("Lead"), Some("#64748b"))
    .with_option("qualified", crate::labels("Qualified"), Some("#0ea5e9"))
    .with_option("opportunity", crate::labels("Opportunity"), Some("#8b5cf6"))
    .with_option("customer", crate::labels("Customer"), Some("#22c55e"))
    .with_option(
        "former_customer",
        crate::labels("Former customer"),
        Some("#f97316"),
    ))
}

pub struct LeadsModule;

impl Module for LeadsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Leads", APHEL_VERSION)
            .description(
                "Lifecycle stages of people and organizations, automatic transitions, the lead inbox (qualify, disqualify, convert) and disqualification reasons",
            )
            .depends_on(people::KEY, "*")
            .enabled_by_default()
            .depends_on(organizations::KEY, "*")
            .owned_tables(&["lead_disqualification_reasons", "lead_disqualifications", "lifecycle_settings", "lifecycle_stage_history", "leads_contact_extension", "leads_company_extension"])
            .migration(Migration::new(
                1,
                "create leads",
                include_str!("../migrations/0001_create_leads.sql"),
            ))
            .migration(Migration::new(2, "move lifecycle extensions", include_str!("../migrations/0002_move_lifecycle_extensions.sql")))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.declare_hook(
            crate::hooks::LeadQualifying,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a lead is qualified in the lead inbox; an error blocks the qualification",
        );
        app.declare_hook(
            crate::hooks::LeadDisqualifying,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a lead is disqualified with a reason; an error blocks it",
        );
        app.declare_hook(crate::hooks::LeadConverting, &[aphel_kernel::modules::HookUsage::Filter],
            "Values of the deal (and optional company) created by the lead conversion; return them changed to enrich the conversion",
        );
        app.declare_hook(
            crate::hooks::LeadConverted,
            &[aphel_kernel::modules::HookUsage::Action],
            "After a lead was converted, inside the conversion transaction",
        );
        app.on(
            crate::hooks::LeadQualifying,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, lead| Box::pin(plugin_bridge::qualifying(ctx, lead)),
        )
        .on(
            crate::hooks::LeadDisqualifying,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, args| Box::pin(plugin_bridge::disqualifying(ctx, args)),
        )
        .filter(
            crate::hooks::LeadConverting,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, conversion| Box::pin(plugin_bridge::converting(ctx, conversion)),
        )
        .on(
            crate::hooks::LeadConverted,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, converted| Box::pin(plugin_bridge::converted(ctx, converted)),
        );
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(180, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::lifecycle_section(pool))
            }),
        ));
        app.contribute(Arc::new(DefaultsSeed(|connection| {
            Box::pin(store::seed_disqualification_reasons(connection))
        })));
        let stage = lifecycle_stage_field()?;
        app.extend("people.person", |contact| {
            contact.side_table("leads_contact_extension", "record_id").system_field(stage.clone());
        })
        .extend("organizations.organization", |company| {
            company.side_table("leads_company_extension", "record_id").system_field(stage.clone());
        })
        .on(RecordValidating, LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::validate_stage(ctx, args))
        })
        .on(RecordDeleted, LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::close_stage(ctx, args))
        })
        .on(RecordRestored, LIFECYCLE_STAGE_PRIORITY, |ctx, args| {
            Box::pin(hooks::restart_stage(ctx, args))
        })
        .consumer(|env| {
            service::lifecycle_transitions_consumer(env.pool.clone(), env.record_types.clone())
        })
        .provide::<dyn LeadConverter>(Arc::new(DealLeadConverter))
        .openapi_tag(
            http::TAG,
            "Lifecycle stages of people and organizations: automatic transitions, stage history, conversion report, lead inbox actions (qualify, disqualify, convert) and disqualification reasons",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::LIFECYCLE_MANAGE,
        ))
        .grant_to(["admin"]);
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for hook in plugin_bridge::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
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
