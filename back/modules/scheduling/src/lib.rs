use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub(crate) mod dto;
pub(crate) mod email;
pub(crate) mod http;
pub(crate) mod ics;
pub(crate) mod model;
pub(crate) mod payload;
pub(crate) mod plugin_bridge;
pub(crate) mod public_http;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod slots;
pub(crate) mod store;
pub mod token;
pub(crate) mod validation;

use aphel_contract_crm::CrmPermissions;
use aphel_kernel::modules::Migration;
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use model::*;
pub use payload::*;

pub(crate) const KEY: &str = "scheduling";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "meeting.booked",
        people::CONTACT_RESOURCE,
        "The contact booked a meeting through a scheduling link",
    ),
    event(
        "meeting.rescheduled",
        people::CONTACT_RESOURCE,
        "A meeting booked by the contact through a scheduling link was rescheduled",
    ),
    event(
        "meeting.canceled",
        people::CONTACT_RESOURCE,
        "A meeting booked by the contact through a scheduling link was canceled",
    ),
];

pub struct SchedulingModule;

impl Module for SchedulingModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Scheduling", APHEL_VERSION)
            .description(
                "Scheduling links: availability, meeting types, team links and the public booking page",
            )
            .depends_on(activities::KEY, "*")
            .depends_on(people::KEY, "*")
            .optional("lead_distribution")
            .enabled_by_default()
            .owned_tables(&["meeting_bookings", "meeting_types"])
            .migration(Migration::new(
                1,
                "create scheduling",
                include_str!("../migrations/0001_create_scheduling.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Scheduling links: availability, meeting types, team links and the public booking page",
        )
        .routes(|_| {
            OpenApiRouter::new()
                .merge(http::router())
                .merge(public_http::router())
        })
        .triggers(automation::meeting_triggers())
        .worker(runtime::booking_worker);
        app.permission(PermissionDefinition::record(Permission::SCHEDULING_MANAGE))
            .grant_to(["admin", "manager", "member"]);
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
