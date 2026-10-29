use chrono::{DateTime, Utc};
use mail_parser::{Address, HeaderValue, Message, MessageParser, MimeHeaders};

use crate::domain::normalize_message_id;
use crate::{EmailAddress, EmailAttachmentContent, FetchedEmail};

const MAX_REFERENCES: usize = 50;

fn header_ids(value: &HeaderValue<'_>) -> Vec<String> {
    let raw: Vec<String> = match value {
        HeaderValue::Text(text) => vec![text.to_string()],
        HeaderValue::TextList(list) => list.iter().map(ToString::to_string).collect(),
        _ => Vec::new(),
    };
    raw.iter()
        .flat_map(|item| {
            item.split(|character: char| character.is_whitespace() || character == ',')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter_map(|item| normalize_message_id(&item))
        .take(MAX_REFERENCES)
        .collect()
}

fn addresses(value: Option<&Address<'_>>) -> Vec<EmailAddress> {
    value
        .map(|address| {
            address
                .iter()
                .filter_map(|addr| {
                    addr.address().map(|address| EmailAddress {
                        address: address.to_owned(),
                        name: addr.name().map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn content_type(message: &Message<'_>) -> Option<String> {
    let content_type = message.root_part().content_type()?;
    let mut value = match content_type.subtype() {
        Some(subtype) => format!("{}/{}", content_type.ctype(), subtype),
        None => content_type.ctype().to_owned(),
    };
    if let Some(report) = content_type.attribute("report-type") {
        value.push_str("; report-type=");
        value.push_str(report);
    }
    Some(value.to_lowercase())
}

fn report_for(message: &Message<'_>) -> Option<String> {
    for part in &message.parts {
        if let Some(nested) = part.message()
            && let Some(id) = nested.message_id().and_then(normalize_message_id)
        {
            return Some(id);
        }
        let is_headers = part.content_type().is_some_and(|content_type| {
            content_type.ctype().eq_ignore_ascii_case("text")
                && content_type
                    .subtype()
                    .is_some_and(|subtype| subtype.eq_ignore_ascii_case("rfc822-headers"))
        });
        if is_headers
            && let Some(headers) = MessageParser::default().parse(part.contents())
            && let Some(id) = headers.message_id().and_then(normalize_message_id)
        {
            return Some(id);
        }
    }
    None
}

fn attachments(message: &Message<'_>) -> Vec<EmailAttachmentContent> {
    message
        .attachments()
        .map(|part| {
            let mime_type = part
                .content_type()
                .map(|content_type| match content_type.subtype() {
                    Some(subtype) => format!("{}/{}", content_type.ctype(), subtype),
                    None => content_type.ctype().to_owned(),
                })
                .unwrap_or_else(|| "application/octet-stream".to_owned())
                .to_lowercase();
            let name = part
                .attachment_name()
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if mime_type == "message/rfc822" {
                        "message.eml".to_owned()
                    } else {
                        "attachment".to_owned()
                    }
                });
            let inline = part
                .content_disposition()
                .is_some_and(|disposition| disposition.is_inline());
            let content = match part.message() {
                Some(nested) => nested.raw_message().to_vec(),
                None => part.contents().to_vec(),
            };
            EmailAttachmentContent {
                name,
                mime_type,
                content,
                content_id: part.content_id().map(str::to_owned),
                inline,
            }
        })
        .collect()
}

pub(crate) fn parse_message(
    bytes: &[u8],
    provider_message_id: String,
    sent_folder: bool,
    headers_only: bool,
) -> Option<FetchedEmail> {
    let message = MessageParser::default().parse(bytes)?;
    let date = message
        .date()
        .and_then(|date| DateTime::<Utc>::from_timestamp(date.to_timestamp(), 0));
    let html = if headers_only {
        None
    } else {
        message
            .html_part(0)
            .filter(|part| part.is_text_html())
            .and_then(|part| part.text_contents())
            .map(str::to_owned)
    };
    let text = if headers_only {
        None
    } else {
        message.body_text(0).map(|text| text.into_owned())
    };
    Some(FetchedEmail {
        provider_message_id: Some(provider_message_id),
        message_id: message.message_id().and_then(normalize_message_id),
        in_reply_to: header_ids(message.in_reply_to()).into_iter().next(),
        references: header_ids(message.references()),
        subject: message.subject().unwrap_or_default().trim().to_owned(),
        from: addresses(message.from()).into_iter().next(),
        to: addresses(message.to()),
        cc: addresses(message.cc()),
        bcc: addresses(message.bcc()),
        reply_to: addresses(message.reply_to()),
        date,
        text,
        html,
        attachments: if headers_only {
            Vec::new()
        } else {
            attachments(&message)
        },
        content_type: content_type(&message),
        auto_submitted: message
            .header_raw("Auto-Submitted")
            .map(|value| value.trim().to_owned()),
        report_for_message_id: report_for(&message),
        sent_by_account: sent_folder,
    })
}
