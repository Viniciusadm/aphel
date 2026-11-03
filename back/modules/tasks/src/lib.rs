mod dashboard;
mod followers;
use aphel_sdk::privacy::PrivacyTraits;
mod privacy;
mod timeline;
use aphel_sdk::assignment::AssignmentTraits;
mod calendar_source;
use aphel_contract_automation::AutomationBuilder;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod hooks;
pub mod http;
pub mod notifications;
pub(crate) mod operations;
pub(crate) mod plugin_bridge;
mod points;
pub(crate) mod recurrence;
pub mod section;
pub(crate) mod seed;
pub mod service;
pub mod store;
pub mod types;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use ::notifications::domain::{ALL_CHANNELS, NotificationKindDefinition};
use aphel_kernel::application::fields::builtin::{field, record_field, timestamps, with_fields};
use aphel_kernel::application::fields::{
    EntityDefinition, EntityTraits, RecordRelation, RelationPath, SystemField,
};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::defaults::{DefaultsProbe, DefaultsSeed};
use aphel_kernel::infrastructure::query::aggregate::LookupTitleSource;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const ACTIVITY_RESOURCE: PublicResource = PublicResource::new("tasks.task", "ActivityResponse");

pub const KEY: &str = "tasks";

const RECORD_PERMISSIONS: [Permission; 5] = [
    Permission::TASKS_READ,
    Permission::TASKS_CREATE,
    Permission::TASKS_UPDATE,
    Permission::TASKS_DELETE,
    Permission::TASKS_EXPORT,
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

pub(crate) fn activity_capabilities() -> EntityTraits {
    EntityTraits::default()
        .privacy_policy(aphel_sdk::privacy::EntityPrivacyPolicy {
            protected_fields: vec![
                "priority".to_owned(),
                "status".to_owned(),
                "activity_type_id".to_owned(),
            ],
        })
        .exportable()
        .workload(aphel_sdk::assignment::WorkloadMapping {
            state_field: "status".into(),
            active_states: vec!["open".into()],
        })
        .personal_data()
        .notifies()
        .automatable()
        .attached_to_parent()
        .validation_rules()
        .web_path("tasks")
        .localized_labels(crate::labels("Activities"))
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
            table: "task_links".to_owned(),
            source_type_column: None,
            source_id_column: "activity_id".to_owned(),
            target_type_column: Some("record_type".to_owned()),
            target_id_column: "record_id".to_owned(),
        })
}

fn activity_location_fields() -> Result<Vec<SystemField>, AppError> {
    Ok(vec![
        field(
            "location_latitude",
            crate::labels("Latitude"),
            FieldType::Number,
        )?
        .personal(),
        field(
            "location_longitude",
            crate::labels("Longitude"),
            FieldType::Number,
        )?
        .personal(),
        field(
            "location_accuracy_m",
            crate::labels("Location accuracy (m)"),
            FieldType::Number,
        )?,
        field(
            "location_captured_at",
            crate::labels("Location captured at"),
            FieldType::DateTime,
        )?,
    ])
}

pub(crate) fn activity_definition() -> Result<EntityDefinition, AppError> {
    let definition = EntityDefinition::new(RecordType::ACTIVITY, "tasks", Permission::TASKS_READ)
        .capabilities(activity_capabilities())
        .update_permission(Permission::TASKS_UPDATE)
        .title_field("title")
        .owner_column("owner_id")
        .soft_delete_column("deleted_at")
        .system_field(
            field("title", crate::labels("Title"), FieldType::Text)?
                .searchable()
                .required(),
        )
        .system_field(record_field(
            "activity_type_id",
            crate::labels("Type"),
            RecordType::ACTIVITY_TYPE.as_str(),
        )?)
        .system_field(
            field("status", crate::labels("Status"), FieldType::Select)?
                .with_option("open", crate::labels("Open"), Some("#1f6feb"))
                .with_option("done", crate::labels("Done"), Some("#1a7f37"))
                .with_option("canceled", crate::labels("Canceled"), Some("#6e7781")),
        )
        .system_field(
            field("priority", crate::labels("Priority"), FieldType::Select)?
                .with_option("low", crate::labels("Low"), Some("#6e7781"))
                .with_option("normal", crate::labels("Normal"), Some("#1f6feb"))
                .with_option("high", crate::labels("High"), Some("#bf8700"))
                .with_option("urgent", crate::labels("Urgent"), Some("#cf222e")),
        )
        .system_field(field(
            "due_at",
            crate::labels("Due date"),
            FieldType::DateTime,
        )?)
        .system_field(field(
            "completed_at",
            crate::labels("Completed at"),
            FieldType::DateTime,
        )?)
        .system_field(field("owner_id", crate::labels("Owner"), FieldType::User)?)
        .system_field(field(
            "created_by",
            crate::labels("Created by"),
            FieldType::User,
        )?)
        .system_field(field(
            "record_type",
            crate::labels("Record type"),
            FieldType::Text,
        )?)
        .system_field(field(
            "record_id",
            crate::labels("Record"),
            FieldType::Text,
        )?)
        .system_field(field(
            "duration_minutes",
            crate::labels("Duration (min)"),
            FieldType::Number,
        )?)
        .system_field(field(
            "description",
            crate::labels("Description"),
            FieldType::LongText,
        )?)
        .system_fields(activity_location_fields()?)
        .relation(activity_record("contact", "Contato", RecordType::CONTACT))
        .relation(activity_record("company", "Empresa", RecordType::COMPANY))
        .relation(activity_record("deal", "Oportunidade", RecordType::DEAL));
    Ok(with_fields(definition, timestamps()?))
}

