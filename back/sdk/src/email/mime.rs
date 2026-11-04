use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use mail_parser::{Address, ContentType as ParsedContentType, MessageParser, MimeHeaders};

use super::MAIL_INVALID_MESSAGE;
use super::mailbox::{EmailAddress, EmailAttachmentContent, FetchedEmail, ProviderEmail};
use aphel_kernel::errors::AppError;

const MAX_REFERENCES: usize = 50;

pub fn decode_base64url(value: &str) -> Option<Vec<u8>> {
    let cleaned: String = value
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '=')
        .collect();
    URL_SAFE_NO_PAD.decode(cleaned).ok()
}

pub fn encode_base64url(bytes: &[u8]) -> String {
    URL_SAFE.encode(bytes)
}

pub fn encode_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn normalize_message_id(value: &str) -> Option<String> {
    let id = value
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        return None;
    }
    Some(id.to_owned())
}

fn header_ids(value: &mail_parser::HeaderValue<'_>) -> Vec<String> {
    let raw: Vec<String> = match value {
        mail_parser::HeaderValue::Text(text) => vec![text.to_string()],
        mail_parser::HeaderValue::TextList(list) => list.iter().map(ToString::to_string).collect(),
        _ => Vec::new(),
    };
    raw.iter()
        .flat_map(|item| {
            item.split(|character: char| character.is_whitespace() || character == ',')
                .filter_map(normalize_message_id)
                .collect::<Vec<_>>()
        })
        .take(MAX_REFERENCES)
        .collect()
}

