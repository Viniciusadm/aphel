use super::{SmtpMailer, SmtpSecurity, SmtpSettings};
use crate::application::mail::{
    EmailAttachment, EmailMessage, MAIL_NOT_CONFIGURED, Mailer, SharedMailer,
};
use crate::application::organizations::MailStatus;
use crate::application::secrets::SecretVault;
use crate::errors::AppError;
use crate::infrastructure::platform::SharedSecretVault;
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

pub fn mail_not_configured() -> AppError {
    AppError::validation_code(
        MAIL_NOT_CONFIGURED,
        "no outgoing mail transport is configured",
    )
}
pub struct SmtpMailerProvider {
    pool: MySqlPool,
    vault: SharedSecretVault,
}
impl SmtpMailerProvider {
    pub fn new(pool: MySqlPool, vault: SharedSecretVault) -> Self {
        Self { pool, vault }
    }
    pub fn shared(pool: MySqlPool, vault: SharedSecretVault) -> SharedMailer {
        Arc::new(Self::new(pool, vault))
    }
    async fn resolve(&self) -> Result<Option<SharedMailer>, AppError> {
        if let Some(mailer) = crate::application::mail::delivery_override() {
            return Ok(Some(mailer));
        }
        let settings: Option<sqlx::types::Json<serde_json::Value>> =
            sqlx::query_scalar("SELECT settings FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        let config = settings.map(|settings| settings.0).unwrap_or_default();
        let mail = config.get("mail").filter(|mail| {
            mail.get("host")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|host| !host.is_empty())
        });
        let text = |key: &str, env: &str| {
            mail.and_then(|mail| mail.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| std::env::var(env).ok())
                .filter(|value| !value.is_empty())
        };
        if let (Some(host), Some(from)) = (text("host", "SMTP_HOST"), text("from", "SMTP_FROM")) {
            let security = match text("security", "SMTP_SECURITY").as_deref() {
                Some("none") => SmtpSecurity::None,
                Some("tls") => SmtpSecurity::Tls,
                _ => SmtpSecurity::StartTls,
            };
            let settings = SmtpSettings {
                host,
                from,
                security,
                port: mail
                    .and_then(|mail| mail.get("port"))
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|port| u16::try_from(port).ok())
                    .or_else(|| {
                        std::env::var("SMTP_PORT")
                            .ok()
                            .and_then(|port| port.parse().ok())
                    })
                    .unwrap_or(587),
                username: text("username", "SMTP_USERNAME"),
                password: match self
                    .vault
                    .reveal(
                        &crate::domain::access::AccessScope::system(),
                        &crate::application::organizations::mail_secret_key(),
                    )
                    .await?
                {
                    Some(password) => Some(password),
                    None => std::env::var("SMTP_PASSWORD")
                        .ok()
                        .map(crate::domain::Secret::new),
                },
            };
            return Ok(Some(Arc::new(SmtpMailer::with_timeout(&settings, None)?)));
        }
        for source in crate::modules::app()
            .contributions::<dyn crate::application::mail::MailTransportSource>()
        {
            if let Some(mailer) = source.mailer(&self.pool, &self.vault).await? {
                return Ok(Some(mailer));
            }
        }
        Ok(None)
    }
    pub async fn is_configured(&self) -> Result<bool, AppError> {
        Ok(self.resolve().await?.is_some())
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
        self.resolve()
            .await?
            .ok_or_else(mail_not_configured)?
            .send_with_attachments(message, attachments)
            .await
    }
}
#[async_trait]
impl MailStatus for SmtpMailerProvider {
    async fn configured(&self) -> Result<bool, AppError> {
        self.is_configured().await
    }
}
