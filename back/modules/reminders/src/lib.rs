pub(crate) mod delivery;
pub(crate) mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod http;
pub(crate) mod notifications;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerEnv, WorkerParts};
use aphel_sdk::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "reminders";

pub struct RemindersModule;

fn worker(env: &WorkerEnv) -> WorkerParts {
    WorkerParts::default()
        .handler(delivery::reminder_delivery_handler(env.pool.clone()))
        .handler(delivery::tasks_due_today_handler(env.pool.clone()))
        .schedule(delivery::reminders_deliver_schedule())
        .schedule(delivery::tasks_due_today_schedule())
}

impl Module for RemindersModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Reminders", APHEL_VERSION)
            .description(
                "Reminders before a task is due, the morning notice of tasks due today and standalone reminders linked to a record",
            )
            .optional("tasks")
            .enabled_by_default()
            .owned_tables(&["reminders"])
            .migration(Migration::new(
                1,
                "create reminders",
                include_str!("../migrations/0001_create_reminders.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            crate::notifications::ReminderNotificationTexts,
        ));
        app.openapi_tag(
            http::TAG,
            "Reminders: before a task is due and standalone reminders linked to a record",
        )
        .routes(|_| http::router())
        .contribute(std::sync::Arc::new(NotificationKindDefinition {
            kind: crate::notifications::REMINDER_NOTIFICATION,
            channels: ALL_CHANNELS,
            email_default: true,
            push_default: true,
        }))
        .worker(worker);
        exposed_operations::register(app);
        Ok(())
    }
}
