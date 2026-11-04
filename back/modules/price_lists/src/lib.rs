pub(crate) mod bundles;
pub mod domain;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod public;
pub(crate) mod section;
pub mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use hooks::{
    BundleExpander, CatalogBundleExpander, CatalogPriceListResolver, DiscountLimits,
    PriceListResolver, RoleDiscountLimits, bundle_expander, discount_limits, price_list_resolver,
};

pub const KEY: &str = "price_lists";

pub(crate) const PRICE_LIST_RESOURCE: PublicResource =
    PublicResource::new("price_lists.price_list", "PriceListResponse");

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        domain::PRICE_LIST_CREATED,
        PRICE_LIST_RESOURCE,
        "A price list was created",
    ),
    changed(
        domain::PRICE_LIST_UPDATED,
        PRICE_LIST_RESOURCE,
        "Fields of a price list changed",
        PreviousValues::Changes,
    ),
    deleted(
        domain::PRICE_LIST_DELETED,
        PRICE_LIST_RESOURCE,
        "A price list was removed",
    ),
    event(
        domain::PRICE_LIST_ITEMS_REPLACED,
        PRICE_LIST_RESOURCE,
        "The prices of a price list were replaced (items at `GET /price-lists/{id}/items`)",
    ),
    event(
        domain::PRODUCT_BUNDLE_UPDATED,
        products::PRODUCT_RESOURCE,
        "The bundle of a product was saved (components at `GET /products/{id}/bundle`)",
    )
    .with_context(&["pricing", "components"]),
    event(
        domain::PRODUCT_BUNDLE_REMOVED,
        products::PRODUCT_RESOURCE,
        "A product stopped being a bundle",
    ),
];

pub struct PriceListsModule;

impl Module for PriceListsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Price lists", APHEL_VERSION)
            .description(
                "Price lists by currency, segment or company with validity periods and quantity tiers, product bundles and the maximum discount per role",
            )
            .depends_on(products::KEY, "*")
            .optional(organizations::KEY)
            .optional(segments::KEY)
            .enabled_by_default()
            .owned_tables(&["price_lists", "price_list_companies", "price_list_items", "product_bundle_components", "product_bundles", "role_discount_limits"])
            .migration(Migration::new(
                1,
                "create price lists",
                include_str!("../migrations/0001_create_price_lists.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::pricing::DESCRIPTOR);
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(60, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::PriceListsSection::new(pool.clone()))
            }),
        ));
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(70, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::DiscountLimitsSection::new(pool.clone()))
            }),
        ));
        app.openapi_tag(
            http::TAG,
            "Price lists by currency, segment or company with validity periods and quantity tiers, and the maximum discount per role",
        )
        .routes(|_| OpenApiRouter::new().merge(http::router()).merge(http::bundle_router()))
        .provide::<dyn PriceListResolver>(Arc::new(CatalogPriceListResolver))
        .provide::<dyn DiscountLimits>(Arc::new(RoleDiscountLimits))
        .provide::<dyn BundleExpander>(Arc::new(CatalogBundleExpander));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        app.contribute::<dyn aphel_kernel::application::public_events::PublicResourceLoader>(
            Arc::new(public::PriceListPublicResource),
        );
        Ok(())
    }
}
