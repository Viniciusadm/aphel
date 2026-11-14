pub(crate) mod dto;
pub mod engine;
pub(crate) mod http;
mod points;
pub(crate) mod registry;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "integrations";

pub struct IntegrationsModule;

impl Module for IntegrationsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Integrations", APHEL_VERSION)
            .description(
                "Integration center: connection cards contributed by modules (webhooks, channels, AI, single sign-on) and a provider framework with configuration, connection test and activation",
            )
            .owned_tables(&["integrations"])
            .migration(Migration::new(
                1,
                "create integrations",
                include_str!("../migrations/0001_create_integrations.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::modules::HealthSource>(std::sync::Arc::new(
            points::IntegrationFailures,
        ));
        app.openapi_tag(
            http::TAG,
            "Integration center: providers, configuration, connection test and activation",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
