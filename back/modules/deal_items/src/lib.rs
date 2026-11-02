pub mod domain;
pub(crate) mod hooks;
pub mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod recurring;
pub(crate) mod section;
pub mod service;
pub(crate) mod store;
pub mod widgets;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::ItemTable;
use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::config_packages::ConfigSectionFactory;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_kernel::records::RecordValidating;
use deals::AmountCalculator;
use quotes::DealItemsWriter;

pub(crate) use hooks::{ItemsAmountCalculator, QuoteDealItemsWriter};

pub const KEY: &str = "deal_items";

pub struct DealItemsModule;

impl Module for DealItemsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Deal items", APHEL_VERSION)
            .description(
                "Line items of a deal, the amount mode and the recurring revenue (MRR, ARR, TCV and contract dates)",
            )
            .enabled_by_default()
            .depends_on(deals::KEY, "*")
            .depends_on(products::KEY, "*")
            .optional(quotes::KEY)
            .owned_tables(&["deal_item_settings", "deal_items"])
            .migration(Migration::new(
                1,
                "create deal items",
                include_str!("../migrations/0001_create_deal_items.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::fields::EntityKeyAliases(&[(
                "deal_item_settings",
                "deal_items.deal_item_settings",
            )]),
        ));
        let revenue_fields = [
            field("mrr", crate::labels("MRR"), FieldType::Currency)?,
            field("arr", crate::labels("ARR"), FieldType::Currency)?,
            field(
                "tcv",
                crate::labels("Total contract value"),
                FieldType::Currency,
            )?,
            field(
                "contract_start",
                crate::labels("Contract start"),
                FieldType::Date,
            )?,
            field(
                "contract_end",
                crate::labels("Contract end"),
                FieldType::Date,
            )?,
        ];
        app.extend("deals.deal", |deal| {
            for revenue_field in revenue_fields {
                deal.system_field(revenue_field);
            }
            deal.item_table(
                ItemTable::new("items", crate::labels("Items"), "deal_items", "deal_id")
                    .column("total", crate::labels("Total"))
                    .column("quantity", crate::labels("Quantity"))
                    .column("unit_price", crate::labels("Unit price"))
                    .column("discount_value", crate::labels("Desconto")),
            );
        })
        .provide::<dyn AmountCalculator>(Arc::new(ItemsAmountCalculator))
        .provide::<dyn DealItemsWriter>(Arc::new(QuoteDealItemsWriter))
        .openapi_tag(
            http::TAG,
            "Line items of a deal (products or free items) and the deal amount mode",
        )
        .routes(|_| http::router())
        .widget(Arc::new(widgets::RecurringRevenueWidget))
        .on(
            RecordValidating,
            hooks::CURRENCY_LOCK_PRIORITY,
            |ctx, args| Box::pin(hooks::block_currency_change_with_items(ctx, args)),
        );
        app.contribute(Arc::new(ConfigSectionFactory::new(
            55,
            |pool, _registry| Arc::new(section::DealItemsSection::new(pool.clone())),
        )));
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
