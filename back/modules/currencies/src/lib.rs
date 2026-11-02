pub mod domain;
pub(crate) mod hooks;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod provider;
pub(crate) mod public;
pub(crate) mod runtime;
pub(crate) mod section;
pub mod service;
pub mod store;
pub(crate) mod sync;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::application::fields::builtin::field;
use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use hooks::{CurrencyConverter, RateCurrencyConverter};

pub const KEY: &str = "currencies";

pub(crate) const EXCHANGE_RATE_RESOURCE: PublicResource =
    PublicResource::new("exchange_rate", "ExchangeRateResponse");
pub(crate) const CURRENCY_RESOURCE: PublicResource = PublicResource::payload("currency");
pub(crate) const CURRENCY_BASE_RESOURCE: PublicResource = PublicResource::payload("currency_base");

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        domain::EXCHANGE_RATE_CREATED,
        EXCHANGE_RATE_RESOURCE,
        "An exchange rate was registered, by hand or fetched from the configured provider",
    ),
    changed(
        domain::EXCHANGE_RATE_UPDATED,
        EXCHANGE_RATE_RESOURCE,
        "An exchange rate changed (edited by hand or refreshed by the provider)",
        PreviousValues::Changes,
    ),
    deleted(
        domain::EXCHANGE_RATE_DELETED,
        EXCHANGE_RATE_RESOURCE,
        "An exchange rate was removed",
    ),
    event(
        domain::CURRENCY_ENABLED,
        CURRENCY_RESOURCE,
        "A currency was enabled for the organization (`data` has `currency` and `base`)",
    ),
    event(
        domain::CURRENCY_DISABLED,
        CURRENCY_RESOURCE,
        "A currency was disabled for the organization (`data` has `currency` and `base`)",
    ),
    changed(
        domain::EXCHANGE_RATES_REBASED,
        CURRENCY_BASE_RESOURCE,
        "The base currency of the organization changed and the stored rates were converted to it",
        PreviousValues::Fields(&[("base", "previous_base")]),
    ),
];

pub struct CurrenciesModule;

impl Module for CurrenciesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Currencies", APHEL_VERSION)
            .description(
                "Enabled currencies of the organization, dated exchange rates and the conversion of amounts to the base currency",
            )
            .required()
            .owned_tables(&["currency_settings", "exchange_rates", "organization_currencies"])
            .migration(Migration::new(
                1,
                "create currencies",
                include_str!("../migrations/0001_create_currencies.sql"),
            ))
            .migration(Migration::new(
                2,
                "create currency settings",
                include_str!("../migrations/0002_create_currency_settings.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                50,
                |pool, registry| {
                    let _ = registry;
                    std::sync::Arc::new(crate::section::CurrenciesSection::new(pool.clone()))
                },
            ),
        ));
        let amount_base = field(
            domain::BASE_AMOUNT_FIELD,
            "Valor na moeda base",
            FieldType::Currency,
        )?;
        let exchange_rate = field("exchange_rate", crate::labels("Quote"), FieldType::Number)?;
        app.extend("deal", |deal| {
            deal.system_field(amount_base.clone())
                .system_field(exchange_rate.clone());
        })
        .provide::<dyn CurrencyConverter>(Arc::new(RateCurrencyConverter))
        .openapi_tag(
            http::TAG,
            "Enabled currencies of the organization and dated exchange rates to the base currency",
        )
        .routes(|_| http::router())
        .job(|env| runtime::exchange_rates_sync_handler(env.pool.clone()))
        .schedule(runtime::exchange_rates_sync_schedule())
        .consumer(|env| Arc::new(runtime::BaseCurrencyConsumer::new(env.pool.clone())));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        app.contribute::<dyn aphel_kernel::application::public_events::PublicResourceLoader>(
            Arc::new(public::ExchangeRatePublicResource),
        );
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
