mod dataset_seed;
mod defaults;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod engine;
pub(crate) mod exposed_operations;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod notifications;
pub(crate) mod runtime;
pub(crate) mod section;
pub mod service;
pub(crate) mod store;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::RecordValidating;
use aphel_sdk::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_sdk::platform::application::public_events::{
    PublicEventDefinition, PublicResource, event,
};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use hooks::{ApprovalGate, EngineApprovalGate, approval_gate};

const DEAL_RESOURCE: PublicResource =
    PublicResource::new(aphel_contract_crm::keys::DEAL, "DealResponse");

pub const KEY: &str = "approvals";
pub(crate) const APPROVAL_PRIORITY: i32 = 10;

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "approval.requested",
        DEAL_RESOURCE,
        "An approval was requested for the deal (stage change or on demand)",
    ),
    event(
        "approval.approved",
        DEAL_RESOURCE,
        "Every step of an approval request of the deal was approved",
    ),
    event(
        "approval.rejected",
        DEAL_RESOURCE,
        "An approval request of the deal was rejected",
    ),
    event(
        "approval.cancelled",
        DEAL_RESOURCE,
        "An approval request of the deal was cancelled",
    ),
];

const NOTIFICATION_KINDS: &[NotificationKindDefinition] = &[
    NotificationKindDefinition {
        kind: crate::notifications::APPROVAL_REQUESTED_NOTIFICATION,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: true,
    },
    NotificationKindDefinition {
        kind: crate::notifications::APPROVAL_DECIDED_NOTIFICATION,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: true,
    },
    NotificationKindDefinition {
        kind: crate::notifications::APPROVAL_OVERDUE_NOTIFICATION,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: true,
    },
];

pub struct ApprovalsModule;

impl Module for ApprovalsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Approvals", APHEL_VERSION)
            .description(
                "Approval policies (record type, moment, condition, sequential steps), approval requests with decisions and the approval gate of record changes",
            )
            .enabled_by_default()
            .owned_tables(&["approval_policies", "approval_policy_steps", "approval_request_approvers", "approval_requests"])
            .migration(Migration::new(
                1,
                "create approvals",
                include_str!("../migrations/0001_create_approvals.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::ApprovalsSeed,
        ));
        app.contract(aphel_sdk::approvals::DESCRIPTOR);
        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::infrastructure::defaults::seed::DefaultsSeed(|connection| {
                Box::pin(defaults::seed_templates(connection))
            }),
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            crate::notifications::ApprovalNotificationTexts,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationSource>(std::sync::Arc::new(
            notifications::ApprovalNotifications,
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(210, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::ApprovalPoliciesSection::new(
                    registry.clone(),
                    crate::store::ApprovalStore::new(pool.clone()),
                    crate::store::ApprovalDirectory::new(pool.clone(), registry.clone()),
                    pool.clone(),
                ))
            }),
        ));
        app.triggers(automation::approval_triggers())
            .action(Arc::new(automation::RequestApproval));
        app.on(RecordValidating, APPROVAL_PRIORITY, |ctx, args| {
            Box::pin(hooks::guard_write(ctx, args))
        })
        .provide::<dyn ApprovalGate>(Arc::new(EngineApprovalGate))
        .openapi_tag(
            http::TAG,
            "Approval policies (record type, moment, condition, sequential steps) and approval requests with decisions",
        )
        .routes(|_| http::router())
        .worker(|env| {
            let runtime = runtime::ApprovalRuntime::new(env.pool.clone(), env.record_types.clone());
            aphel_kernel::modules::WorkerParts::default()
                .consumer(runtime::ApprovalCleanupConsumer::shared(runtime.clone()))
                .handler(runtime::ApprovalOverdueHandler::shared(runtime))
                .schedule(runtime::approvals_overdue_schedule())
        });
        app.permission(PermissionDefinition::administrative(
            Permission::APPROVALS_MANAGE,
        ))
        .grant_to(["admin"]);
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for definition in NOTIFICATION_KINDS {
            app.contribute(std::sync::Arc::new(*definition));
        }
        exposed_operations::register(app);
        Ok(())
    }
}
