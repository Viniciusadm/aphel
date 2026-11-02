pub(crate) mod automation;
pub(crate) mod events_dto;
pub(crate) mod events_http;
pub(crate) mod incoming;
pub(crate) mod notifications;
pub(crate) mod operations;
pub(crate) mod outgoing;
mod points;

use std::sync::Arc;

use aphel_kernel::application::security::{PersonalTextColumn, PersonalTextSource};
use aphel_kernel::modules::Migration;

use aphel_kernel::domain::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerParts};
use incoming::{incoming_webhooks_purge_handler, incoming_webhooks_purge_schedule};
use outgoing::outgoing_webhook_worker;

pub(crate) const KEY: &str = "webhooks";

const NOTIFICATION_KINDS: &[NotificationKindDefinition] = &[NotificationKindDefinition {
    kind: crate::notifications::WEBHOOK_ENDPOINT_DISABLED_NOTIFICATION,
    channels: ALL_CHANNELS,
    email_default: true,
    push_default: false,
}];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WebhookPersonalText;

impl PersonalTextSource for WebhookPersonalText {
    fn personal_text_columns(&self) -> Vec<PersonalTextColumn> {
        vec![
            PersonalTextColumn::new("webhook_deliveries", "body"),
            PersonalTextColumn::new("webhook_deliveries", "response_body"),
            PersonalTextColumn::new("incoming_webhook_requests", "body"),
            PersonalTextColumn::new("incoming_webhook_requests", "headers"),
            PersonalTextColumn::new("incoming_webhook_requests", "results"),
        ]
    }
}

pub struct WebhooksModule;

impl Module for WebhooksModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Webhooks", APHEL_VERSION)
            .description(
                "Outgoing webhooks over the open catalog of public events (signed deliveries, retries, redelivery) and incoming webhooks that create or update records",
            )
            .enabled_by_default()
            .owned_tables(&[
                "webhook_endpoints",
                "webhook_deliveries",
                "incoming_webhooks",
                "incoming_webhook_requests",
                "incoming_webhook_replays",
            ])
            .owned_tables(&["incoming_webhook_replays", "incoming_webhook_requests", "incoming_webhooks", "webhook_deliveries", "webhook_endpoints"])
            .migration(Migration::new(
                1,
                "create webhooks",
                include_str!("../migrations/0001_create_webhooks.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::fields::EntityKeyAliases(&[
                ("incoming_webhook", "webhooks.incoming_webhook"),
                ("webhook_delivery", "webhooks.webhook_delivery"),
                ("webhook_endpoint", "webhooks.webhook_endpoint"),
            ]),
        ));
        app.contribute::<dyn aphel_kernel::modules::HealthSource>(std::sync::Arc::new(
            points::WebhookFailures,
        ));
        app.contribute::<dyn aphel_kernel::application::notifications::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::WebhookNotificationTexts),
        );
        app.contribute::<dyn aphel_kernel::application::notifications::NotificationSource>(
            std::sync::Arc::new(notifications::WebhookDisabledNotifications),
        );
        app.contribute::<dyn aphel_sdk::points::ReferenceProvider>(std::sync::Arc::new(
            operations::IncomingWebhookRefs,
        ));
        app.contribute::<dyn PersonalTextSource>(Arc::new(WebhookPersonalText));
        app.trigger(Arc::new(automation::IncomingWebhook))
            .action(Arc::new(automation::CallWebhook));
        app.openapi_tag(
            events_http::TAG,
            "Catalog of public event types delivered by outgoing webhooks",
        )
        .openapi_tag(
            outgoing::http::TAG,
            "Outgoing webhooks: endpoints, signed deliveries, test events and redelivery",
        )
        .openapi_tag(
            incoming::http::TAG,
            "Incoming webhooks: secret URLs that create or update records from external systems",
        )
        .routes(|_| {
            events_http::router()
                .merge(outgoing::http::router())
                .merge(incoming::http::router())
        })
        .worker(outgoing_webhook_worker)
        .worker(|env| {
            WorkerParts::default()
                .handler(incoming_webhooks_purge_handler(env.pool.clone()))
                .schedule(incoming_webhooks_purge_schedule())
        });
        for definition in NOTIFICATION_KINDS {
            app.notification_kind(*definition);
        }
        Ok(())
    }
}