pub struct TasksModule;

impl Module for TasksModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Tasks", APHEL_VERSION)
            .description("Tasks, logged tasks, activity types, calendar and activity inbox")
            .owned_tables(&["tasks", "task_links", "task_recurrences", "task_types"])
            .migration(Migration::new(
                1,
                "create activity types",
                include_str!("../migrations/0001_create_task_types.sql"),
            ))
            .migration(Migration::new(
                2,
                "create tasks",
                include_str!("../migrations/0002_create_activities.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::followers::FollowedEventSource>(std::sync::Arc::new(
            followers::TasksFollowedEvents,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::DashboardSource>(std::sync::Arc::new(
            dashboard::TaskSummarySource,
        ));
        app.contribute::<dyn aphel_sdk::dashboard::DashboardSource>(Arc::new(
            dashboard::TaskPendingSource,
        ));
        app.contribute::<dyn aphel_sdk::privacy::PrivacySource>(std::sync::Arc::new(
            privacy::TasksPrivacy,
        ));
        app.contribute::<dyn aphel_sdk::timeline::TimelineSource>(std::sync::Arc::new(
            timeline::TasksTimeline,
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "task_types",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "activity_type",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        app.entity(aphel_kernel::application::fields::builtin::with_fields(
            aphel_kernel::application::fields::EntityDefinition::new(
                aphel_kernel::application::shared::RecordType::from_static("tasks.type"),
                "task_types",
                aphel_kernel::domain::access::Permission::TASKS_READ,
            )
            .capabilities(
                aphel_kernel::application::fields::EntityTraits::default()
                    .with_trait("records.global"),
            )
            .title_field("name")
            .system_field(aphel_kernel::application::fields::builtin::field(
                "name",
                "Name",
                aphel_kernel::domain::fields::FieldType::Text,
            )?)
            .system_field(aphel_kernel::application::fields::builtin::field(
                "is_active",
                "Active",
                aphel_kernel::domain::fields::FieldType::Checkbox,
            )?),
            aphel_kernel::application::fields::builtin::timestamps()?,
        ));

        app.contract(aphel_sdk::calendar::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::calendar::CalendarSource>(Arc::new(
            calendar_source::TaskCalendar,
        ));
        app.contribute::<dyn ::notifications::DigestSource>(std::sync::Arc::new(points::OpenTasks));
        app.contribute::<dyn ::notifications::application::texts::NotificationTexts>(
            std::sync::Arc::new(crate::notifications::ActivityNotificationTexts),
        );
        app.contribute::<dyn ::notifications::application::NotificationSource>(
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
        app.contribute(Arc::new(DefaultsSeed(|connection| {
            Box::pin(seed::seed_task_types(connection))
        })));
        app.contribute(Arc::new(DefaultsProbe(seed::SEEDED_TABLE)));
        app.contribute(Arc::new(LookupTitleSource {
            record_type: "tasks.type",
            table: seed::SEEDED_TABLE,
            title_column: "name",
        }));
        app.contribute::<dyn aphel_sdk::points::ReferenceProvider>(std::sync::Arc::new(
            operations::ActivityTypeRefs,
        ));
        app.provide::<dyn aphel_contract_automation::application::TaskOperations>(
            std::sync::Arc::new(operations::AutomationTasks),
        );
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(90, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::task_types_section(pool.clone()))
            }),
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
            .openapi_tag(http::TAG, "Tasks and logged tasks")
            .openapi_tag(http::TYPES_TAG, "Activity types of the organization")
            .routes(|_| http::router());
        for permission in RECORD_PERMISSIONS {
            app.permission(PermissionDefinition::record(permission));
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for definition in NOTIFICATION_KINDS {
            app.contribute(std::sync::Arc::new(definition));
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

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
