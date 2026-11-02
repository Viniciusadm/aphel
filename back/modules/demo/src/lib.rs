pub(crate) mod cli;
pub(crate) mod dataset;
pub(crate) mod domain;
pub(crate) mod revenue;
pub(crate) mod service;
pub(crate) mod store;

pub(crate) use dataset::*;
pub(crate) use domain::*;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "demo";

pub struct DemoModule;

impl Module for DemoModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Demo", APHEL_VERSION)
            .description(
                "Fictional demo data (seed, removal and reset) and the public demo mode status",
            )
            .depends_on(onboarding::KEY, "*")
            .optional(currencies::KEY)
            .optional(pipelines::KEY)
            .optional(products::KEY)
            .optional(price_lists::KEY)
            .optional(deal_items::KEY)
            .optional(quotes::KEY)
            .optional(approvals::KEY)
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create demo records",
                include_str!("../migrations/0001_create_demo_records.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::fields::EntityKeyAliases(&[
                ("approval_policy", "approvals.approval_policy"),
                ("exchange_rate", "currencies.exchange_rate"),
                ("price_list", "price_lists.price_list"),
            ]),
        ));
        app.contribute::<dyn aphel_kernel::modules::ModuleCommand>(std::sync::Arc::new(
            cli::DemoCli,
        ));
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
