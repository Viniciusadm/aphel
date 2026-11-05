mod context_source;
mod dataset_seed;
mod email_engagement;
mod merges;
use aphel_sdk::imports::ImportTraits;
use aphel_sdk::party::{
    ContactPoint, ContactPointKind, ContactPoints, PartyTraits, address_fields, phone_field,
};
pub mod domain;
pub mod hierarchy;
pub(crate) mod http;
pub mod service;
pub mod store;

use aphel_contract_crm::{
    CrmPermissions, CrmRecordTypes, activities_relation, email_engagement_fields,
};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::application::fields::builtin::{
    field, record_field, timestamps, with_fields,
};
use aphel_sdk::platform::application::fields::{
    EntityDefinition, EntityTraits, LabelNormalization, RecordRelation,
};
use aphel_sdk::platform::application::public_events::{
    PreviousValues, PublicEventDefinition, changed, deleted, event,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::FieldType;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub use aphel_sdk::organizations::RESOURCE as COMPANY_RESOURCE;

pub use aphel_sdk::organizations::KEY;

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

pub(crate) fn company_capabilities() -> EntityTraits {
    EntityTraits::default()
        .with_trait(aphel_sdk::tagging::TAGGABLE)
        .with_trait("scoring.subject")
        .relatable()
        .importable()
        .import_policy(
            serde_json::from_str(include_str!("../locales/imports.json"))
                .expect("owner import metadata"),
        )
        .exportable()
        .personal_data()
        .notifies()
        .automatable()
        .validation_rules()
        .bulk_fields(COMPANY_BULK_FIELDS)
        .web_path("organizations")
        .localized_labels(crate::labels("Companies"))
        .with_contact_points(ContactPoints::new([
            ContactPoint::new("domain", ContactPointKind::Web),
            ContactPoint::new("email", ContactPointKind::Email),
        ]))
        .label_alias("domain", LabelNormalization::Domain)
        .free_text_fields(&["description"])
        .locked_fields(&["name"])
        .restore_order(0)
}

pub(crate) fn company_definition() -> Result<EntityDefinition, AppError> {
    let definition = EntityDefinition::new(
        RecordType::COMPANY,
        "organizations",
        Permission::COMPANIES_READ,
    )
    .capabilities(company_capabilities())
    .update_permission(Permission::COMPANIES_UPDATE)
    .title_field("name")
    .owner_column("owner_id")
    .soft_delete_column("deleted_at")
    .archive_column("archived_at")
    .hierarchy("parent_company_id")
    .system_field(
        field("name", crate::labels("Name"), FieldType::Text)?
            .searchable()
            .required(),
    )
    .system_field(field("domain", crate::labels("Domain"), FieldType::Text)?.searchable())
    .system_field(field("website", crate::labels("Website"), FieldType::Url)?)
    .system_field(field(
        "industry",
        crate::labels("Segment"),
        FieldType::Text,
    )?)
    .system_field(
        field("size_range", crate::labels("Size"), FieldType::Select)?
            .with_option("size_1_10", crate::labels("1–10"), None)
            .with_option("size_11_50", crate::labels("11–50"), None)
            .with_option("size_51_200", crate::labels("51–200"), None)
            .with_option("size_201_500", crate::labels("201–500"), None)
            .with_option("size_501_1000", crate::labels("501–1000"), None)
            .with_option("size_1001_plus", crate::labels("1001+"), None),
    )
    .system_field(phone_field("phone", crate::labels("Phone"))?)
    .system_field(field("email", crate::labels("Email"), FieldType::Email)?.searchable())
    .system_field(field("owner_id", crate::labels("Owner"), FieldType::User)?);
    let definition = with_fields(definition, address_fields()?)
        .system_field(field(
            "description",
            crate::labels("Description"),
            FieldType::LongText,
        )?)
        .system_field(field("source", crate::labels("Source"), FieldType::Text)?)
        .system_field(record_field(
            "parent_company_id",
            crate::labels("Parent company"),
            RecordType::COMPANY.as_str(),
        )?)
        .relation(RecordRelation::belongs_to(
            "parent_company",
            crate::labels("Parent company"),
            RecordType::COMPANY,
            "parent_company_id",
        ))
        .relation(RecordRelation::has_many(
            "subsidiaries",
            crate::labels("Branches"),
            RecordType::COMPANY,
            "parent_company_id",
        ))
        .relation(RecordRelation::has_many(
            "people",
            crate::labels("Contacts"),
            RecordType::CONTACT,
            "company_id",
        ))
        .relation(RecordRelation::has_many(
            "deals",
            crate::labels("Deals"),
            RecordType::DEAL,
            "company_id",
        ))
        .relation(activities_relation());
    let definition = with_fields(definition, email_engagement_fields()?);
    Ok(with_fields(definition, timestamps()?))
}

pub struct OrganizationsModule;

impl Module for OrganizationsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Organizations", APHEL_VERSION)
            .description("Companies, their hierarchy (company groups) and group totals")
            .owned_tables(&["organizations"])
            .migration(Migration::new(
                1,
                "create organizations",
                include_str!("../migrations/0001_create_organizations.sql"),
            ))
            .migration(Migration::new(
                2,
                "company detached parent",
                include_str!("../migrations/0002_company_detached_parent.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::maintenance::DatasetSeed>(std::sync::Arc::new(
            dataset_seed::OrganizationsSeed,
        ));
        app.contribute::<dyn aphel_sdk::merges::MergeOwner>(std::sync::Arc::new(merges::Owner));
        app.contribute::<dyn aphel_sdk::email::EmailEngagementProjection>(std::sync::Arc::new(
            email_engagement::Projection,
        ));
        app.contract(aphel_sdk::organizations::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::record_context::RecordListSource>(std::sync::Arc::new(
            context_source::ContextSource,
        ));
        app.contract(aphel_sdk::party::DESCRIPTOR);
        app.entity(
            company_definition()?
                .events("company")
                .handler(Arc::new(http::CompanyHandler)),
        )
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

pub(crate) fn labels(key: &str) -> aphel_sdk::platform::application::fields::LocalizedLabels {
    aphel_sdk::platform::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
