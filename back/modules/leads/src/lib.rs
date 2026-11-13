mod capture_defaults;
pub mod domain;
pub(crate) mod exposed_operations;
pub mod hooks;
pub(crate) mod http;
mod merge_history;
pub(crate) mod section;
pub mod service;
pub mod store;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::{RecordDeleted, RecordRestored, RecordValidating};
use aphel_sdk::platform::application::fields::SystemField;
use aphel_sdk::platform::application::fields::builtin::field;
use aphel_sdk::platform::application::public_events::{PublicEventDefinition, event};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use hooks::{
    ConvertedLead, DealLeadConverter, DisqualifyingLead, Lead, LeadConversion, LeadConverted,
    LeadConverter, LeadConverting, LeadDisqualifying, LeadQualifying,
};

pub use aphel_sdk::leads::KEY;
pub(crate) const LIFECYCLE_STAGE_PRIORITY: i32 = 40;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "lead.qualified",
        aphel_sdk::platform::application::public_events::PublicResource::new(
            aphel_contract_crm::keys::CONTACT,
            "ContactResponse",
        )
        .with_nested(&[("company", aphel_contract_crm::keys::COMPANY)]),
        "A lead was qualified in the lead inbox (the contact moved from `lead` to `qualified`)",
    ),
    event(
        "lead.disqualified",
        aphel_sdk::platform::application::public_events::PublicResource::new(
            aphel_contract_crm::keys::CONTACT,
            "ContactResponse",
        )
        .with_nested(&[("company", aphel_contract_crm::keys::COMPANY)]),
        "A lead was disqualified in the lead inbox with a reason",
    ),
    event(
        "lead.converted",
        aphel_sdk::platform::application::public_events::PublicResource::new(
            aphel_contract_crm::keys::CONTACT,
            "ContactResponse",
        )
        .with_nested(&[("company", aphel_contract_crm::keys::COMPANY)]),
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

fn extend_lifecycle(
    definition: &mut aphel_sdk::platform::application::fields::EntityDefinition,
) -> Result<(), AppError> {
    let table = match definition.record_type.as_str() {
        aphel_contract_crm::keys::CONTACT => "leads_contact_extension",
        aphel_contract_crm::keys::COMPANY => "leads_company_extension",
        _ => return Ok(()),
    };
    if definition.system_field_by_key("lifecycle_stage").is_some() {
        return Err(AppError::Configuration(
            "the lifecycle stage field is already declared".into(),
        ));
    }
    let mut field = lifecycle_stage_field()?;
    field.extension = Some(aphel_sdk::platform::application::fields::ExtensionColumn {
        table: table.into(),
        record_column: "record_id".into(),
    });
    definition.system_fields.push(field);
    Ok(())
}

pub struct LeadsModule;

impl Module for LeadsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Leads", APHEL_VERSION)
            .description(
                "Lifecycle stages of people and organizations, automatic transitions, the lead inbox (qualify, disqualify, convert) and disqualification reasons",
            )
            .depends_on("people", "*")
            .depends_on("organizations", "*")
            .enabled_by_default()
            .owned_tables(&["lead_disqualification_reasons", "lead_disqualifications", "lifecycle_settings", "lifecycle_stage_history", "leads_contact_extension", "leads_company_extension"])
            .migration(Migration::new(
                1,
                "create leads",
                include_str!("../migrations/0001_create_leads.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_sdk::leads::LeadMergeHistory>(std::sync::Arc::new(
            merge_history::History,
        ));
        app.contract(aphel_sdk::leads::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::party::PartyCaptureDefaults>(Arc::new(
            capture_defaults::CaptureDefaults,
        ));
        app.on(
            aphel_kernel::records::RecordPurged,
            LIFECYCLE_STAGE_PRIORITY,
            |ctx, args| {
                Box::pin(async move {
                    if args.record_type.as_str() == aphel_contract_crm::keys::CONTACT {
                        sqlx::query("DELETE FROM lead_disqualifications WHERE contact_id = ?")
                            .bind(args.record.record_id.to_string())
                            .execute(&mut **ctx.tx)
                            .await?;
                    }
                    sqlx::query(
                        "DELETE FROM lifecycle_stage_history WHERE record_type = ? AND record_id = ?",
                    )
                    .bind(args.record_type.as_str())
                    .bind(args.record.record_id.to_string())
                    .execute(&mut **ctx.tx)
                    .await?;
                    Ok(())
                })
            },
        );
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
        let contact_hook = |key, kind, description| {
            aphel_kernel::modules::PluginHook::new(
                key,
                aphel_contract_crm::keys::CONTACT,
                kind,
                description,
            )
        };
        let veto = aphel_kernel::modules::PluginHookKind::Veto;
        app.forward_hook_with(
            crate::hooks::LeadQualifying,
            contact_hook(crate::hooks::QUALIFYING, veto, "Before a lead is qualified in the lead inbox; values: { lead }. An error blocks the qualification"),
            crate::hooks::qualifying_forwarding(),
        )
        .forward_hook(
            crate::hooks::LeadDisqualifying,
            contact_hook(crate::hooks::DISQUALIFYING, veto, "Before a lead is disqualified; values: { lead, reason: { id, key, name }, note }. An error blocks it"),
        )
        .forward_filter_hook_with(
            crate::hooks::LeadConverting,
            contact_hook(crate::hooks::CONVERTING, aphel_kernel::modules::PluginHookKind::Filter, "Values of the deal (and optional company) a lead conversion creates; values: { contact_id, lead, deal, company }. Return { deal?: {..}, company?: {..} } to merge field values into them (a company object makes the conversion create a company when the contact has none)"),
            crate::hooks::converting_forwarding(),
        )
        .forward_hook(
            crate::hooks::LeadConverted,
            contact_hook(crate::hooks::CONVERTED, aphel_kernel::modules::PluginHookKind::Action, "After a lead was converted, inside the transaction; values: { lead, deal_id, company_id }"),
        );
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(180, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::lifecycle_section(pool))
            }),
        ));
        app.transform_entities("leads.lifecycle",extend_lifecycle)
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
        exposed_operations::register(app);
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_sdk::platform::application::fields::LocalizedLabels {
    aphel_sdk::platform::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
