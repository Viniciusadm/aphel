pub mod domain;
pub mod http;
pub mod service;
pub mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::fields::{
    LabelNormalization, RecordCapabilities, RecordTypeDefinition,
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
                RecordCapabilities::default()
                    .relatable()
                    .importable()
                    .exportable()
                    .validation_rules()
                    .bulk_fields(PRODUCT_BULK_FIELDS)
                    .web_path("products")
                    .plural_labels("Produtos", "Products")
                    .label_alias("code", LabelNormalization::Exact)
                    .restore_order(2),
            )
            .update_permission(Permission::PRODUCTS_UPDATE)
            .title_field("name")
            .soft_delete_column("deleted_at")
            .system_field(
                field("name", "Nome", FieldType::Text)?
                    .searchable()
                    .required(),
            )
            .system_field(field("code", "Código", FieldType::Text)?.searchable())
            .system_field(field("price", "Preço", FieldType::Currency)?)
            .system_field(field("currency", "Moeda", FieldType::Text)?)
            .system_field(field("unit", "Unidade", FieldType::Text)?)
            .system_field(
                field("recurrence", "Recorrência", FieldType::Select)?
                    .with_option("none", "Sem recorrência", None)
                    .with_option("monthly", "Mensal", None)
                    .with_option("quarterly", "Trimestral", None)
                    .with_option("yearly", "Anual", None),
            )
            .system_field(field("term_months", "Prazo (meses)", FieldType::Number)?)
            .system_field(field("is_active", "Ativo", FieldType::Checkbox)?)
            .system_field(field("description", "Descrição", FieldType::LongText)?)
            .system_field(field("created_by", "Criado por", FieldType::User)?)
            .system_field(field("created_at", "Criado em", FieldType::DateTime)?)
            .system_field(field("updated_at", "Atualizado em", FieldType::DateTime)?),
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
        app.record_type(product_definition()?.handler(Arc::new(http::ProductHandler)))
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
