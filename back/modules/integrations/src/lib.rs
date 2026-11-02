pub(crate) mod dto;
pub(crate) mod http;
mod points;
pub(crate) mod registry;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "integrations";

pub struct IntegrationsModule;

impl Module for IntegrationsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Integrations", APHEL_VERSION)
            .description(
                "Integration center: providers (SMTP, generic webhook and module providers), configuration, connection test and activation",
            )
            .required()
            .owned_tables(&["integrations"])
            .migration(Migration::new(
                1,
                "create integrations",
                include_str!("../migrations/0001_create_integrations.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::application::fields::EntityKeyAliases(&[(
                "integration",
                "integrations.integration",
            )]),
        ));
        app.contribute::<dyn aphel_kernel::modules::HealthSource>(std::sync::Arc::new(
            points::IntegrationFailures,
        ));
        app.contribute::<dyn aphel_kernel::modules::IntegrationStore>(std::sync::Arc::new(
            points::Integrations,
        ));
        app.openapi_tag(
            http::TAG,
            "Integration center: providers, configuration, connection test and activation",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
