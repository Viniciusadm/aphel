pub mod domain;
pub mod http;
pub mod service;
pub mod store;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::fields::{EntityTraits, LabelNormalization, RecordTypeDefinition};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "products";

pub(crate) const PRODUCTS_EXPORT: Permission = Permission::from_static("products.export");

pub const PRODUCT_RESOURCE: PublicResource = PublicResource::new("product", "ProductResponse");

const PRODUCT_BULK_FIELDS: &[&str] = &["price", "unit", "recurrence", "term_months", "description"];

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        "product.created",
        PRODUCT_RESOURCE,
        "A product was added to the catalog",
    ),
    changed(
        "product.updated",
        PRODUCT_RESOURCE,
        "Fields of a product changed",
        PreviousValues::Changes,
    ),
    deleted(
        "product.deleted",
        PRODUCT_RESOURCE,
        "A product was removed from the catalog",
    ),
];

pub(crate) fn product_definition() -> Result<RecordTypeDefinition, AppError> {
    Ok(
        RecordTypeDefinition::new(RecordType::PRODUCT, "products", Permission::PRODUCTS_READ)
            .capabilities(
                EntityTraits::default()
                    .relatable()
                    .importable()
                    .exportable()
                    .validation_rules()
                    .bulk_fields(PRODUCT_BULK_FIELDS)
                    .web_path("products")
                    .localized_labels(crate::labels("Products"))
                    .label_alias("code", LabelNormalization::Exact)
                    .restore_order(2),
            )
            .update_permission(Permission::PRODUCTS_UPDATE)
            .title_field("name")
            .soft_delete_column("deleted_at")
            .system_field(
                field("name", crate::labels("Name"), FieldType::Text)?
                    .searchable()
                    .required(),
            )
            .system_field(field("code", crate::labels("Code"), FieldType::Text)?.searchable())
            .system_field(field("price", crate::labels("Price"), FieldType::Currency)?)
            .system_field(field(
                "currency",
                crate::labels("Currency"),
                FieldType::Text,
            )?)
            .system_field(field("unit", crate::labels("Unit"), FieldType::Text)?)
            .system_field(
                field("recurrence", crate::labels("Recurrence"), FieldType::Select)?
                    .with_option("none", crate::labels("No recurrence"), None)
                    .with_option("monthly", crate::labels("Monthly"), None)
                    .with_option("quarterly", crate::labels("Quarterly"), None)
                    .with_option("yearly", crate::labels("Yearly"), None),
            )
            .system_field(field(
                "term_months",
                crate::labels("Term (months)"),
                FieldType::Number,
            )?)
            .system_field(field(
                "is_active",
                crate::labels("Active"),
                FieldType::Checkbox,
            )?)
            .system_field(field(
                "description",
                crate::labels("Description"),
                FieldType::LongText,
            )?)
            .system_field(field(
                "created_by",
                crate::labels("Created by"),
                FieldType::User,
            )?)
            .system_field(field(
                "created_at",
                crate::labels("Created at"),
                FieldType::DateTime,
            )?)
            .system_field(field(
                "updated_at",
                crate::labels("Updated at"),
                FieldType::DateTime,
            )?),
    )
}

pub struct ProductsModule;

impl Module for ProductsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Products", APHEL_VERSION)
            .description("Catalog of products and services")
            .owned_tables(&["products"])
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create products",
                include_str!("../migrations/0001_create_products.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.entity(product_definition()?.handler(Arc::new(http::ProductHandler)))
            .openapi_tag(http::TAG, "Catalog of products and services")
            .routes(|_| http::router());
        app.permission(PermissionDefinition::record(Permission::PRODUCTS_READ))
            .grant_organization_wide_to(["admin", "manager", "member"]);
        app.permission(PermissionDefinition::record(PRODUCTS_EXPORT))
            .grant_organization_wide_to(["admin", "manager"]);
        for permission in [
            Permission::PRODUCTS_CREATE,
            Permission::PRODUCTS_UPDATE,
            Permission::PRODUCTS_DELETE,
        ] {
            app.permission(PermissionDefinition::record(permission))
                .grant_to(["admin"]);
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
