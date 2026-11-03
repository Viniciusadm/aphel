pub mod gateway;
pub mod gateway_impl;
mod operation_action;
mod timeline;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod actions;
pub(crate) mod clock;
pub(crate) mod conditions;
pub(crate) mod dto;
pub(crate) mod engine;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod model;
pub(crate) mod plugin_bridge;
mod points;
pub(crate) mod runtime;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod time;
pub(crate) mod triggers;
pub(crate) mod validation;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

pub(crate) use aphel_contract_automation::application::definition::*;
pub(crate) use aphel_contract_automation::application::{AutomationTrigger, MessageChannel};
pub(crate) use model::*;
pub(crate) use runtime::*;
pub(crate) use service::{
    CreateAutomationCommand, DryRunCommand, DryRunResult, UpdateAutomationCommand,
};

use ::notifications::domain::{
    ALL_CHANNELS, EMAIL_ONLY, IN_APP_AND_PUSH, NotificationKind, NotificationKindDefinition,
};
use actions::{AssignOwner, CreateRelatedRecord, SendMessage, UpdateFields};
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerEnv, WorkerParts};
use time::{
    TimeCatchUpConsumer, TimeTriggerSweep, WaitingRunsCanceller, WaitingRunsResumer,
    resume_waiting_schedule, time_sweep_schedule,
};
use triggers::{Manual, RecordCreated, RecordDeleted, RecordUpdated};

pub(crate) const KEY: &str = "automations";

const NOTIFICATION_KINDS: &[NotificationKindDefinition] = &[
    NotificationKindDefinition {
        kind: NotificationKind::AUTOMATION_MESSAGE,
        channels: IN_APP_AND_PUSH,
        email_default: false,
        push_default: false,
    },
    NotificationKindDefinition {
        kind: NotificationKind::AUTOMATION_EMAIL,
        channels: EMAIL_ONLY,
        email_default: true,
        push_default: false,
    },
    NotificationKindDefinition {
        kind: NotificationKind::AUTOMATION_SKIPPED,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: false,
    },
];

pub struct AutomationsModule;

impl Module for AutomationsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Automations", APHEL_VERSION)
            .description(
                "No-code automations: triggers, conditions and actions over records, versions, dry runs, time triggers, waits and run history",
            )
            .enabled_by_default()
            .owned_tables(&["automation_run_steps", "automation_runs", "automation_time_cursors", "automation_versions", "automations"])
            .migration(Migration::new(
                1,
                "create automations",
                include_str!("../migrations/0001_create_automations.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::timeline::TimelineEventExtension>(std::sync::Arc::new(
            timeline::AutomationActors,
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "automations",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "automation",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        app.contribute::<dyn aphel_kernel::modules::HealthSource>(std::sync::Arc::new(
            points::AutomationFailures,
        ));
        app.declare_hook(crate::hooks::ActionRunning, &[aphel_kernel::modules::HookUsage::Veto, aphel_kernel::modules::HookUsage::Filter],
            "Before an automation action runs (dry run and execution; an error skips it) and, as a filter, the result recorded for the step",
        );
        app.on(
            crate::hooks::ActionRunning,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, running| Box::pin(plugin_bridge::action_running(ctx, running)),
        );
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(165, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::AutomationsSection::new(
                    registry.clone(),
                    crate::store::AutomationStore::new(pool.clone()),
                    tags::TagStore::new(pool.clone()),
                    workflow::store::PipelineStore::new(pool.clone()),
                    ::config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone()),
                    tasks::store::ActivityTypeStore::new(pool.clone()),
                ))
            }),
        ));
        app.trigger(Arc::new(RecordCreated))
            .trigger(Arc::new(RecordUpdated))
            .trigger(Arc::new(RecordDeleted))
            .trigger(Arc::new(Manual))
            .triggers(time::time_triggers())
            .action(Arc::new(UpdateFields))
            .action(Arc::new(AssignOwner))
            .action(Arc::new(SendMessage {
                channel: MessageChannel::InApp,
            }))
            .action(Arc::new(SendMessage {
                channel: MessageChannel::Email,
            }))
            .action(Arc::new(CreateRelatedRecord));
        app.openapi_tag(
            http::TAG,
            "No-code automations: triggers, conditions and actions over records, versions, dry runs and run history",
        )
        .routes(|_| http::router())
        .worker(worker);
        app.permission(PermissionDefinition::administrative(
            Permission::AUTOMATIONS_MANAGE,
        ))
        .grant_to(["admin"]);
        for definition in NOTIFICATION_KINDS {
            app.contribute(std::sync::Arc::new(*definition));
        }
        for hook in plugin_bridge::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
        }
        app.contribute::<dyn aphel_contract_automation::application::AutomationActionSource>(
            std::sync::Arc::new(BoundOperations),
        );
        Ok(())
    }
}

fn worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.state();
    let runtime = Arc::new(AutomationRuntime::new(state));
    WorkerParts::default()
        .consumer(Arc::new(AutomationTriggerConsumer(runtime.clone())))
        .handler(Arc::new(AutomationRunHandler(runtime.clone())))
        .consumer(Arc::new(WaitingRunsCanceller(runtime.clone())))
        .consumer(Arc::new(TimeCatchUpConsumer(runtime.clone())))
        .handler(Arc::new(TimeTriggerSweep(runtime.clone())))
        .handler(Arc::new(WaitingRunsResumer(runtime)))
        .schedules(vec![time_sweep_schedule(), resume_waiting_schedule()])
}

struct BoundOperations;
impl aphel_contract_automation::application::AutomationActionSource for BoundOperations {
    fn actions(&self) -> Vec<aphel_contract_automation::Action> {
        aphel_kernel::modules::app()
            .contributions::<aphel_kernel::modules::operations::OperationEntry>()
            .into_iter()
            .filter_map(operation_action::OperationAction::new)
            .map(|action| std::sync::Arc::new(action) as aphel_contract_automation::Action)
            .collect()
    }
}
