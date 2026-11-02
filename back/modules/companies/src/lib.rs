pub mod domain;
pub mod hierarchy;
pub(crate) mod http;
pub mod service;
pub mod store;

use aphel_contract_crm::{
    CrmPermissions, CrmRecordTypes, activities_relation, address_fields, email_engagement_fields,
    phone_field,
};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::{field, record_field, timestamps, with_fields};
use aphel_kernel::application::fields::{
    LabelNormalization, RecordCapabilities, RecordRelation, RecordTypeDefinition,
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

pub const COMPANY_RESOURCE: PublicResource = PublicResource::new("company", "CompanyResponse");

pub const KEY: &str = "companies";

const COMPANY_BULK_FIELDS: &[&str] = &[
    "name",
    "domain",
    "website",
    "industry",
    "size_range",
    "phone",
    "email",
    "owner_id",
    "description",
    "source",
];

const RECORD_PERMISSIONS: [Permission; 6] = [
    Permission::COMPANIES_READ,
    Permission::COMPANIES_CREATE,
    Permission::COMPANIES_UPDATE,
    Permission::COMPANIES_DELETE,
    Permission::COMPANIES_EXPORT,
    Permission::COMPANIES_ARCHIVE,
];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event("company.created", COMPANY_RESOURCE, "A company was created"),
    changed(
        "company.updated",
        COMPANY_RESOURCE,
        "Fields of a company changed",
        PreviousValues::Changes,
    ),
    deleted(
        "company.deleted",
        COMPANY_RESOURCE,
        "A company was moved to the trash",
    ),
    event(
        "company.archived",
        COMPANY_RESOURCE,
        "A company was archived",
    ),
    event(
        "company.unarchived",
        COMPANY_RESOURCE,
        "A company was unarchived",
    ),
    event(
        "company.merged",
        COMPANY_RESOURCE,
        "Another company was merged into this one",
    ),
];

pub(crate) fn company_capabilities() -> RecordCapabilities {
    RecordCapabilities::default()
        .relatable()
        .importable()
        .exportable()
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(COMPANY_BULK_FIELDS)
        .web_path("companies")
        .plural_labels("Empresas", "Companies")
        .contact_columns(&["domain", "email"])
        .label_alias("domain", LabelNormalization::Domain)
        .free_text_fields(&["description"])
        .locked_fields(&["name"])
        .restore_order(0)
}

pub(crate) fn company_definition() -> Result<RecordTypeDefinition, AppError> {
    let definition =
        RecordTypeDefinition::new(RecordType::COMPANY, "companies", Permission::COMPANIES_READ)
            .capabilities(company_capabilities())
            .update_permission(Permission::COMPANIES_UPDATE)
            .title_field("name")
            .owner_column("owner_id")
            .soft_delete_column("deleted_at")
            .archive_column("archived_at")
            .hierarchy("parent_company_id")
            .system_field(
                field("name", "Nome", FieldType::Text)?
                    .searchable()
                    .required(),
            )
            .system_field(field("domain", "Domínio", FieldType::Text)?.searchable())
            .system_field(field("website", "Site", FieldType::Url)?)
            .system_field(field("industry", "Segmento", FieldType::Text)?)
            .system_field(
                field("size_range", "Porte", FieldType::Select)?
                    .with_option("size_1_10", "1–10", None)
                    .with_option("size_11_50", "11–50", None)
                    .with_option("size_51_200", "51–200", None)
                    .with_option("size_201_500", "201–500", None)
                    .with_option("size_501_1000", "501–1000", None)
                    .with_option("size_1001_plus", "1001+", None),
            )
            .system_field(phone_field("phone", "Telefone")?)
            .system_field(field("email", "E-mail", FieldType::Email)?.searchable())
            .system_field(field("owner_id", "Responsável", FieldType::User)?);
    let definition = with_fields(definition, address_fields()?)
        .system_field(field("description", "Descrição", FieldType::LongText)?)
        .system_field(field("source", "Origem", FieldType::Text)?)
        .system_field(record_field(
            "parent_company_id",
            "Empresa matriz",
            RecordType::COMPANY.as_str(),
        )?)
        .relation(RecordRelation::belongs_to(
            "parent_company",
            "Empresa matriz",
            RecordType::COMPANY,
            "parent_company_id",
        ))
        .relation(RecordRelation::has_many(
            "subsidiaries",
            "Filiais",
            RecordType::COMPANY,
            "parent_company_id",
        ))
        .relation(RecordRelation::has_many(
            "contacts",
            "Contatos",
            RecordType::CONTACT,
            "company_id",
        ))
        .relation(RecordRelation::has_many(
            "deals",
            "Oportunidades",
            RecordType::DEAL,
            "company_id",
        ))
        .relation(activities_relation());
    let definition = with_fields(definition, email_engagement_fields()?);
    Ok(with_fields(definition, timestamps()?))
}

pub struct CompaniesModule;

impl Module for CompaniesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Companies", APHEL_VERSION)
            .description("Companies, their hierarchy (company groups) and group totals")
            .required()
            .owned_tables(&["companies"])
            .migration(Migration::new(
                1,
                "create companies",
                include_str!("../migrations/0001_create_companies.sql"),
            ))
            .migration(Migration::new(
                2,
                "company detached parent",
                include_str!("../migrations/0002_company_detached_parent.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.entity(company_definition()?.handler(Arc::new(http::CompanyHandler)))
            .openapi_tag(http::TAG, "Companies")
            .routes(|_| http::router().merge(hierarchy::http::router()))
            .consumer(|env| {
                hierarchy::restore::HierarchyRestoreConsumer::shared(
                    env.pool.clone(),
                    env.record_types.clone(),
                )
            });
        for permission in RECORD_PERMISSIONS {
            let definition = PermissionDefinition::record(permission.clone());
            if permission == Permission::COMPANIES_ARCHIVE {
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
