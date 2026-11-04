pub(crate) mod domain;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod widget;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerEnv, WorkerParts};

use crate::runtime::{GoalOwnerRemovedConsumer, PurgeOrphanGoalsHandler, purge_orphans_schedule};

pub(crate) use widget::GoalsProgressWidget;

pub(crate) const KEY: &str = "goals";

pub struct GoalsModule;

impl Module for GoalsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Goals", APHEL_VERSION)
            .description(
                "Sales goals per user, team or organization (won amount, won deals, completed tasks, new people) and their progress",
            )
            .depends_on(deals::KEY, "*")
            .enabled_by_default()
            .owned_tables(&["goals"])
            .migration(Migration::new(
                1,
                "create goals",
                include_str!("../migrations/0001_create_goals.sql"),
            ))
            .migration(Migration::new(
                2,
                "goal owner set null",
                include_str!("../migrations/0002_goal_owner_set_null.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Sales goals per user, team or organization and their progress",
        )
        .routes(|_| http::router())
        .contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(
            Arc::new(GoalsProgressWidget),
        )
        .worker(worker);
        app.permission(PermissionDefinition::record(Permission::GOALS_MANAGE))
            .grant_to(["admin", "manager", "member"]);
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}

fn worker(env: &WorkerEnv) -> WorkerParts {
    let purge = PurgeOrphanGoalsHandler {
        pool: env.pool.clone(),
        catalog: env.record_types.clone(),
    };
    WorkerParts::default()
        .consumer(Arc::new(GoalOwnerRemovedConsumer(purge.clone())))
        .handler(Arc::new(purge))
        .schedule(purge_orphans_schedule())
}
