mod references;
use aphel_sdk::imports::ImportTraits;
pub mod domain;
pub mod http;
pub mod service;
pub mod store;

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::fields::{EntityDefinition, EntityTraits, LabelNormalization};
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, changed, deleted, event,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use aphel_sdk::catalog_items::KEY;

pub(crate) const PRODUCTS_EXPORT: Permission =
    Permission::from_static(aphel_sdk::catalog_items::EXPORT);

pub use aphel_sdk::catalog_items::RESOURCE as PRODUCT_RESOURCE;

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

pub(crate) fn product_definition() -> Result<EntityDefinition, AppError> {
    Ok(
        EntityDefinition::new(RecordType::PRODUCT, "products", Permission::PRODUCTS_READ)
            .capabilities(
                EntityTraits::default()
                    .relatable()
                    .importable()
                    .import_policy(
                        serde_json::from_str(include_str!("../locales/imports.json"))
                            .expect("owner import metadata"),
                    )
                    .exportable()
                    .validation_rules()
                    .bulk_fields(PRODUCT_BULK_FIELDS)
                    .web_path("catalog-items")
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

pub struct CatalogItemsModule;

impl Module for CatalogItemsModule {
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
        app.provide::<dyn aphel_sdk::catalog_items::CatalogReferences>(Arc::new(
            references::ReferenceReader,
        ));
        app.contract(aphel_sdk::catalog_items::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::line_items::ItemCatalog>(std::sync::Arc::new(
            LineItemCatalog,
        ));
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

struct LineItemCatalog;
#[async_trait::async_trait]
impl aphel_sdk::line_items::ItemCatalog for LineItemCatalog {
    fn record_type(&self) -> aphel_kernel::application::shared::RecordType {
        aphel_kernel::application::shared::RecordType::from_static("catalog_items.item")
    }
    async fn item(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        id: uuid::Uuid,
    ) -> Result<Option<aphel_sdk::line_items::CatalogItem>, AppError> {
        Ok(self.items(ctx, &[id]).await?.pop())
    }
    async fn items(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<aphel_sdk::line_items::CatalogItem>, AppError> {
        use aphel_kernel::application::access::Authorizer;
        if aphel_sdk::access::require_organization_record_scope(
            ctx.scope,
            &Permission::PRODUCTS_READ,
        )
        .is_err()
            && ctx
                .scope
                .require(&aphel_kernel::domain::access::Permission::SETTINGS_MANAGE)
                .is_err()
        {
            return Ok(Vec::new());
        }
        let fields = ctx.field_set(ctx.scope, "catalog_items.item").await?;
        if [
            "name",
            "unit",
            "currency",
            "price",
            "recurrence",
            "term_months",
        ]
        .into_iter()
        .any(|key| fields.is_hidden(key))
        {
            return Ok(Vec::new());
        }
        Ok(store::ProductStore::of(ctx)
            .find_many(ids)
            .await?
            .into_iter()
            .map(|item| aphel_sdk::line_items::CatalogItem {
                code: (!fields.is_hidden("code"))
                    .then_some(item.data.code)
                    .flatten(),
                id: item.id,
                name: item.data.name,
                unit: item.data.unit,
                currency: item.data.currency,
                price: item.data.price,
                recurrence: item.data.recurrence,
                term_months: item.data.term_months,
                active: item.data.is_active,
            })
            .collect())
    }
}
