use crate::modules::points::IntegrationStore;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use serde_json::{Map, Value};
use sqlx::MySqlPool;

use crate::application::integrations::{
    IntegrationError, IntegrationErrorSource, IntegrationStatus, ResolvedConfig, secret_key,
};
use crate::application::mail::{
    EmailAttachment, EmailMessage, MAIL_NOT_CONFIGURED, Mailer, SharedMailer,
};
use crate::application::organizations::MailStatus;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::demo::{DemoLogMailer, demo_settings};
use crate::infrastructure::integrations::smtp::{PASSWORD_FIELD, PROVIDER_KEY, delivery_mailer};
use crate::infrastructure::mail::SmtpMailer;
use crate::infrastructure::platform::SharedSecretVault;
use crate::infrastructure::repositories::mapper;

pub fn mail_not_configured() -> AppError {
    AppError::validation_code(
        MAIL_NOT_CONFIGURED,
        "no outgoing email server is configured; set up SMTP in the organization settings",
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheKey {
    id: String,
    status: String,
    updated_at: NaiveDateTime,
}

#[derive(Clone)]
struct ResolvedMailer {
    mailer: Arc<SmtpMailer>,
    integration_id: String,
    in_error: bool,
}

pub struct SmtpMailerProvider {
    pool: MySqlPool,
    vault: SharedSecretVault,
    cache: Mutex<Option<(CacheKey, ResolvedMailer)>>,
}

impl SmtpMailerProvider {
    pub(crate) fn new(pool: MySqlPool, vault: SharedSecretVault) -> Self {
        Self {
            pool,
            vault,
            cache: Mutex::new(None),
        }
    }

    pub fn shared(pool: MySqlPool, vault: SharedSecretVault) -> SharedMailer {
        if demo_settings().enabled {
            return Arc::new(DemoLogMailer);
        }
        Arc::new(Self::new(pool, vault))
    }

    pub(crate) async fn is_configured(&self) -> Result<bool, AppError> {
        Ok(match self.store() {
            Some(store) => store.active(&self.pool, PROVIDER_KEY).await?.is_some(),
            None => false,
        })
    }

    fn store(&self) -> Option<Arc<dyn IntegrationStore>> {
        crate::modules::app()
            .contributions::<dyn IntegrationStore>()
            .into_iter()
            .next()
    }

    fn cached(&self, key: &CacheKey) -> Option<ResolvedMailer> {
        self.cache
            .lock()
            .ok()
            .and_then(|cache| cache.clone())
            .filter(|(cached_key, _)| cached_key == key)
            .map(|(_, resolved)| resolved)
    }

    fn remember(&self, key: CacheKey, resolved: ResolvedMailer) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some((key, resolved));
        }
    }

    async fn resolve(&self) -> Result<Option<ResolvedMailer>, AppError> {
        let Some(store) = self.store() else {
            return Ok(None);
        };
        let Some(active) = store.active(&self.pool, PROVIDER_KEY).await? else {
            return Ok(None);
        };
        let key = CacheKey {
            id: active.id,
            status: active.status,
            updated_at: active.updated_at,
        };
        if let Some(resolved) = self.cached(&key) {
            return Ok(Some(resolved));
        }
        let integration_id = uuid::Uuid::parse_str(&key.id)
            .map_err(|_| mapper::invalid_column("integrations.id", &key.id))?;
        let config = active.config;
        let mut secrets = BTreeMap::new();
        if let Some(password) = self
            .vault
            .reveal(
                &AccessScope::system(),
                &secret_key(integration_id, PASSWORD_FIELD),
            )
            .await?
        {
            secrets.insert(PASSWORD_FIELD.to_owned(), password);
        }
        let resolved_config = ResolvedConfig {
            values: match config {
                Value::Object(map) => map,
                _ => Map::new(),
            },
            secrets,
        };
        let mailer = delivery_mailer(&resolved_config).map_err(AppError::Configuration)?;
        let resolved = ResolvedMailer {
            mailer: Arc::new(mailer),
            integration_id: key.id.clone(),
            in_error: IntegrationStatus::parse(&key.status) == Some(IntegrationStatus::Error),
        };
        self.remember(key, resolved.clone());
        Ok(Some(resolved))
    }

    async fn record_failure(&self, integration_id: &str, message: &str) -> Result<(), AppError> {
        let error = IntegrationError::new(Utc::now(), IntegrationErrorSource::Delivery, message);
        match self.store() {
            Some(store) => {
                store
                    .record_failure(&self.pool, integration_id, &error)
                    .await
            }
            None => Ok(()),
        }
    }

    async fn record_recovery(&self, integration_id: &str) -> Result<(), AppError> {
        match self.store() {
            Some(store) => store.record_recovery(&self.pool, integration_id).await,
            None => Ok(()),
        }
    }
}

#[async_trait]
impl Mailer for SmtpMailerProvider {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        self.send_with_attachments(message, Vec::new()).await
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        let resolved = self.resolve().await?.ok_or_else(mail_not_configured)?;
        match resolved
            .mailer
            .send_with_attachments(message, attachments)
            .await
        {
            Ok(()) => {
                if resolved.in_error
                    && let Err(error) = self.record_recovery(&resolved.integration_id).await
                {
                    tracing::warn!("failed to record SMTP recovery: {error}");
                }
                Ok(())
            }
            Err(error) => {
                if let Err(failure) = self
                    .record_failure(&resolved.integration_id, error.client_message())
                    .await
                {
                    tracing::warn!("failed to record SMTP failure: {failure}");
                }
                Err(error)
            }
        }
    }
}

#[async_trait]
impl MailStatus for SmtpMailerProvider {
    async fn configured(&self) -> Result<bool, AppError> {
        self.is_configured().await
    }
}
