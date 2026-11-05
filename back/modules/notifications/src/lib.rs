pub mod application;
mod dataset_seed;
mod delivery;
pub mod domain;
pub mod dto;
mod http;
pub mod infrastructure;
pub mod intern;
pub mod push_dto;
mod push_http;
use crate::infrastructure::notification_worker_parts;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::mail::GlobalMailerResolver;
use std::sync::Arc;
pub const KEY: &str = "notifications";
pub use aphel_sdk::notifications::DigestSource;

pub struct NotificationsModule;
impl aphel_kernel::modules::Module for NotificationsModule {
    fn manifest(&self) -> aphel_kernel::modules::Manifest {
        aphel_kernel::modules::Manifest::new(
            KEY,
            "Notifications",
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create notifications",
            include_str!("../migrations/0730_create_notifications.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            2,
            "create notification preferences",
            include_str!("../migrations/0731_create_notification_preferences.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            3,
            "create push subscriptions",
            include_str!("../migrations/2711_create_push_subscriptions.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            4,
            "add notifications pushed at",
            include_str!("../migrations/2712_add_notifications_pushed_at.sql"),
        ))
        .owned_tables(&[
            "notifications",
            "notification_preferences",
            "push_subscriptions",
        ])
    }
    fn register(&self, app: &mut aphel_kernel::modules::AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::NotificationsSeed,
        ));
        app.provide::<dyn aphel_sdk::notifications::NotificationDelivery>(Arc::new(
            delivery::Delivery,
        ));
        app.contract(aphel_contract_notifications::DESCRIPTOR);
        app.openapi_tag("notifications", "Notifications and delivery preferences")
            .routes(|_| http::router().merge(push_http::router()))
            .worker(workers);
        for kind in domain::builtin_notification_kinds() {
            app.contribute(Arc::new(*kind));
        }
        app.contribute::<dyn aphel_sdk::platform::modules::unified_catalog::CatalogExtension>(
            Arc::new(NotificationCatalog),
        );
        app.on(
            aphel_sdk::platform::application::security::SessionRevoked,
            10,
            |ctx, args| {
                Box::pin(async move {
                    if let Some(session) = &args.session_id {
                        infrastructure::push::remove_session_push_subscriptions(
                            &mut **ctx.tx,
                            session,
                        )
                        .await?;
                    }
                    if let Some(user) = &args.user_id {
                        infrastructure::push::remove_user_push_subscriptions(
                            &mut **ctx.tx,
                            user,
                            args.keep_session.as_ref(),
                        )
                        .await?;
                    }
                    Ok(())
                })
            },
        );
        Ok(())
    }
}
struct NotificationCatalog;
impl aphel_sdk::platform::modules::unified_catalog::CatalogExtension for NotificationCatalog {
    fn project(&self, app: &aphel_kernel::modules::App) -> (String, serde_json::Value) {
        ("notification_kinds".to_owned(),serde_json::Value::Array(app.contributions::<domain::NotificationKindDefinition>().into_iter().map(|kind|serde_json::json!({"kind":kind.kind.as_str(),"email_default":kind.email_default,"push_default":kind.push_default})).collect()))
    }
}
fn workers(env: &aphel_kernel::modules::WorkerEnv) -> aphel_kernel::modules::WorkerParts {
    let pool = &env.pool;
    let mut parts = aphel_kernel::modules::WorkerParts::default();
    match crate::infrastructure::push::push_delivery_handler(
        pool.clone(),
        env.secrets.clone(),
        &env.public_url.get(),
    ) {
        Ok(handler) => parts = parts.handler(handler),
        Err(error) => tracing::warn!(%error, "web push delivery disabled"),
    }
    let mailers = Arc::new(GlobalMailerResolver::new(env.mailer.clone()));
    let notifications = notification_worker_parts(pool.clone(), mailers, env.public_url.clone());
    parts
        .consumer(notifications.consumer)
        .handlers(notifications.handlers)
        .schedules(notifications.schedules)
}
