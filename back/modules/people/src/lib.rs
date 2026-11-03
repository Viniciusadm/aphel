mod timeline;
use aphel_sdk::party::{
    ContactPoint, ContactPointKind, ContactPoints, PartyTraits, address_fields, phone_field,
};
pub mod domain;
pub mod http;
pub mod service;
pub mod store;

use aphel_contract_crm::traits;
use aphel_contract_crm::{
    CrmPermissions, CrmRecordTypes, activities_relation, email_engagement_fields,
};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::{field, record_field, timestamps, with_fields};
use aphel_kernel::application::fields::{
    EntityDefinition, EntityTraits, LabelNormalization, RecordRelation, SystemField,
};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const CONTACT_RESOURCE: PublicResource =
    PublicResource::new("people.person", "ContactResponse");

pub const KEY: &str = "people";

const CONTACT_BULK_FIELDS: &[&str] = &[
    "first_name",
    "last_name",
    "email",
    "phone",
    "mobile",
    "job_title",
    "company_id",
    "owner_id",
    "source",
    "notes",
];

const RECORD_PERMISSIONS: [Permission; 6] = [
    Permission::CONTACTS_READ,
    Permission::CONTACTS_CREATE,
    Permission::CONTACTS_UPDATE,
    Permission::CONTACTS_DELETE,
    Permission::CONTACTS_EXPORT,
    Permission::CONTACTS_ARCHIVE,
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event("contact.created", CONTACT_RESOURCE, "A contact was created"),
    changed(
        "contact.updated",
        CONTACT_RESOURCE,
        "Fields of a contact changed",
        PreviousValues::Changes,
    ),
    deleted(
        "contact.deleted",
        CONTACT_RESOURCE,
        "A contact was moved to the trash",
    ),
    event(
        "contact.archived",
        CONTACT_RESOURCE,
        "A contact was archived",
    ),
    event(
        "contact.unarchived",
        CONTACT_RESOURCE,
        "A contact was unarchived",
    ),
    event(
        "contact.merged",
        CONTACT_RESOURCE,
        "Another contact was merged into this one",
    ),
];

pub(crate) fn contact_capabilities() -> EntityTraits {
    EntityTraits::default()
        .with_trait("scoring.subject")
        .relatable()
        .importable()
        .exportable()
        .with_trait(traits::SEQUENCEABLE)
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(CONTACT_BULK_FIELDS)
        .web_path("people")
        .localized_labels(crate::labels("Contacts"))
        .with_contact_points(ContactPoints::new([
            ContactPoint::new("email", ContactPointKind::Email),
            ContactPoint::new("mobile", ContactPointKind::Phone),
            ContactPoint::new("phone", ContactPointKind::Phone),
        ]))
        .label_alias("email", LabelNormalization::Lowercase)
        .related_record("company_id", RecordType::COMPANY)
        .free_text_fields(&["notes"])
        .locked_fields(&["name", "first_name", "last_name"])
        .restore_order(1)
}

pub(crate) fn contact_definition() -> Result<EntityDefinition, AppError> {
    let definition =
        EntityDefinition::new(RecordType::CONTACT, "people", Permission::CONTACTS_READ)
            .capabilities(contact_capabilities())
            .update_permission(Permission::CONTACTS_UPDATE)
            .title_field("name")
            .owner_column("owner_id")
            .soft_delete_column("deleted_at")
            .archive_column("archived_at")
            .system_field(
                SystemField::new(
                    "name",
                    crate::labels("Full name"),
                    FieldType::Text,
                    "full_name",
                )?
                .searchable()
                .personal(),
            )
            .system_field(
                field("first_name", crate::labels("Name"), FieldType::Text)?
                    .searchable()
                    .required()
                    .personal(),
            )
            .system_field(
                field("last_name", crate::labels("Last name"), FieldType::Text)?
                    .searchable()
                    .personal(),
            )
            .system_field(
                field("email", crate::labels("Email"), FieldType::Email)?
                    .searchable()
                    .personal(),
            )
            .system_field(phone_field("phone", crate::labels("Phone"))?.personal())
            .system_field(phone_field("mobile", crate::labels("Mobile"))?.personal())
            .system_field(field(
                "job_title",
                crate::labels("Job title"),
                FieldType::Text,
            )?)
            .system_field(record_field(
                "company_id",
                crate::labels("Company"),
                RecordType::COMPANY.as_str(),
            )?)
            .system_field(field("owner_id", crate::labels("Owner"), FieldType::User)?);
    let personal_address = address_fields()?
        .into_iter()
        .map(SystemField::personal)
        .collect();
    let definition = with_fields(definition, personal_address)
        .system_field(field("source", crate::labels("Source"), FieldType::Text)?)
        .system_field(field("notes", crate::labels("Notes"), FieldType::LongText)?)
        .relation(RecordRelation::belongs_to(
            "company",
            crate::labels("Company"),
            RecordType::COMPANY,
            "company_id",
        ))
        .relation(RecordRelation::has_many(
            "deals",
            crate::labels("Deals"),
            RecordType::DEAL,
            "contact_id",
        ))
        .relation(activities_relation());
    let definition = with_fields(definition, email_engagement_fields()?);
    Ok(with_fields(definition, timestamps()?))
}

pub struct PeopleModule;

impl Module for PeopleModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "People", APHEL_VERSION)
            .description("Contacts and their links to organizations")
            .depends_on(organizations::KEY, "*")
            .required()
            .owned_tables(&["people", "people_organizations"])
            .migration(Migration::new(
                1,
                "create people",
                include_str!("../migrations/0001_create_people.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::timeline::TimelineEventScope>(std::sync::Arc::new(
            timeline::RelatedEvents,
        ));
        app.contract(aphel_sdk::party::DESCRIPTOR);
        app.entity(
            contact_definition()?
                .events("contact")
                .generic_mutations(Permission::CONTACTS_CREATE, Permission::CONTACTS_DELETE)
                .handler(Arc::new(
                    aphel_kernel::records::GenericRecordHandler::new(RecordType::CONTACT)
                        .with_behavior(Arc::new(http::ContactBehavior)),
                )),
        )
        .openapi_tag(http::TAG, "Contacts and their organizations")
        .routes(|_| http::router());
        for permission in RECORD_PERMISSIONS {
            let definition = PermissionDefinition::record(permission.clone());
            if permission == Permission::CONTACTS_ARCHIVE {
                app.permission(definition)
                    .grant_to(["admin", "manager", "member"]);
            } else {
                app.permission(definition);
            }
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
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
