pub(crate) mod automation;
pub mod domain;
pub(crate) mod hooks;
pub mod http;
pub mod notifications;
pub(crate) mod operations;
pub(crate) mod plugin_bridge;
pub(crate) mod recurrence;
pub mod section;
pub(crate) mod seed;
pub mod service;
pub mod store;
pub mod types;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::{field, record_field, timestamps, with_fields};
use aphel_kernel::application::fields::{
    RecordCapabilities, RecordRelation, RecordTypeDefinition, RelationPath, SystemField,
};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::domain::notifications::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::crm::{CrmDefaultsProbe, CrmDefaultsSeed};
use aphel_kernel::infrastructure::query::aggregate::LookupTitleSource;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const ACTIVITY_RESOURCE: PublicResource = PublicResource::new("activity", "ActivityResponse");

pub const KEY: &str = "activities";

const RECORD_PERMISSIONS: [Permission; 5] = [
    Permission::ACTIVITIES_READ,
    Permission::ACTIVITIES_CREATE,
    Permission::ACTIVITIES_UPDATE,
    Permission::ACTIVITIES_DELETE,
    Permission::ACTIVITIES_EXPORT,
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "activity.created",
        ACTIVITY_RESOURCE,
        "An activity was created",
    ),
    changed(
        "activity.updated",
        ACTIVITY_RESOURCE,
        "Fields of an activity changed",
        PreviousValues::Changes,
    ),
    event(
        "activity.completed",
        ACTIVITY_RESOURCE,
        "An activity was completed",
    ),
    event(
        "activity.reopened",
        ACTIVITY_RESOURCE,
        "A completed activity was reopened",
    ),
    deleted(
        "activity.deleted",
        ACTIVITY_RESOURCE,
        "An activity was deleted",
    ),
];

const NOTIFICATION_KINDS: [NotificationKindDefinition; 2] = [
    NotificationKindDefinition {
        kind: crate::notifications::TASK_ASSIGNED_NOTIFICATION,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: true,
    },
    NotificationKindDefinition {
        kind: crate::notifications::TASK_DUE_NOTIFICATION,
        channels: ALL_CHANNELS,
        email_default: true,
        push_default: true,
    },
];

pub(crate) fn activity_capabilities() -> RecordCapabilities {
    RecordCapabilities::default()
        .exportable()
        .personal_data()
        .notifies()
        .automatable()
        .attached_to_parent()
        .validation_rules()
        .web_path("tasks")
        .plural_labels("Atividades", "Activities")
        .free_text_fields(&["description"])
        .locked_fields(&[
            "title",
            "status",
            "activity_type_id",
            "record_type",
            "record_id",
        ])
        .restore_order(5)
}

fn activity_record(key: &str, label: &str, target: RecordType) -> RecordRelation {
    RecordRelation::new(key, label, target)
        .path(RelationPath::Polymorphic {
            type_column: "record_type".to_owned(),
            id_column: "record_id".to_owned(),
        })
        .path(RelationPath::Linked {
            table: "activity_links".to_owned(),
            source_type_column: None,
            source_id_column: "activity_id".to_owned(),
            target_type_column: Some("record_type".to_owned()),
            target_id_column: "record_id".to_owned(),
        })
}

fn activity_location_fields() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field("location_latitude", "Latitude", FieldType::Number)?.personal(),
        field("location_longitude", "Longitude", FieldType::Number)?.personal(),
        field(
            "location_accuracy_m",
            "Precisão da localização (m)",
            FieldType::Number,
        )?,
        field(
            "location_captured_at",
            "Localização capturada em",
            FieldType::DateTime,
        )?,
    ])
}

