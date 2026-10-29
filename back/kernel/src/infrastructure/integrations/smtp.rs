use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::application::integrations::{
    ConfigField, ConfigFieldType, ConfigSchema, ConnectionTestOptions, IntegrationProvider,
    ResolvedConfig,
};
use crate::application::mail::{EmailMessage, Mailer};
use crate::domain::Secret;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::demo::demo_settings;
use crate::infrastructure::mail::{SmtpMailer, SmtpSecurity, SmtpSettings};

pub(crate) const PROVIDER_KEY: &str = "smtp";
pub(crate) const PASSWORD_FIELD: &str = "password";
const TEST_TIMEOUT: Duration = Duration::from_secs(15);
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(30);

fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|character| !matches!(character, '"' | '\\' | '<' | '>' | '\r' | '\n'))
        .collect::<String>()
        .trim()
        .to_owned()
}

pub(crate) fn smtp_settings(config: &ResolvedConfig) -> Result<SmtpSettings, String> {
    let host = config.text("host").ok_or("the SMTP host is missing")?;
    let port = config
        .integer("port")
        .and_then(|port| u16::try_from(port).ok())
        .ok_or("the SMTP port is invalid")?;
    let security = match config.text("security").unwrap_or("starttls") {
        "none" => SmtpSecurity::None,
        "tls" => SmtpSecurity::Tls,
        _ => SmtpSecurity::StartTls,
    };
    let from_email = config
        .text("from_email")
        .ok_or("the sender address is missing")?;
    let from = match config.text("from_name").map(sanitize_name) {
        Some(name) if !name.is_empty() => format!("\"{name}\" <{from_email}>"),
        _ => from_email.to_owned(),
    };
    let username = config.text("username").map(str::to_owned);
    let password = config
        .secret(PASSWORD_FIELD)
        .map(|secret| Secret::new(secret.expose()));
    Ok(SmtpSettings {
        host: host.to_owned(),
        port,
        security,
        username,
        password,
        from,
    })
}

pub(crate) fn smtp_mailer(
    config: &ResolvedConfig,
    timeout: Duration,
) -> Result<SmtpMailer, String> {
    let config = smtp_settings(config)?;
    SmtpMailer::with_timeout(&config, Some(timeout))
        .map_err(|error| error.client_message().to_owned())
}

pub(crate) fn delivery_mailer(config: &ResolvedConfig) -> Result<SmtpMailer, String> {
    smtp_mailer(config, DELIVERY_TIMEOUT)
}

fn test_message(to: &str) -> EmailMessage {
    let pt_br = "Este é um e-mail de teste do Aphel. Se você recebeu esta mensagem, o e-mail de envio do Aphel está configurado corretamente.";
    let en = "This is a test email from Aphel. If you received this message, Aphel's outgoing email is configured correctly.";
    EmailMessage {
        calendar: None,
        to: to.to_owned(),
        subject: "Teste de SMTP do Aphel / Aphel SMTP test".to_owned(),
        html: format!("<p lang=\"pt-BR\">{pt_br}</p><p lang=\"en\">{en}</p>"),
        text: format!("{pt_br}\n\n{en}"),
    }
}

fn message_of(error: AppError) -> String {
    error.client_message().to_owned()
}

pub struct SmtpIntegrationProvider;

#[async_trait]
impl IntegrationProvider for SmtpIntegrationProvider {
    fn key(&self) -> &'static str {
        PROVIDER_KEY
    }

    fn name(&self) -> &'static str {
        "Outgoing email (SMTP)"
    }

    fn description(&self) -> &'static str {
        "SMTP server used for every email the system sends (invitations, password resets, notifications and reports)"
    }

    fn icon(&self) -> &'static str {
        "mail"
    }

    fn config_schema(&self) -> ConfigSchema {
        let (host, port, security) = if cfg!(debug_assertions) {
            (Some(json!("127.0.0.1")), 1025, "none")
        } else {
            (None, 587, "starttls")
        };
        let host_field = ConfigField::new("host", ConfigFieldType::Text).required();
        let host_field = match host {
            Some(host) => host_field.with_default(host),
            None => host_field,
        };
        ConfigSchema::new(vec![
            host_field,
            ConfigField::new("port", ConfigFieldType::Number)
                .required()
                .with_default(json!(port))
                .with_range(1, 65535),
            ConfigField::new("security", ConfigFieldType::Select)
                .required()
                .with_options(&["none", "starttls", "tls"])
                .with_default(json!(security)),
            ConfigField::new("username", ConfigFieldType::Text),
            ConfigField::new(PASSWORD_FIELD, ConfigFieldType::Secret),
            ConfigField::new("from_email", ConfigFieldType::Email).required(),
            ConfigField::new("from_name", ConfigFieldType::Text),
        ])
    }

    fn validate(&self, config: &ResolvedConfig) -> Vec<FieldViolation> {
        let mut violations = Vec::new();
        if let Some(host) = config.text("host")
            && (host.contains(char::is_whitespace) || host.contains('/'))
        {
            violations.push(FieldViolation::new("config.host", "must be a host name"));
        }
        violations
    }

    fn requires_successful_test_to_activate(&self) -> bool {
        true
    }

    async fn test_connection(
        &self,
        config: &ResolvedConfig,
        options: &ConnectionTestOptions,
    ) -> Result<(), String> {
        let mailer = smtp_mailer(config, TEST_TIMEOUT)?;
        mailer.test_connection().await.map_err(message_of)?;
        if let Some(recipient) = &options.send_test_email_to {
            if demo_settings().enabled {
                return Err("sending e-mail is disabled in the demo".to_owned());
            }
            mailer
                .send(test_message(recipient))
                .await
                .map_err(message_of)?;
        }
        Ok(())
    }
}
