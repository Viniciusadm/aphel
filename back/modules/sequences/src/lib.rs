mod config_references;
mod contacts;
mod enrollment_service;
mod reference_cleanup;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod dto;
pub(crate) mod events;
pub(crate) mod exits;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod model;
pub(crate) mod plugin_hooks;
pub(crate) mod runner;
pub(crate) mod runtime;
pub(crate) mod section;
pub mod service;
pub(crate) mod store;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::application::public_events::{PublicEventDefinition, event};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) use automation::*;
pub(crate) use events::*;
pub(crate) use exits::*;
pub use model::*;
pub(crate) use runner::{SEQUENCE_STEP_JOB, SEQUENCES_TICK_CRON, SEQUENCES_TICK_JOB};
pub use service::{parse_window, validate_steps};

pub(crate) const KEY: &str = "sequences";

const SEQUENCE_CONTEXT: &[&str] = &[
    "sequence_id",
    "sequence_key",
    "sequence_name",
    "exit_reason",
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "sequence.enrolled",
        aphel_sdk::people::RESOURCE,
        "A contact was enrolled in a sequence",
    )
    .with_context(SEQUENCE_CONTEXT),
    event(
        "sequence.completed",
        aphel_sdk::people::RESOURCE,
        "A contact went through every step of a sequence",
    )
    .with_context(SEQUENCE_CONTEXT),
    event(
        "sequence.exited",
        aphel_sdk::people::RESOURCE,
        "A contact left a sequence (reply, meeting, deal won or lost, bounce, objection or removal)",
    )
    .with_context(SEQUENCE_CONTEXT),
];

pub struct SequencesModule;

impl Module for SequencesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Sequences", APHEL_VERSION)
            .description(
                "Sales sequences (cadences): steps, enrollments, automatic exits, sending limits and per-step metrics",
            )
            .optional("email")
            .optional("people")
            .depends_on("tasks", "*")
            .enabled_by_default()
            .owned_tables(&["sequence_enrollments", "sequence_step_runs", "sequence_steps", "sequences"])
            .migration(Migration::new(
                1,
                "create sequences",
                include_str!("../migrations/0001_create_sequences.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        reference_cleanup::register(app);
        app.contract(aphel_sdk::sequences::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::sequences::SequenceEnrollment>(std::sync::Arc::new(
            enrollment_service::Enrollment,
        ));
        app.contribute::<dyn aphel_sdk::configuration::ConfigurationReferences>(
            std::sync::Arc::new(config_references::References),
        );
        app.declare_hook(crate::hooks::StepRunning, &[aphel_kernel::modules::HookUsage::Veto],
            "Before a sequence step runs; a definitive error removes the contact from the sequence, a transient one retries the job",
        );
        app.on(
            crate::hooks::StepRunning,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, step| Box::pin(plugin_hooks::step_running(ctx, step)),
        );
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(160, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::SequencesSection::new(
                    crate::store::SequenceStore::new(pool.clone()),
                ))
            }),
        ));
        app.openapi_tag(
            http::TAG,
            "Sales sequences (cadences): steps, enrollments, automatic exits, sending limits and per-step metrics",
        )
        .routes(|_| OpenApiRouter::new().merge(http::router()))
        .action(Arc::new(EnrollInSequence))
        .worker(runtime::sequence_worker);
        for permission in [Permission::SEQUENCES_READ, Permission::SEQUENCES_ENROLL] {
            app.permission(PermissionDefinition::record(permission))
                .grant_to(["admin", "manager", "member"]);
        }
        app.permission(PermissionDefinition::administrative(
            Permission::SEQUENCES_MANAGE,
        ))
        .grant_to(["admin"]);
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for hook in plugin_hooks::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
        }
        plugin_hooks::register(app);
        Ok(())
    }
}