pub(crate) fn activity_definition() -> Result<RecordTypeDefinition, AppError> {
    let definition = RecordTypeDefinition::new(
        RecordType::ACTIVITY,
        "activities",
        Permission::ACTIVITIES_READ,
    )
    .capabilities(activity_capabilities())
    .update_permission(Permission::ACTIVITIES_UPDATE)
    .title_field("title")
    .owner_column("owner_id")
    .soft_delete_column("deleted_at")
    .system_field(
        field("title", "Título", FieldType::Text)?
            .searchable()
            .required(),
    )
    .system_field(record_field(
        "activity_type_id",
        "Tipo",
        RecordType::ACTIVITY_TYPE.as_str(),
    )?)
    .system_field(
        field("status", "Situação", FieldType::Select)?
            .with_option("open", "Aberta", Some("#1f6feb"))
            .with_option("done", "Concluída", Some("#1a7f37"))
            .with_option("canceled", "Cancelada", Some("#6e7781")),
    )
    .system_field(
        field("priority", "Prioridade", FieldType::Select)?
            .with_option("low", "Baixa", Some("#6e7781"))
            .with_option("normal", "Normal", Some("#1f6feb"))
            .with_option("high", "Alta", Some("#bf8700"))
            .with_option("urgent", "Urgente", Some("#cf222e")),
    )
    .system_field(field("due_at", "Prazo", FieldType::DateTime)?)
    .system_field(field("completed_at", "Concluída em", FieldType::DateTime)?)
    .system_field(field("owner_id", "Responsável", FieldType::User)?)
    .system_field(field("created_by", "Criada por", FieldType::User)?)
    .system_field(field("record_type", "Tipo do registro", FieldType::Text)?)
    .system_field(field("record_id", "Registro", FieldType::Text)?)
    .system_field(field(
        "duration_minutes",
        "Duração (min)",
        FieldType::Number,
    )?)
    .system_field(field("description", "Descrição", FieldType::LongText)?)
    .system_fields(activity_location_fields()?)
    .relation(activity_record("contact", "Contato", RecordType::CONTACT))
    .relation(activity_record("company", "Empresa", RecordType::COMPANY))
    .relation(activity_record("deal", "Oportunidade", RecordType::DEAL));
    Ok(with_fields(definition, timestamps()?))
}

pub struct ActivitiesModule;

impl Module for ActivitiesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Activities", APHEL_VERSION)
            .description("Tasks, logged activities, activity types, calendar and activity inbox")
            .required()
            .owned_tables(&[
                "activities",
                "activity_links",
                "activity_recurrences",
                "activity_types",
            ])
            .migration(Migration::new(
                1,
                "create activity types",
                include_str!("../migrations/0001_create_activity_types.sql"),
            ))
            .migration(Migration::new(
                2,
                "create activities",
                include_str!("../migrations/0002_create_activities.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::application::notifications::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::ActivityNotificationTexts),
        );
        app.contribute::<dyn aphel_kernel::application::notifications::NotificationSource>(
            std::sync::Arc::new(notifications::TaskAssignedNotifications),
        );
        app.declare_hook(
            crate::hooks::ActivityCompleting,
            &[aphel_kernel::modules::HookUsage::Veto],
            "Before an activity is completed; an error blocks it",
        );
        app.on(
            crate::hooks::ActivityCompleting,
            aphel_kernel::modules::PLUGIN_HOOK_PRIORITY,
            |ctx, completing| Box::pin(plugin_bridge::completing(ctx, completing)),
        );
        app.contribute(Arc::new(CrmDefaultsSeed(|connection| {
            Box::pin(seed::seed_activity_types(connection))
        })));
        app.contribute(Arc::new(CrmDefaultsProbe(seed::SEEDED_TABLE)));
        app.contribute(Arc::new(LookupTitleSource {
            record_type: "activity_type",
            table: seed::SEEDED_TABLE,
            title_column: "name",
        }));
        app.provide::<dyn aphel_kernel::application::automations::ActivityTypeOperations>(
            std::sync::Arc::new(operations::AutomationActivityTypes),
        );
        app.provide::<dyn aphel_kernel::application::automations::TaskOperations>(
            std::sync::Arc::new(operations::AutomationTasks),
        );
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                90,
                |pool, registry| {
                    let _ = registry;
                    std::sync::Arc::new(crate::section::activity_types_section(pool.clone()))
                },
            ),
        ));
        app.filter(
            aphel_kernel::records::RecordSerializing,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, records| Box::pin(crate::hooks::mask_location(ctx, records)),
        );
        app.on(
            aphel_kernel::records::RecordPurged,
            aphel_kernel::modules::DEFAULT_PRIORITY,
            |ctx, removed| Box::pin(crate::hooks::unlink_purged_record(ctx, removed)),
        );
        app.trigger(Arc::new(crate::automation::ActivityCompleted))
            .action(Arc::new(crate::automation::CreateTask));
        app.entity(activity_definition()?.handler(Arc::new(http::ActivityHandler)))
            .openapi_tag(http::TAG, "Tasks and logged activities")
            .openapi_tag(http::TYPES_TAG, "Activity types of the organization")
            .openapi_tag(
                http::CALENDAR_TAG,
                "Calendar of activities with due dates and meetings, by month, week or day",
            )
            .routes(|_| http::router());
        for permission in RECORD_PERMISSIONS {
            app.permission(PermissionDefinition::record(permission));
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for definition in NOTIFICATION_KINDS {
            app.notification_kind(definition);
        }
        for hook in plugin_bridge::hooks() {
            app.contribute::<aphel_kernel::modules::PluginHook>(hook);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
