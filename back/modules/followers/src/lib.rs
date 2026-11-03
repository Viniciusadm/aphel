pub(crate) mod domain;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod notifications;
pub(crate) mod plugin_bridge;
pub(crate) mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use ::notifications::application::RecordFollowerSource;
use ::notifications::domain::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "followers";

pub struct FollowersModule;

impl Module for FollowersModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Followers", APHEL_VERSION)
            .description(
                "Following records to be notified of their changes, tasks, notes and emails",
            )
            .enabled_by_default()
            .owned_tables(&["record_followers"])
            .migration(Migration::new(
                1,
                "create record followers",
                include_str!("../migrations/0001_create_record_followers.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn ::notifications::application::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::FollowerNotificationTexts),
        );
        app.openapi_tag(
            http::TAG,
            "Following records to be notified of their changes, tasks, notes and emails",
        )
        .routes(|_| http::router());
        app.contribute(std::sync::Arc::new(NotificationKindDefinition {
            kind: crate::notifications::FOLLOWED_RECORD_ACTIVITY_NOTIFICATION,
            channels: ALL_CHANNELS,
            email_default: false,
            push_default: false,
        }));
        app.provide::<dyn RecordFollowerSource>(Arc::new(service::FollowerSource));
        app.contribute::<dyn ::notifications::application::NotificationSource>(Arc::new(
            notifications::FollowedRecordNotifications,
        ));
        app.consumer(|env| {
            hooks::OwnerFollowConsumer::shared(env.pool.clone(), env.record_types.clone())
        });
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
