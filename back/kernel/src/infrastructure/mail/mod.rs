pub(crate) mod provider;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::application::mail::{CalendarInvite, EmailAttachment, EmailMessage, Mailer};
use crate::domain::Secret;
use crate::errors::AppError;

pub use provider::{SmtpMailerProvider, mail_not_configured};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SmtpSecurity {
    None,
    StartTls,
    Tls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmtpSettings {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) security: SmtpSecurity,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<Secret>,
    pub(crate) from: String,
}

pub(crate) fn describe_smtp_error(error: &lettre::transport::smtp::Error) -> String {
    let code = error.status().map(|code| code.to_string());
    if matches!(code.as_deref(), Some("530" | "534" | "535")) {
        return format!(
            "authentication failed: the SMTP server rejected the username or password ({error})"
        );
    }
    if error.is_timeout() {
        return format!(
            "the connection to the SMTP server timed out: check the host, the port and the firewall ({error})"
        );
    }
    if error.is_tls() {
        return format!(
            "TLS negotiation failed: check the security mode (none, starttls or tls) for this port ({error})"
        );
    }
    if error.is_permanent() || error.is_transient() {
        return format!("the SMTP server refused the request: {error}");
    }
    format!("could not connect to the SMTP server: check the host and the port ({error})")
}

#[derive(Clone)]
pub(crate) struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    pub(crate) fn with_timeout(
        settings: &SmtpSettings,
        timeout: Option<Duration>,
    ) -> Result<Self, AppError> {
        let builder = match settings.security {
            SmtpSecurity::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
            }
            SmtpSecurity::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
                &settings.host,
            )
            .map_err(|error| AppError::Configuration(format!("invalid SMTP host: {error}")))?,
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.host)
                .map_err(|error| AppError::Configuration(format!("invalid SMTP host: {error}")))?,
        };
        let builder = builder.port(settings.port);
        let builder = match timeout {
            Some(timeout) => builder.timeout(Some(timeout)),
            None => builder,
        };
        let builder = match (&settings.username, &settings.password) {
            (Some(username), Some(password)) => builder.credentials(Credentials::new(
                username.clone(),
                password.expose().to_owned(),
            )),
            _ => builder,
        };
        let from = settings
            .from
            .parse::<Mailbox>()
            .map_err(|error| AppError::Configuration(format!("invalid sender address: {error}")))?;

        Ok(Self {
            transport: builder.build(),
            from,
        })
    }

    pub(crate) async fn test_connection(&self) -> Result<(), AppError> {
        match self.transport.test_connection().await {
            Ok(true) => Ok(()),
            Ok(false) => Err(AppError::Internal(
                "the SMTP server did not accept the connection".to_owned(),
            )),
            Err(error) => Err(AppError::Internal(describe_smtp_error(&error))),
        }
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        self.send_with_attachments(message, Vec::new()).await
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        let to = message
            .to
            .parse::<Mailbox>()
            .map_err(|error| AppError::Validation(format!("invalid recipient: {error}")))?;
        let builder = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(message.subject);
        let body = match message.calendar {
            None => MultiPart::alternative_plain_html(message.text, message.html),
            Some(invite) => calendar_body(message.text, message.html, &invite)?,
        };
        let email = builder
            .multipart(with_attachments(body, attachments)?)
            .map_err(|error| AppError::Internal(format!("failed to build email: {error}")))?;

        self.transport
            .send(email)
            .await
            .map(|_| ())
            .map_err(|error| AppError::Internal(describe_smtp_error(&error)))
    }
}

fn content_type(value: &str) -> Result<ContentType, AppError> {
    ContentType::parse(value)
        .map_err(|error| AppError::Internal(format!("invalid content type: {error}")))
}

fn with_attachments(
    body: MultiPart,
    attachments: Vec<EmailAttachment>,
) -> Result<MultiPart, AppError> {
    if attachments.is_empty() {
        return Ok(body);
    }
    let mut mixed = MultiPart::mixed().multipart(body);
    for attachment in attachments {
        mixed = mixed.singlepart(
            Attachment::new(attachment.filename)
                .body(attachment.bytes, content_type(&attachment.content_type)?),
        );
    }
    Ok(mixed)
}

fn calendar_body(
    text: String,
    html: String,
    invite: &CalendarInvite,
) -> Result<MultiPart, AppError> {
    let method = invite.method.as_str();
    let inline = SinglePart::builder()
        .header(content_type(&format!(
            "text/calendar; method={method}; charset=UTF-8"
        ))?)
        .body(invite.content.clone());
    let body = MultiPart::alternative()
        .singlepart(SinglePart::plain(text))
        .singlepart(SinglePart::html(html))
        .singlepart(inline);
    let attachment = Attachment::new("invite.ics".to_owned()).body(
        invite.content.clone(),
        content_type(&format!("application/ics; method={method}"))?,
    );
    Ok(MultiPart::mixed().multipart(body).singlepart(attachment))
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryMailer {
    sent: Arc<Mutex<Vec<EmailMessage>>>,
}

impl MemoryMailer {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        self.sent
            .lock()
            .map_err(|_| AppError::Internal("memory mailer lock poisoned".to_owned()))?
            .push(message);
        Ok(())
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        _attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        self.send(message).await
    }
}
