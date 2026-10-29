use std::time::Duration;

use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Tokio1Executor};

use crate::store::imap::client::{resolve, timed};
use crate::store::imap::config::{ImapSettings, NetworkPolicy, Security};
use crate::{EmailAddress, MailProviderError, ProviderEmail};

pub(crate) const SMTP_TIMEOUT: Duration = Duration::from_secs(30);
const PROTECTED_HEADERS: [&str; 12] = [
    "from",
    "to",
    "cc",
    "bcc",
    "subject",
    "message-id",
    "in-reply-to",
    "references",
    "content-type",
    "content-transfer-encoding",
    "mime-version",
    "date",
];

pub(crate) async fn transport(
    settings: &ImapSettings,
    password: &str,
    policy: &NetworkPolicy,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailProviderError> {
    resolve(&settings.smtp_host, settings.smtp_port, policy).await?;
    let builder = match settings.smtp_security {
        Security::None => {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.smtp_host)
        }
        Security::StartTls => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)
                .map_err(|_| MailProviderError::rejected("invalid SMTP host"))?
        }
        Security::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.smtp_host)
            .map_err(|_| MailProviderError::rejected("invalid SMTP host"))?,
    };
    Ok(builder
        .port(settings.smtp_port)
        .credentials(Credentials::new(
            settings.smtp_username.clone(),
            password.to_owned(),
        ))
        .timeout(Some(SMTP_TIMEOUT))
        .build())
}

pub(crate) fn smtp_failure(error: &lettre::transport::smtp::Error) -> MailProviderError {
    let code = error
        .status()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "-".to_owned());
    if error.is_permanent() {
        if code.starts_with("535") || code.starts_with("534") || code.starts_with("530") {
            return MailProviderError::authentication(
                "the SMTP server rejected the user name or password",
            );
        }
        MailProviderError::rejected(format!(
            "the SMTP server rejected the message (code {code})"
        ))
    } else if error.is_timeout() {
        MailProviderError::connection("the SMTP server did not answer in time")
    } else {
        MailProviderError::connection(format!(
            "the SMTP server could not take the message now (code {code})"
        ))
    }
}

pub(crate) async fn verify(
    settings: &ImapSettings,
    password: &str,
    policy: &NetworkPolicy,
) -> Result<(), MailProviderError> {
    let transport = transport(settings, password, policy).await?;
    let reachable = timed(transport.test_connection(), SMTP_TIMEOUT * 2, |error| {
        smtp_failure(&error)
    })
    .await?;
    if reachable {
        Ok(())
    } else {
        Err(MailProviderError::connection(
            "the SMTP server did not accept the connection",
        ))
    }
}

fn mailbox(address: &EmailAddress) -> Result<Mailbox, MailProviderError> {
    let parsed = address
        .address
        .parse()
        .map_err(|_| MailProviderError::rejected(format!("invalid address {}", address.address)))?;
    Ok(Mailbox::new(address.name.clone(), parsed))
}

fn bracket(id: &str) -> String {
    format!("<{}>", id.trim().trim_matches(['<', '>']))
}

pub(crate) fn build_message(email: &ProviderEmail) -> Result<lettre::Message, MailProviderError> {
    let invalid = |message: &str| MailProviderError::rejected(message.to_owned());
    let mut builder = lettre::Message::builder()
        .from(mailbox(&email.from)?)
        .subject(email.subject.clone())
        .message_id(Some(bracket(&email.message_id)));
    for address in &email.to {
        builder = builder.to(mailbox(address)?);
    }
    for address in &email.cc {
        builder = builder.cc(mailbox(address)?);
    }
    for address in &email.bcc {
        builder = builder.bcc(mailbox(address)?);
    }
    if let Some(reply_to) = &email.reply_to {
        builder = builder.reply_to(mailbox(reply_to)?);
    }
    if let Some(in_reply_to) = &email.in_reply_to {
        builder = builder.in_reply_to(bracket(in_reply_to));
    }
    if !email.references.is_empty() {
        builder = builder.references(
            email
                .references
                .iter()
                .map(|id| bracket(id))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    for (name, value) in &email.headers {
        if PROTECTED_HEADERS.contains(&name.trim().to_lowercase().as_str())
            || value.contains(['\r', '\n'])
        {
            continue;
        }
        let header = HeaderName::new_from_ascii(name.trim().to_owned())
            .map_err(|_| invalid("invalid custom header name"))?;
        builder = builder.raw_header(HeaderValue::new(header, value.clone()));
    }
    let body = match &email.html {
        Some(html) => MultiPart::alternative_plain_html(email.text.clone(), html.clone()),
        None => MultiPart::mixed().singlepart(SinglePart::plain(email.text.clone())),
    };
    let message = if email.attachments.is_empty() {
        builder.multipart(body)
    } else {
        let mut mixed = MultiPart::mixed().multipart(body);
        for attachment in &email.attachments {
            let content_type = ContentType::parse(&attachment.mime_type)
                .unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("valid"));
            let part = match (&attachment.content_id, attachment.inline) {
                (Some(content_id), true) => Attachment::new_inline_with_name(
                    content_id.trim_matches(['<', '>']).to_owned(),
                    attachment.name.clone(),
                ),
                _ => Attachment::new(attachment.name.clone()),
            }
            .body(attachment.content.clone(), content_type);
            mixed = mixed.singlepart(part);
        }
        builder.multipart(mixed)
    };
    message.map_err(|_| invalid("could not build the e-mail"))
}
