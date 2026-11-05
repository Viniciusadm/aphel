mod associations;
mod context_source;
mod dataset_seed;
mod lifecycle;
mod merge_references;
mod probability;
mod renewal_writer;
use aphel_sdk::imports::ImportTraits;
mod dashboard;
mod followers;
mod reports;
use aphel_sdk::privacy::PrivacyTraits;
mod line_items;
mod timeline;
use aphel_sdk::line_items::LineItemTraits;
mod financials;
use aphel_sdk::assignment::AssignmentTraits;
use aphel_sdk::money::MoneyTraits;
use aphel_sdk::workflow::WorkflowRecordTypes;
mod workflow_source;
use aphel_sdk::workflow::WorkflowTraits;
mod rotting;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod exposed_operations;
pub mod hooks;
pub mod http;
pub(crate) mod notifications;
pub(crate) mod operations;
pub(crate) mod runtime;
pub mod service;
pub mod store;
pub mod team;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes, activities_relation};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::DEFAULT_PRIORITY;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::{RecordQueryFilter, SearchPlanFilter};
use aphel_sdk::notifications::NotificationKindDefinition;
use aphel_sdk::platform::application::fields::builtin::{
    field, record_field, timestamps, with_fields,
};
use aphel_sdk::platform::application::fields::{EntityDefinition, EntityTraits, RecordRelation};
use aphel_sdk::platform::application::public_events::{
    PreviousValues, PublicEventDefinition, changed, deleted, event,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use hooks::{
    ClosingDeal, DealLosing, DealLost, DealStageChanging, DealWinning, DealWon, StageChange,
};

pub use aphel_sdk::deals::RESOURCE as DEAL_RESOURCE;

pub use aphel_sdk::deals::KEY;

const DEAL_BULK_FIELDS: &[&str] = &[
    "title",
    "amount",
    "currency",
    "probability",
    "expected_close_date",
    "contact_id",
    "company_id",
    "owner_id",
    "source",
];
pub(crate) const DEAL_MEMBERS_TABLE: &str = "deal_members";

const RECORD_PERMISSIONS: [Permission; 6] = [
    Permission::DEALS_READ,
    Permission::DEALS_CREATE,
    Permission::DEALS_UPDATE,
    Permission::DEALS_DELETE,
    Permission::DEALS_EXPORT,
    Permission::DEALS_ARCHIVE,
];

const NOTIFICATION_KINDS: [NotificationKindDefinition; 2] = [
    NotificationKindDefinition::new(crate::notifications::DEAL_ASSIGNED_NOTIFICATION)
        .email_by_default()
        .push_by_default(),
    NotificationKindDefinition::new(crate::notifications::DEAL_STAGE_CHANGED_NOTIFICATION),
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event("deal.created", DEAL_RESOURCE, "A deal was created"),
    changed(
        "deal.updated",
        DEAL_RESOURCE,
        "Fields of a deal changed (stage, pipeline and status have their own events)",
        PreviousValues::Changes,
    ),
    changed(
        "deal.stage_changed",
        DEAL_RESOURCE,
        "A deal moved to another stage",
        PreviousValues::FromTo {
            field: "stage",
            from: "from",
        },
    ),
    changed(
        "deal.pipeline_changed",
        DEAL_RESOURCE,
        "A deal moved to another pipeline",
        PreviousValues::Fields(&[("pipeline", "from"), ("stage", "from_stage")]),
    ),
    changed(
        "deal.amount_changed",
        DEAL_RESOURCE,
        "The amount of a deal changed",
        PreviousValues::OldNew { field: "amount" },
    ),
    changed(
        "deal.owner_changed",
        DEAL_RESOURCE,
        "The owner of a deal changed",
        PreviousValues::OldNew { field: "owner" },
    ),
    changed(
        "deal.expected_close_date_changed",
        DEAL_RESOURCE,
        "The expected close date of a deal changed",
        PreviousValues::OldNew {
            field: "expected_close_date",
        },
    ),
    event("deal.won", DEAL_RESOURCE, "A deal was won"),
    event("deal.lost", DEAL_RESOURCE, "A deal was lost"),
    event(
        "deal.reopened",
        DEAL_RESOURCE,
        "A won or lost deal was reopened",
    ),
    deleted(
        "deal.deleted",
        DEAL_RESOURCE,
        "A deal was moved to the trash",
    ),
    event("deal.archived", DEAL_RESOURCE, "A deal was archived"),
    event("deal.unarchived", DEAL_RESOURCE, "A deal was unarchived"),
    changed(
        "deal.member_added",
        DEAL_RESOURCE,
        "A user joined the team of a deal (`previous` has `member.user_id`, `member.role` and `member.credit_percent` as null; the team is at `GET /deals/{id}/members`)",
        PreviousValues::Changes,
    ),
    changed(
        "deal.member_updated",
        DEAL_RESOURCE,
        "The role or revenue credit of a member of a deal changed",
        PreviousValues::Changes,
    ),
    changed(
        "deal.member_removed",
        DEAL_RESOURCE,
        "A user left the team of a deal (`previous` has the user, role and credit of the member)",
        PreviousValues::Changes,
    ),
];

pub(crate) fn deal_capabilities() -> EntityTraits {
    EntityTraits::default()
        .with_trait(aphel_sdk::tagging::TAGGABLE)
        .retention_mapping(aphel_sdk::privacy::RetentionMapping {
            recency_fields: vec!["last_activity_at".to_owned()],
        })
        .privacy_policy(aphel_sdk::privacy::EntityPrivacyPolicy {
            protected_fields: vec![
                "currency".to_owned(),
                "status".to_owned(),
                "pipeline_id".to_owned(),
                "stage_id".to_owned(),
                "stage_entered_at".to_owned(),
                "amount".to_owned(),
            ],
        })
        .line_items(aphel_sdk::line_items::LineItemMapping {
            currency_field: "currency".into(),
            amount_field: Some("amount".into()),
            exchange_rate_field: Some("exchange_rate".into()),
        })
        .with_trait("scoring.subject")
        .relatable()
        .importable()
        .import_policy(
            serde_json::from_str(include_str!("../locales/imports.json"))
                .expect("owner import metadata"),
        )
        .exportable()
        .monetary(aphel_sdk::money::MonetaryMapping {
            amount_field: "amount".into(),
            currency_field: "currency".into(),
            base_amount_field: "amount_base".into(),
            exchange_rate_field: "exchange_rate".into(),
        })
        .workload(aphel_sdk::assignment::WorkloadMapping {
            state_field: "status".into(),
            active_states: vec!["open".into()],
        })
        .staged(aphel_sdk::workflow::StagedMapping {
            rotting: true,
            state_field: Some("status".to_owned()),
            active_state: Some("open".to_owned()),
            completed_states: vec!["won".to_owned()],
            ..Default::default()
        })
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(DEAL_BULK_FIELDS)
        .web_path("deals")
        .localized_labels(crate::labels("Deals"))
        .related_record("contact_id", RecordType::CONTACT)
        .related_record("company_id", RecordType::COMPANY)
        .free_text_fields(&["lost_reason_note"])
        .locked_fields(&["title", "pipeline_id", "stage_id", "status", "currency"])
        .restore_order(4)
}

pub(crate) fn deal_definition() -> Result<EntityDefinition, AppError> {
    let definition = EntityDefinition::new(RecordType::DEAL, "deals", Permission::DEALS_READ)
        .capabilities(deal_capabilities())
        .reference(aphel_sdk::fields::SoftReference {
            field: "contact_id".to_owned(),
            target: aphel_sdk::fields::ReferenceTarget::Entity {
                entity: RecordType::CONTACT.as_str().to_owned(),
            },
            on_delete: aphel_sdk::fields::DeletePolicy::Clear,
        })
        .reference(aphel_sdk::fields::SoftReference {
            field: "company_id".to_owned(),
            target: aphel_sdk::fields::ReferenceTarget::Entity {
                entity: RecordType::COMPANY.as_str().to_owned(),
            },
            on_delete: aphel_sdk::fields::DeletePolicy::Clear,
        })
        .update_permission(Permission::DEALS_UPDATE)
        .title_field("title")
        .system_field(field("mrr", crate::labels("MRR"), FieldType::Currency)?)
        .system_field(field("arr", crate::labels("ARR"), FieldType::Currency)?)
        .system_field(field(
            "tcv",
            crate::labels("Total contract value"),
            FieldType::Currency,
        )?)
        .system_field(field(
            "contract_start",
            crate::labels("Contract start"),
            FieldType::Date,
        )?)
        .system_field(field(
            "contract_end",
            crate::labels("Contract end"),
            FieldType::Date,
        )?)
        .owner_column("owner_id")
        .soft_delete_column("deleted_at")
        .archive_column("archived_at")
        .system_field(
            field("title", crate::labels("Title"), FieldType::Text)?
                .searchable()
                .required(),
        )
        .system_field(field(
            "amount",
            crate::labels("Amount"),
            FieldType::Currency,
        )?)
        .system_field(field(
            "currency",
            crate::labels("Currency"),
            FieldType::Text,
        )?)
        .system_field(record_field(
            "pipeline_id",
            crate::labels("Pipeline"),
            RecordType::PIPELINE.as_str(),
        )?)
        .system_field(record_field(
            "stage_id",
            crate::labels("Stage"),
            RecordType::PIPELINE_STAGE.as_str(),
        )?)
        .system_field(
            field("status", crate::labels("Status"), FieldType::Select)?
                .with_option("open", crate::labels("Open"), Some("#1f6feb"))
                .with_option("won", crate::labels("Won"), Some("#1a7f37"))
                .with_option("lost", crate::labels("Lost"), Some("#cf222e")),
        )
        .system_field(field(
            "probability",
            crate::labels("Probability (%)"),
            FieldType::Number,
        )?)
        .system_field(field(
            "expected_close_date",
            crate::labels("Expected close date"),
            FieldType::Date,
        )?)
        .system_field(record_field(
            "contact_id",
            crate::labels("Contact"),
            RecordType::CONTACT.as_str(),
        )?)
        .system_field(record_field(
            "company_id",
            crate::labels("Company"),
            RecordType::COMPANY.as_str(),
        )?)
        .system_field(field("owner_id", crate::labels("Owner"), FieldType::User)?)
        .system_field(field("source", crate::labels("Source"), FieldType::Text)?)
        .system_field(record_field(
            "lost_reason_id",
            crate::labels("Lost reason"),
            RecordType::LOST_REASON.as_str(),
        )?)
        .system_field(field(
            "closed_at",
            crate::labels("Closed at"),
            FieldType::DateTime,
        )?)
        .system_field(field(
            "stage_entered_at",
            crate::labels("Stage entered at"),
            FieldType::DateTime,
        )?)
        .system_field(field(
            "last_activity_at",
            crate::labels("Last activity"),
            FieldType::DateTime,
        )?)
        .relation(RecordRelation::belongs_to(
            "company",
            crate::labels("Company"),
            RecordType::COMPANY,
            "company_id",
        ))
        .relation(RecordRelation::belongs_to(
            "contact",
            crate::labels("Contact"),
            RecordType::CONTACT,
            "contact_id",
        ))
        .relation(activities_relation())
        .system_fields(aphel_sdk::money::base_currency_fields()?);
    Ok(with_fields(definition, timestamps()?))
}

pub struct DealsModule;

impl Module for DealsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Deals", APHEL_VERSION)
            .description("Deals, stage changes, the Kanban board and the team of each deal")
            .depends_on(aphel_sdk::money::KEY, "*")
            .owned_tables(&[
                "deals",
                "deal_contacts",
                "deal_member_roles",
                "deal_members",
                "deal_stage_history",
            ])
            .migration(Migration::new(
                1,
                "create deals",
                include_str!("../migrations/0001_create_deals.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.on(aphel_kernel::records::RecordPurged, 10, |ctx, args| {
            Box::pin(async move {
                if args.record_type == RecordType::CONTACT {
                    sqlx::query("DELETE FROM deal_contacts WHERE contact_id = ?")
                        .bind(args.record.record_id.to_string())
                        .execute(&mut **ctx.tx)
                        .await?;
                }
                Ok(())
            })
        });
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::DealsSeed,
        ));
        app.contribute::<dyn aphel_sdk::merges::MergeReferences>(std::sync::Arc::new(
            merge_references::References,
        ));
        app.provide::<dyn aphel_sdk::deals::DealRenewalReader>(std::sync::Arc::new(
            renewal_writer::Writer,
        ));
        app.provide::<dyn aphel_sdk::deals::RenewalDealWriter>(std::sync::Arc::new(
            renewal_writer::Writer,
        ));
        app.contribute::<dyn aphel_sdk::party::PartyAssociationSource>(std::sync::Arc::new(
            associations::Associations,
        ));
        app.provide::<dyn aphel_sdk::deals::DealProbabilityReader>(Arc::new(
            probability::ProbabilityReader,
        ));
        app.contract(aphel_sdk::deals::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::record_context::RecordListSource>(std::sync::Arc::new(
            context_source::ContextSource,
        ));
        app.provide::<dyn aphel_sdk::deals::DealCreditReader>(Arc::new(
            team::credit_reader::CreditReader,
        ));
        app.provide::<dyn aphel_sdk::deals::DealLifecycle>(Arc::new(lifecycle::Lifecycle));
        app.contribute::<dyn aphel_sdk::followers::FollowedEventSource>(std::sync::Arc::new(
            followers::DealsFollowedEvents,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::DashboardSource>(std::sync::Arc::new(
            dashboard::widgets::DealSummarySource,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::DashboardSource>(Arc::new(
            dashboard::widgets::DealPendingSource,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(std::sync::Arc::new(dashboard::widgets::SalesIndicatorsWidget));
        app.contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(std::sync::Arc::new(dashboard::widgets::DealsByStageWidget));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::timeline::KindDefinition::new("deal_stage", "Stages", "Etapas")
                .events(&["deal.stage_changed"]),
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::timeline::KindDefinition::new(
                "deal_change",
                "Deal changes",
                "Alterações de oportunidades",
            )
            .events(&[
                "deal.amount_changed",
                "deal.owner_changed",
                "deal.expected_close_date_changed",
                "deal.won",
                "deal.lost",
                "deal.reopened",
            ]),
        ));
        app.contribute::<dyn aphel_sdk::timeline::TimelineEventScope>(std::sync::Arc::new(
            timeline::RelatedEvents,
        ));
        app.contribute::<dyn aphel_sdk::line_items::LineItemProjection>(Arc::new(
            line_items::Projection,
        ));
        app.provide::<dyn aphel_sdk::deals::DealFinancials>(Arc::new(financials::Financials));
        app.contribute::<dyn aphel_sdk::workflow::StagedRecordSource>(std::sync::Arc::new(
            workflow_source::DealStages,
        ));
        app.extend(RecordType::DEAL.as_str(), |deal| {
            let mut items = aphel_sdk::platform::application::fields::ItemTable::new(
                "items",
                crate::labels("Items"),
                "line_items",
                "record_id",
            )
            .partition("record_type", RecordType::DEAL.as_str())
            .column("total", crate::labels("Total"))
            .column("quantity", crate::labels("Quantity"))
            .column("unit_price", crate::labels("Unit price"))
            .column("discount_value", crate::labels("Discount"))
            .column("tax_percent", crate::labels("Tax"));
            items.contributed_by = Some(aphel_sdk::line_items::KEY);
            deal.item_table(items);
        });
        app.trigger(std::sync::Arc::new(rotting::DealRotting));
        app.contribute(std::sync::Arc::new(
            aphel_contract_automation::application::time::triggers::TimeTriggerPlanner {
                kind: rotting::TIME_DEAL_ROTTING,
                plan: rotting::rotting_plan,
                catch_up: Some(
                    aphel_contract_automation::application::time::triggers::TimeCatchUp {
                        record_types: &["deals.deal"],
                        event_type: aphel_sdk::workflow::rotting::PIPELINE_STAGE_ROTTING_TIGHTENED,
                        event_key: aphel_sdk::workflow::rotting::tightened_stage,
                        filter: aphel_sdk::workflow::rotting::rotting_catch_up_filter,
                    },
                ),
            },
        ));

        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            crate::notifications::DealNotificationTexts,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationSource>(std::sync::Arc::new(
            notifications::DealNotifications,
        ));
        app.declare_hook(
            crate::hooks::DealStageChanging,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a deal moves to another stage; an error blocks the move",
        );
        app.declare_hook(
            crate::hooks::DealWinning,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a deal is marked as won; an error blocks it",
        );
        app.declare_hook(
            crate::hooks::DealLosing,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before a deal is marked as lost; an error blocks it",
        );
        app.declare_hook(
            crate::hooks::DealWon,
            &[aphel_kernel::modules::HookUsage::Action],
            "After a deal was won, inside the same transaction",
        );
        app.declare_hook(
            crate::hooks::DealLost,
            &[aphel_kernel::modules::HookUsage::Action],
            "After a deal was lost, inside the same transaction",
        );
        let deal_hook = |key, kind, description| {
            aphel_kernel::modules::PluginHook::new(key, "deals.deal", kind, description)
        };
        let veto = aphel_kernel::modules::PluginHookKind::Veto;
        let action = aphel_kernel::modules::PluginHookKind::Action;
        app.forward_hook(
            crate::hooks::DealStageChanging,
            deal_hook(crate::hooks::STAGE_CHANGING, veto, "Before a deal moves to another stage; values: { deal, previous, from, to }. An error blocks the move"),
        )
        .forward_hook(
            crate::hooks::DealWinning,
            deal_hook(crate::hooks::WINNING, veto, "Before a deal is won; values: { deal, previous, stage, lost_reason }. An error blocks it"),
        )
        .forward_hook(
            crate::hooks::DealLosing,
            deal_hook(crate::hooks::LOSING, veto, "Before a deal is lost; values: { deal, previous, stage, lost_reason }. An error blocks it"),
        )
        .forward_hook(
            crate::hooks::DealWon,
            deal_hook(crate::hooks::WON, action, "After a deal is won, inside the transaction; values: { deal, previous, stage }"),
        )
        .forward_hook(
            crate::hooks::DealLost,
            deal_hook(crate::hooks::LOST, action, "After a deal is lost, inside the transaction; values: { deal, previous, stage, lost_reason }"),
        );
        exposed_operations::register(app);

        app.provide::<dyn aphel_contract_automation::application::StageOperations>(
            std::sync::Arc::new(operations::AutomationStages),
        );
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(170, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::team::section::DealMemberRolesSection::new(
                    pool.clone(),
                ))
            }),
        ));
        app.filter(SearchPlanFilter, DEFAULT_PRIORITY, |ctx, plan| {
            Box::pin(hooks::restrict_search(ctx, plan))
        });
        app.filter(RecordQueryFilter, DEFAULT_PRIORITY, |ctx, constraints| {
            Box::pin(hooks::restrict_query(ctx, constraints))
        });
        app.trigger(Arc::new(crate::automation::StageChanged))
            .triggers(crate::automation::deal_closed_triggers())
            .action(Arc::new(crate::automation::MoveStage));
        app.entity(deal_definition()?.handler(Arc::new(http::DealHandler)))
            .extend("deals.deal", |deal| {
                deal.member_access(DEAL_MEMBERS_TABLE, "deal_id", "user_id");
            })
            .openapi_tag(http::TAG, "Deals, stage changes and the Kanban board")
            .openapi_tag(
                reports::http::TAG,
                "Deal conversion and sales performance reports",
            )
            .routes(|_| http::router().merge(reports::http::router()))
            .consumer(|env| Arc::new(runtime::DealLastActivityConsumer::new(env.pool.clone())))
            .consumer(|env| Arc::new(runtime::DealStageHistoryConsumer::new(env.pool.clone())));
        app.contribute::<dyn aphel_sdk::money::MoneyRecalculationSource>(Arc::new(
            runtime::DealRecalculation,
        ));
        for permission in RECORD_PERMISSIONS {
            let definition = PermissionDefinition::record(permission.clone());
            if permission == Permission::DEALS_ARCHIVE {
                app.permission(definition)
                    .grant_to(["admin", "manager", "member"]);
            } else {
                app.permission(definition);
            }
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for definition in NOTIFICATION_KINDS {
            app.contribute(std::sync::Arc::new(definition));
        }
        team::register(app)?;
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_sdk::platform::application::fields::LocalizedLabels {
    aphel_sdk::platform::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
