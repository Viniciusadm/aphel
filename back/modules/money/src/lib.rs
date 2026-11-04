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

use aphel_kernel::application::public_events::{
    PreviousValues, PublicEventDefinition, PublicResource, changed, deleted, event,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

use hooks::{CurrencyConverter, RateCurrencyConverter};

pub const KEY: &str = "money";

pub(crate) const EXCHANGE_RATE_RESOURCE: PublicResource =
    PublicResource::new("money.exchange_rate", "ExchangeRateResponse");
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

pub struct MoneyModule;

impl Module for MoneyModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Money", APHEL_VERSION)
            .description(
                "Enabled currencies of the organization, dated exchange rates and the conversion of amounts to the base currency",
            )
            .owned_tables(&["money_settings", "money_exchange_rates", "money_currencies"])
            .migration(Migration::new(
                1,
                "create currencies",
                include_str!("../migrations/0001_create_currencies.sql"),
            ))
            .migration(Migration::new(
                2,
                "create currency settings",
                include_str!("../migrations/0002_create_money_settings.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(50, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::CurrenciesSection::new(pool.clone()))
            }),
        ));
        app.contract(aphel_sdk::money::DESCRIPTOR);
        use aphel_kernel::application::fields::{
            EntityDefinition, EntityTraits,
            builtin::{field, timestamps, with_fields},
        };
        use aphel_kernel::application::shared::RecordType;
        use aphel_kernel::domain::fields::FieldType;
        app.entity(with_fields(
            EntityDefinition::new(
                RecordType::from_static("money.exchange_rate"),
                "money_exchange_rates",
                aphel_sdk::money::READ_PERMISSION,
            )
            .capabilities(
                EntityTraits::default()
                    .with_trait("records.global")
                    .localized_labels(aphel_kernel::application::fields::LocalizedLabels(
                        std::collections::BTreeMap::from([
                            ("en".into(), "Exchange rates".into()),
                            ("pt-BR".into(), "Taxas de câmbio".into()),
                        ]),
                    )),
            )
            .title_field("currency")
            .system_field(field("currency", labels("Currency"), FieldType::Text)?)
            .system_field(field(
                "effective_date",
                labels("Effective date"),
                FieldType::Date,
            )?)
            .system_field(field("rate", labels("Rate"), FieldType::Number)?)
            .system_field(field("created_by", labels("Created by"), FieldType::User)?),
            timestamps()?,
        ));
        app.permission(
            aphel_kernel::domain::access::PermissionDefinition::administrative(
                aphel_sdk::money::READ_PERMISSION,
            ),
        )
        .grant_to(["admin", "manager", "member"]);
        app.provide::<dyn CurrencyConverter>(Arc::new(RateCurrencyConverter))
        .provide::<dyn aphel_sdk::money::MoneyRates>(Arc::new(RateCurrencyConverter))
        .openapi_tag(
            http::TAG,
            "Enabled currencies of the organization and dated exchange rates to the base currency",
        )
        .routes(|_| http::router())
        .job(|env| runtime::exchange_rates_sync_handler(env.pool.clone()))
        .job(|env| Arc::new(runtime::MoneyRecalculateHandler::new(env.pool.clone())))
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

fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
