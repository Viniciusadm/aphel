pub(crate) mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod http;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;

use std::sync::Arc;

use aphel_kernel::modules::Migration;
use aphel_sdk::configuration::ConfigSectionFactory;

use crate::section::ForecastSection;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "forecast";

pub struct ForecastModule;

impl Module for ForecastModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Forecast", APHEL_VERSION)
            .description(
                "Revenue forecast per month from open deals, with the organization method, categories and horizon",
            )
            .optional(aphel_sdk::deals::KEY)
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create forecast settings",
                include_str!("../migrations/0001_create_forecast_settings.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Revenue forecast: organization settings and the forecast per month",
        )
        .routes(|_| http::router())
        .contribute(Arc::new(ConfigSectionFactory::new(
            225,
            |pool, registry| Arc::new(ForecastSection::new(pool.clone(), registry.clone())),
        )));
        exposed_operations::register(app);
        Ok(())
    }
}
