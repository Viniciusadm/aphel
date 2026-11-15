mod context_source;
mod dashboard;
mod followers;
mod merge_references;
mod privacy;
mod timeline;
mod writer;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod notifications;
pub(crate) mod operations;
pub(crate) mod public;
pub(crate) mod search;
pub mod service;
pub mod store;

use aphel_contract_crm::CrmPermissions;
use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_sdk::platform::application::public_events::{
    PublicEventDefinition, PublicResource, deleted, event,
};
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const NOTE_RESOURCE: PublicResource = PublicResource::new("notes.note", "NoteResponse");

pub const KEY: &str = "notes";

const RECORD_PERMISSIONS: [Permission; 4] = [
    Permission::NOTES_READ,
    Permission::NOTES_CREATE,
    Permission::NOTES_UPDATE,
    Permission::NOTES_DELETE,
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event("note.created", NOTE_RESOURCE, "A note was created"),
    event("note.updated", NOTE_RESOURCE, "A note was edited"),
    deleted("note.deleted", NOTE_RESOURCE, "A note was deleted"),
];

pub struct NotesModule;

impl Module for NotesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Notes", APHEL_VERSION)
            .description("Notes attached to records, with mentions and private notes")
            .optional("timeline")
            .optional("search")
            .optional("followers")
            .optional("dashboard")
            .optional("privacy")
            .optional("automations")
            .optional("duplicates")
            .owned_tables(&["notes"])
            .migration(Migration::new(
                1,
                "create notes",
                include_str!("../migrations/0001_create_notes.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::merges::MergeReferences>(std::sync::Arc::new(
            merge_references::References,
        ));
        app.provide::<dyn aphel_sdk::notes::NoteWriter>(std::sync::Arc::new(writer::Writer));
        app.contribute::<dyn aphel_sdk::record_context::RelatedRecordSource>(std::sync::Arc::new(
            context_source::ContextSource,
        ));
        app.contribute::<dyn aphel_sdk::followers::FollowedEventSource>(std::sync::Arc::new(
            followers::NotesFollowedEvents,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::DashboardEventScope>(std::sync::Arc::new(
            dashboard::NoteDashboardEvents,
        ));
        app.contribute::<dyn aphel_sdk::privacy::PrivacySource>(std::sync::Arc::new(
            privacy::NotesPrivacy,
        ));
        app.contribute::<dyn aphel_sdk::timeline::TimelineSource>(std::sync::Arc::new(
            timeline::NotesTimeline,
        ));
        app.contribute::<dyn aphel_sdk::notifications::NotificationTexts>(std::sync::Arc::new(
            crate::notifications::NoteNotificationTexts,
        ));
        app.provide::<dyn aphel_contract_automation::application::NoteOperations>(
            std::sync::Arc::new(operations::AutomationNotes),
        );
        app.contribute::<dyn aphel_sdk::search::SearchContentSource>(std::sync::Arc::new(
            search::NoteSearchContent,
        ));
        app.contribute::<dyn aphel_sdk::platform::application::public_events::PublicResourceLoader>(
            std::sync::Arc::new(public::NotePublicResource),
        );
        app.contribute::<dyn aphel_sdk::notifications::NotificationSource>(std::sync::Arc::new(
            notifications::NoteMentionNotifications,
        ));
        app.on(
            aphel_kernel::records::RecordPurged,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(crate::hooks::purge_record_notes(ctx, removed)),
        );
        exposed_operations::register(app);
        app.trigger(std::sync::Arc::new(crate::automation::NoteCreated))
            .action(std::sync::Arc::new(crate::automation::CreateNote));
        app.openapi_tag(http::TAG, "Notes attached to records")
            .routes(|_| http::router());
        for permission in RECORD_PERMISSIONS {
            app.permission(PermissionDefinition::record(permission))
                .grant_to(["admin", "manager", "member"]);
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        app.contribute(std::sync::Arc::new(NotificationKindDefinition {
            kind: crate::notifications::NOTE_MENTION_NOTIFICATION,
            channels: ALL_CHANNELS,
            email_default: true,
            push_default: true,
        }));
        Ok(())
    }
}
