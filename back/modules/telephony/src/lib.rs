pub(crate) mod call;
pub mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod hooks;
pub(crate) mod http;
mod reference_cleanup;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod telephony;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, DEFAULT_PRIORITY, Manifest, Module};
use aphel_kernel::records::RecordSerializing;
use aphel_sdk::platform::application::public_events::{
    PreviousValues, PublicEventDefinition, changed, event,
};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) use telephony::TelephonyRouter;

pub(crate) const KEY: &str = "telephony";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "call.logged",
        aphel_sdk::tasks::RESOURCE,
        "A call was logged as an activity (details in `call`: direction, outcome, duration)",
    ),
    changed(
        "call.updated",
        aphel_sdk::tasks::RESOURCE,
        "The details of a logged call changed",
        PreviousValues::Changes,
    ),
];

pub struct TelephonyModule;

impl Module for TelephonyModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Calls", APHEL_VERSION)
            .description("Call logging as tasks, call outcomes, call sessions, telephony providers and the calls report")
            .depends_on("tasks", "*")
            .depends_on("people", "*")
            .depends_on("saved_views", "*")
            .enabled_by_default()
            .owned_tables(&["activity_calls", "call_outcomes", "call_session_items", "call_sessions"])
            .migration(Migration::new(
                1,
                "create call outcomes",
                include_str!("../migrations/0001_create_call_outcomes.sql"),
            ))
            .migration(Migration::new(
                2,
                "create activity calls",
                include_str!("../migrations/0002_create_activity_calls.sql"),
            ))
            .migration(Migration::new(
                3,
                "create call sessions",
                include_str!("../migrations/0003_create_call_sessions.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        reference_cleanup::register(app);
        app.contract(aphel_sdk::telephony::DESCRIPTOR);
        app.contribute(Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(95, |pool, _registry| {
                Arc::new(crate::section::call_outcomes_section(pool.clone()))
            }),
        ));
        app.openapi_tag(
            http::TAG,
            "Call logging as tasks, call outcomes, call sessions over saved views and the calls report",
        )
        .routes(|_| http::router())
        .provide::<dyn TelephonyRouter>(telephony::shared_router())
        .filter(RecordSerializing, DEFAULT_PRIORITY, |ctx, records| {
            Box::pin(hooks::attach_calls(ctx, records))
        });
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        exposed_operations::register(app);
        Ok(())
    }
}