fn addresses(address: Option<&Address<'_>>) -> Vec<EmailAddress> {
    address
        .map(|address| {
            address
                .iter()
                .filter_map(|addr| {
                    let email = addr.address()?.trim();
                    if !email.contains('@') {
                        return None;
                    }
                    Some(EmailAddress {
                        address: email.to_lowercase(),
                        name: addr
                            .name()
                            .map(str::trim)
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn mime_type(content_type: Option<&ParsedContentType<'_>>) -> String {
    match content_type {
        Some(content_type) => match content_type.subtype() {
            Some(subtype) => format!("{}/{subtype}", content_type.ctype()).to_lowercase(),
            None => content_type.ctype().to_lowercase(),
        },
        None => "application/octet-stream".to_owned(),
    }
}

fn full_content_type(content_type: Option<&ParsedContentType<'_>>) -> Option<String> {
    let content_type = content_type?;
    let mut value = mime_type(Some(content_type));
    for attribute in content_type.attributes().unwrap_or_default() {
        if attribute.name.eq_ignore_ascii_case("boundary") {
            continue;
        }
        value.push_str(&format!("; {}={}", attribute.name, attribute.value));
    }
    Some(value)
}

fn report_message_id(message: &mail_parser::Message<'_>) -> Option<String> {
    message.attachments().find_map(|part| {
        if let Some(inner) = part.message() {
            return inner.message_id().and_then(normalize_message_id);
        }
        let headers_part = mime_type(part.content_type()) == "text/rfc822-headers";
        if headers_part {
            let mut raw = part.contents().to_vec();
            raw.extend_from_slice(b"\r\n\r\n");
            return MessageParser::default()
                .parse(&raw)
                .and_then(|inner| inner.message_id().and_then(normalize_message_id));
        }
        None
    })
}

fn attachments(message: &mail_parser::Message<'_>) -> Vec<EmailAttachmentContent> {
    message
        .attachments()
        .map(|part| {
            let content_id = part
                .content_id()
                .and_then(normalize_message_id)
                .filter(|id| id.len() <= 255);
            let inline = part
                .content_disposition()
                .map(|disposition| disposition.is_inline())
                .unwrap_or(content_id.is_some());
            let mime_type = mime_type(part.content_type());
            let name = part
                .attachment_name()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if mime_type == "message/rfc822" {
                        "message.eml".to_owned()
                    } else {
                        "attachment".to_owned()
                    }
                });
            EmailAttachmentContent {
                name,
                mime_type,
                content: part.contents().to_vec(),
                content_id,
                inline,
            }
        })
        .collect()
}

pub fn parse_remote(raw: &[u8]) -> Option<FetchedEmail> {
    let message = MessageParser::default().parse(raw)?;
    let html = message
        .html_part(0)
        .filter(|part| part.is_text_html())
        .and_then(|part| part.text_contents())
        .map(str::to_owned);
    let text = message
        .text_part(0)
        .filter(|part| part.is_text() && !part.is_text_html())
        .and_then(|part| part.text_contents())
        .map(str::to_owned)
        .or_else(|| message.body_text(0).map(|text| text.into_owned()));
    Some(FetchedEmail {
        provider_message_id: None,
        message_id: message.message_id().and_then(normalize_message_id),
        in_reply_to: header_ids(message.in_reply_to()).into_iter().next(),
        references: header_ids(message.references()),
        subject: message.subject().unwrap_or_default().trim().to_owned(),
        from: addresses(message.from()).into_iter().next(),
        to: addresses(message.to()),
        cc: addresses(message.cc()),
        bcc: addresses(message.bcc()),
        reply_to: addresses(message.reply_to()),
        date: message
            .date()
            .and_then(|date| DateTime::<Utc>::from_timestamp(date.to_timestamp(), 0)),
        text,
        html,
        attachments: attachments(&message),
        content_type: full_content_type(message.content_type()),
        auto_submitted: message
            .header_raw("Auto-Submitted")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        report_for_message_id: report_message_id(&message),
        sent_by_account: false,
    })
}

fn invalid(message: &str) -> AppError {
    AppError::validation_code(MAIL_INVALID_MESSAGE, message.to_owned())
}

fn mailbox(address: &EmailAddress) -> Result<Mailbox, AppError> {
    let parsed = address
        .address
        .trim()
        .parse()
        .map_err(|_| invalid("an e-mail address is invalid"))?;
    Ok(Mailbox::new(
        address
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        parsed,
    ))
}

fn bracketed(id: &str) -> Option<String> {
    normalize_message_id(id).map(|id| format!("<{id}>"))
}

fn attachment_part(attachment: &EmailAttachmentContent) -> SinglePart {
    let content_type = ContentType::parse(&attachment.mime_type)
        .or_else(|_| ContentType::parse("application/octet-stream"))
        .unwrap_or(ContentType::TEXT_PLAIN);
    let builder = match (&attachment.content_id, attachment.inline) {
        (Some(content_id), true) => {
            Attachment::new_inline_with_name(content_id.clone(), attachment.name.clone())
        }
        _ => Attachment::new(attachment.name.clone()),
    };
    builder.body(attachment.content.clone(), content_type)
}

pub fn build(mail: &ProviderEmail) -> Result<Vec<u8>, AppError> {
    let message_id = bracketed(&mail.message_id).ok_or_else(|| invalid("missing message id"))?;
    let mut builder = lettre::Message::builder()
        .from(mailbox(&mail.from)?)
        .subject(mail.subject.clone())
        .message_id(Some(message_id))
        .date_now()
        .keep_bcc();
    for address in &mail.to {
        builder = builder.to(mailbox(address)?);
    }
    for address in &mail.cc {
        builder = builder.cc(mailbox(address)?);
    }
    for address in &mail.bcc {
        builder = builder.bcc(mailbox(address)?);
    }
    if let Some(reply_to) = &mail.reply_to {
        builder = builder.reply_to(mailbox(reply_to)?);
    }
    if let Some(in_reply_to) = mail.in_reply_to.as_deref().and_then(bracketed) {
        builder = builder.in_reply_to(in_reply_to);
    }
    let references: Vec<String> = mail
        .references
        .iter()
        .filter_map(|id| bracketed(id))
        .take(MAX_REFERENCES)
        .collect();
    if !references.is_empty() {
        builder = builder.references(references.join(" "));
    }
    for (name, value) in &mail.headers {
        let Ok(header) = HeaderName::new_from_ascii(name.clone()) else {
            continue;
        };
        if value
            .chars()
            .any(|character| character == '\r' || character == '\n')
        {
            continue;
        }
        builder = builder.raw_header(HeaderValue::new(header, value.clone()));
    }
    let text = mail.text.clone();
    let message = match (&mail.html, mail.attachments.is_empty()) {
        (Some(html), true) => {
            builder.multipart(MultiPart::alternative_plain_html(text, html.clone()))
        }
        (None, true) => builder.singlepart(SinglePart::plain(text)),
        (html, false) => {
            let mixed = match html {
                Some(html) => MultiPart::mixed()
                    .multipart(MultiPart::alternative_plain_html(text, html.clone())),
                None => MultiPart::mixed().singlepart(SinglePart::plain(text)),
            };
            let mixed = mail.attachments.iter().fold(mixed, |mixed, attachment| {
                mixed.singlepart(attachment_part(attachment))
            });
            builder.multipart(mixed)
        }
    }
    .map_err(|_| invalid("could not build the e-mail"))?;
    Ok(message.formatted())
}
