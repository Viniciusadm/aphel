mod followers;
mod messaging_channel;
pub mod oauth;
mod sender;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod capture;
pub mod compose;
pub mod domain;
pub mod dto;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod linking;
pub mod mailboxes;
pub(crate) mod model;
pub(crate) mod payload;
pub(crate) mod plugin_bridge;
mod points;
pub(crate) mod provider;
pub(crate) mod reading;
pub(crate) mod runtime;
pub mod service;
pub mod store;
pub(crate) mod sync;
pub mod templates;
pub mod tracking;

use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use aphel_kernel::modules::Migration;
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::application::files::{FileReferenceColumn, FileReferenceSource};
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use hooks::{InboundLinking, LinkingEmail, OutgoingEmail, OutgoingMessage};
pub use model::*;
pub use payload::*;
pub use provider::*;
pub use service::{MailboxSender, Sender};

pub const KEY: &str = "email";

const TAGS: &[(&str, &str)] = &[
    (
        "mailboxes",
        "Personal mailboxes (IMAP/SMTP and module providers), connection test and synchronization",
    ),
    (
        "email threads",
        "E-mail conversations linked to records, visibility, manual links and attachments",
    ),
    (
        "email settings",
        "Organization e-mail settings: relationship capture and blocklist",
    ),
    (
        "emails",
        "Sending e-mail from records through the user's mailbox, booking, cancellation and rendered previews",
    ),
    (
        "email templates",
        "Organization and personal e-mail templates with record variables",
    ),
    (
        "email tracking",
        "Open and click tracking settings, objections and the public tracking endpoints",
    ),
];

const RECORD_PERMISSIONS: [Permission; 2] = [Permission::EMAILS_READ, Permission::EMAILS_SEND];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EmailFileReferences;

impl FileReferenceSource for EmailFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![
            FileReferenceColumn::new("email_messages", "body_file_id"),
            FileReferenceColumn::new("email_attachments", "file_id"),
        ]
    }
}

pub struct EmailModule;

impl Module for EmailModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Email", APHEL_VERSION)
            .description(
                "Personal mailboxes, synchronization, conversations linked to records, sending, templates and tracking",
            )
            .depends_on(aphel_sdk::people::KEY, "*")
            .depends_on(aphel_sdk::organizations::KEY, "*")
            .enabled_by_default()
            .owned_tables(&["mailboxes", "mailbox_sync_failures"])
            .migration(Migration::new(
                1,
                "create mailboxes and conversations",
                include_str!("../migrations/0001_create_mailboxes.sql"),
            ))
            .migration(Migration::new(
                2,
                "create email settings",
                include_str!("../migrations/0002_create_email_settings.sql"),
            ))
            .migration(Migration::new(
                3,
                "create email templates and tracking",
                include_str!("../migrations/0003_create_email_templates.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::email::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::email::EmailSender>(std::sync::Arc::new(sender::Sender));
        app.contribute::<dyn aphel_sdk::followers::FollowedEventSource>(std::sync::Arc::new(
            followers::EmailFollowedEvents,
        ));
        app.contract(aphel_sdk::messaging::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::messaging::MessagingChannel>(Arc::new(
            messaging_channel::EmailChannel,
        ));
        app.contribute::<dyn aphel_kernel::modules::FileReferenceSource>(std::sync::Arc::new(
            points::EmailFiles,
        ));
        app.declare_hook(crate::hooks::OutgoingEmail, &[aphel_kernel::modules::HookUsage::Filter],
            "An e-mail before it is saved and sent; without recipients or subject the send fails with 422",
        );
        app.declare_hook(crate::hooks::InboundLinking, &[aphel_kernel::modules::HookUsage::Filter],
            "The records an incoming e-mail is linked to automatically by the mailbox synchronization",
        );
        app.contribute::<dyn FileReferenceSource>(Arc::new(EmailFileReferences));
        app.contribute::<dyn aphel_sdk::timeline::TimelineEventExtension>(std::sync::Arc::new(
            store::timeline::EmailTimeline,
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(150, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::templates::EmailTemplatesSection::new(
                    crate::store::sending::TemplateStore::new(pool.clone()),
                ))
            }),
        ));
        for (name, description) in TAGS {
            app.openapi_tag(name, description);
        }
        app.routes(|_| OpenApiRouter::new().merge(http::router()))
            .provide::<dyn Sender>(Arc::new(MailboxSender))
            .triggers(compose::automation::email_triggers())
            .action(Arc::new(compose::automation::SendOwnerEmail))
            .worker(runtime::sending_worker)
            .worker(runtime::sync_worker);
        for permission in RECORD_PERMISSIONS {
            app.permission(PermissionDefinition::record(permission))
                .grant_to(["admin", "manager", "member"]);
        }
        app.filter(
            crate::hooks::OutgoingEmail,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, message| Box::pin(plugin_bridge::outgoing(ctx, message)),
        );
        for hook in plugin_bridge::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
