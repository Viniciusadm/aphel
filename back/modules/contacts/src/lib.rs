pub mod domain;
pub mod http;
pub mod service;
pub mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::{
    activities_relation, address_fields, email_engagement_fields, field, phone_field, record_field,
    timestamps, with_fields,
};
use aphel_kernel::application::fields::{
    LabelNormalization, RecordCapabilities, RecordRelation, RecordTypeDefinition, SystemField,
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

pub const CONTACT_RESOURCE: PublicResource = PublicResource::new("contact", "ContactResponse");

pub const KEY: &str = "contacts";

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

pub(crate) fn contact_capabilities() -> RecordCapabilities {
    RecordCapabilities::default()
        .relatable()
        .importable()
        .exportable()
        .sequences()
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(CONTACT_BULK_FIELDS)
        .web_path("contacts")
        .plural_labels("Contatos", "Contacts")
        .contact_columns(&["email", "mobile", "phone"])
        .label_alias("email", LabelNormalization::Lowercase)
        .related_record("company_id", RecordType::COMPANY)
        .free_text_fields(&["notes"])
        .locked_fields(&["name", "first_name", "last_name"])
        .restore_order(1)
}

pub(crate) fn contact_definition() -> Result<RecordTypeDefinition, AppError> {
    let definition =
        RecordTypeDefinition::new(RecordType::CONTACT, "contacts", Permission::CONTACTS_READ)
            .capabilities(contact_capabilities())
            .update_permission(Permission::CONTACTS_UPDATE)
            .title_field("name")
            .owner_column("owner_id")
            .soft_delete_column("deleted_at")
            .archive_column("archived_at")
            .system_field(
                SystemField::new("name", "Nome completo", FieldType::Text, "full_name")?
                    .searchable()
                    .personal(),
            )
            .system_field(
                field("first_name", "Nome", FieldType::Text)?
                    .searchable()
                    .required()
                    .personal(),
            )
            .system_field(
                field("last_name", "Sobrenome", FieldType::Text)?
                    .searchable()
                    .personal(),
            )
            .system_field(
                field("email", "E-mail", FieldType::Email)?
                    .searchable()
                    .personal(),
            )
            .system_field(phone_field("phone", "Telefone")?.personal())
            .system_field(phone_field("mobile", "Celular")?.personal())
            .system_field(field("job_title", "Cargo", FieldType::Text)?)
            .system_field(record_field(
                "company_id",
                "Empresa",
                RecordType::COMPANY.as_str(),
            )?)
            .system_field(field("owner_id", "Responsável", FieldType::User)?);
    let personal_address = address_fields()?
        .into_iter()
        .map(SystemField::personal)
        .collect();
    let definition = with_fields(definition, personal_address)
        .system_field(field("source", "Origem", FieldType::Text)?)
        .system_field(field("notes", "Observações", FieldType::LongText)?)
        .relation(RecordRelation::belongs_to(
            "company",
            "Empresa",
            RecordType::COMPANY,
            "company_id",
        ))
        .relation(RecordRelation::has_many(
            "deals",
            "Oportunidades",
            RecordType::DEAL,
            "contact_id",
        ))
        .relation(activities_relation());
    let definition = with_fields(definition, email_engagement_fields()?);
    Ok(with_fields(definition, timestamps()?))
}

pub struct ContactsModule;

impl Module for ContactsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Contacts", APHEL_VERSION)
            .description("Contacts and their links to companies")
            .depends_on(companies::KEY, "*")
            .required()
            .owned_tables(&["contacts", "contact_companies"])
            .migration(Migration::new(
                1,
                "create contacts",
                include_str!("../migrations/0001_create_contacts.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.record_type(contact_definition()?.handler(Arc::new(http::ContactHandler)))
            .openapi_tag(http::TAG, "Contacts and their companies")
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
