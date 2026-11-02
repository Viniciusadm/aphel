pub mod engine;
mod mailer;
pub mod smtp;
use crate::engine::IntegrationError;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::MySqlPool;
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
        app.contribute::<dyn crate::IntegrationStore>(std::sync::Arc::new(points::Integrations));
        app.contribute::<dyn aphel_kernel::application::mail::MailTransportSource>(
            std::sync::Arc::new(MailSource),
        );
        app.openapi_tag(
            http::TAG,
            "Integration center: providers, configuration, connection test and activation",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveIntegration {
    pub id: String,
    pub status: String,
    pub updated_at: NaiveDateTime,
    pub config: Value,
}

#[async_trait]
pub trait IntegrationStore: Send + Sync {
    async fn active(
        &self,
        pool: &MySqlPool,
        provider_key: &str,
    ) -> Result<Option<ActiveIntegration>, AppError>;

    async fn record_failure(
        &self,
        pool: &MySqlPool,
        integration_id: &str,
        error: &IntegrationError,
    ) -> Result<(), AppError>;

    async fn record_recovery(&self, pool: &MySqlPool, integration_id: &str)
    -> Result<(), AppError>;
}

struct MailSource;
#[async_trait]
impl aphel_kernel::application::mail::MailTransportSource for MailSource {
    async fn mailer(
        &self,
        pool: &MySqlPool,
        vault: &aphel_kernel::infrastructure::platform::SharedSecretVault,
    ) -> Result<Option<aphel_kernel::application::mail::SharedMailer>, AppError> {
        let provider = mailer::IntegrationMailer::new(pool.clone(), vault.clone());
        if provider.is_configured().await? {
            Ok(Some(std::sync::Arc::new(provider)))
        } else {
            Ok(None)
        }
    }
}
