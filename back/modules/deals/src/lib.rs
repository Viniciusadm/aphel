pub(crate) mod automation;
pub mod domain;
pub mod hooks;
pub mod http;
pub(crate) mod notifications;
pub(crate) mod operations;
pub(crate) mod plugin_bridge;
pub(crate) mod runtime;
pub mod service;
pub mod store;
pub mod team;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes, activities_relation};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::{field, record_field, timestamps, with_fields};
use aphel_kernel::application::fields::{RecordCapabilities, RecordRelation, RecordTypeDefinition};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::domain::notifications::NotificationKindDefinition;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::crm::seed::CrmDefaultsSeed;
use aphel_kernel::modules::DEFAULT_PRIORITY;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::{RecordQueryFilter, SearchPlanFilter};

pub use hooks::{
    AmountCalculator, ClosingDeal, DealLosing, DealLost, DealStageChanging, DealWinning, DealWon,
    ManualAmountCalculator, StageChange,
};

pub const DEAL_RESOURCE: PublicResource = PublicResource::new("deal", "DealResponse");

pub const KEY: &str = "deals";

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

pub(crate) fn deal_capabilities() -> RecordCapabilities {
    RecordCapabilities::default()
        .relatable()
        .importable()
        .exportable()
        .stages()
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(DEAL_BULK_FIELDS)
        .web_path("deals")
        .plural_labels("Oportunidades", "Deals")
        .related_record("contact_id", RecordType::CONTACT)
        .related_record("company_id", RecordType::COMPANY)
        .free_text_fields(&["lost_reason_note"])
        .locked_fields(&["title", "pipeline_id", "stage_id", "status", "currency"])
        .restore_order(4)
}

pub(crate) fn deal_definition() -> Result<RecordTypeDefinition, AppError> {
    let definition = RecordTypeDefinition::new(RecordType::DEAL, "deals", Permission::DEALS_READ)
        .capabilities(deal_capabilities())
        .update_permission(Permission::DEALS_UPDATE)
        .title_field("title")
        .owner_column("owner_id")
        .soft_delete_column("deleted_at")
        .archive_column("archived_at")
        .system_field(
            field("title", "Título", FieldType::Text)?
                .searchable()
                .required(),
        )
        .system_field(field("amount", "Valor", FieldType::Currency)?)
        .system_field(field("currency", "Moeda", FieldType::Text)?)
        .system_field(record_field(
            "pipeline_id",
            "Funil",
            RecordType::PIPELINE.as_str(),
        )?)
        .system_field(record_field(
            "stage_id",
            "Etapa",
            RecordType::PIPELINE_STAGE.as_str(),
        )?)
        .system_field(
            field("status", "Situação", FieldType::Select)?
                .with_option("open", "Em aberto", Some("#1f6feb"))
                .with_option("won", "Ganha", Some("#1a7f37"))
                .with_option("lost", "Perdida", Some("#cf222e")),
        )
        .system_field(field(
            "probability",
            "Probabilidade (%)",
            FieldType::Number,
        )?)
        .system_field(field(
            "expected_close_date",
            "Previsão de fechamento",
            FieldType::Date,
        )?)
        .system_field(record_field(
            "contact_id",
            "Contato",
            RecordType::CONTACT.as_str(),
        )?)
        .system_field(record_field(
            "company_id",
            "Empresa",
            RecordType::COMPANY.as_str(),
        )?)
        .system_field(field("owner_id", "Responsável", FieldType::User)?)
        .system_field(field("source", "Origem", FieldType::Text)?)
        .system_field(record_field(
            "lost_reason_id",
            "Motivo da perda",
            RecordType::LOST_REASON.as_str(),
        )?)
        .system_field(field("closed_at", "Fechada em", FieldType::DateTime)?)
        .system_field(field(
            "stage_entered_at",
            "Entrou na etapa em",
            FieldType::DateTime,
        )?)
        .system_field(field(
            "last_activity_at",
            "Última atividade",
            FieldType::DateTime,
        )?)
        .relation(RecordRelation::belongs_to(
            "company",
            "Empresa",
            RecordType::COMPANY,
            "company_id",
        ))
        .relation(RecordRelation::belongs_to(
            "contact",
            "Contato",
            RecordType::CONTACT,
            "contact_id",
        ))
        .relation(activities_relation());
    Ok(with_fields(definition, timestamps()?))
}

pub struct DealsModule;

impl Module for DealsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Deals", APHEL_VERSION)
            .description("Deals, stage changes, the Kanban board and the team of each deal")
            .required()
            .depends_on(pipelines::KEY, "*")
            .depends_on(currencies::KEY, "*")
            .depends_on(contacts::KEY, "*")
            .depends_on(companies::KEY, "*")
            .owned_tables(&[
                "deals",
                "deal_checklist_progress",
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
        app.contribute::<dyn aphel_kernel::application::notifications::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::DealNotificationTexts),
        );
        app.contribute::<dyn aphel_kernel::application::notifications::NotificationSource>(
            std::sync::Arc::new(notifications::DealNotifications),
        );
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
        app.on(
            crate::hooks::DealStageChanging,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, change| Box::pin(plugin_bridge::stage_changing(ctx, change)),
        )
        .on(
            crate::hooks::DealWinning,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, deal| Box::pin(plugin_bridge::closing(ctx, plugin_bridge::WINNING, deal)),
        )
        .on(
            crate::hooks::DealLosing,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, deal| Box::pin(plugin_bridge::closing(ctx, plugin_bridge::LOSING, deal)),
        )
        .on(
            crate::hooks::DealWon,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, deal| Box::pin(plugin_bridge::closing(ctx, plugin_bridge::WON, deal)),
        )
        .on(
            crate::hooks::DealLost,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, deal| Box::pin(plugin_bridge::closing(ctx, plugin_bridge::LOST, deal)),
        );
        for hook in plugin_bridge::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        app.provide::<dyn aphel_kernel::application::automations::StageOperations>(
            std::sync::Arc::new(operations::AutomationStages),
        );
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                170,
                |pool, registry| {
                    let _ = registry;
                    std::sync::Arc::new(crate::team::section::DealMemberRolesSection::new(
                        pool.clone(),
                    ))
                },
            ),
        ));
        app.contribute(Arc::new(CrmDefaultsSeed(|connection| {
            Box::pin(team::store::seed_deal_member_roles(connection))
        })));
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
            .extend("deal", |deal| {
                deal.member_access(DEAL_MEMBERS_TABLE, "deal_id", "user_id");
            })
            .provide::<dyn AmountCalculator>(Arc::new(ManualAmountCalculator))
            .openapi_tag(http::TAG, "Deals, stage changes and the Kanban board")
            .routes(|_| http::router())
            .consumer(|env| Arc::new(runtime::DealLastActivityConsumer::new(env.pool.clone())))
            .consumer(|env| Arc::new(runtime::DealStageHistoryConsumer::new(env.pool.clone())))
            .job(|env| Arc::new(runtime::CurrencyRecalculateHandler::new(env.pool.clone())));
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
            app.notification_kind(definition);
        }
        team::register(app)?;
        Ok(())
    }
}
