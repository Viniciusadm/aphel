use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::DEFAULT_HISTORY_DAYS;
use crate::{FetchedEmail, ParticipantView};
use aphel_kernel::application::files::sha256_hex;
use aphel_kernel::domain::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaptureSettings {
    pub(crate) create_contacts: bool,
    pub(crate) create_companies: bool,
    pub(crate) link_open_deals: bool,
    pub(crate) initial_history_days: u32,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            create_contacts: false,
            create_companies: false,
            link_open_deals: true,
            initial_history_days: DEFAULT_HISTORY_DAYS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BlockKind {
    Domain,
    Address,
}

impl BlockKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Address => "address",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "domain" => Some(Self::Domain),
            "address" => Some(Self::Address),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockedEntry {
    pub(crate) id: Uuid,
    pub(crate) kind: BlockKind,
    pub(crate) value: String,
    pub(crate) reason: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncFailure {
    pub(crate) kind: String,
    pub(crate) failures: u32,
    pub(crate) retry_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredMailboxFile {
    pub(crate) id: Uuid,
    pub(crate) storage_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThreadParticipant {
    pub(crate) thread_id: Uuid,
    pub(crate) participant: ParticipantView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContactMatch {
    pub(crate) address: String,
    pub(crate) contact_id: Uuid,
}

pub(crate) const MAX_SUBJECT_LENGTH: usize = 1000;
pub(crate) const MAX_PARTICIPANT_NAME_LENGTH: usize = 200;
pub(crate) const MAX_PROVIDER_MESSAGE_ID_LENGTH: usize = 255;
pub(crate) const MAX_HEADER_ID_LENGTH: usize = 998;
pub(crate) const AUTHENTICATION_FAILURE: &str = "authentication";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncMailboxRequest {
    pub(crate) mailbox_id: Uuid,
    pub(crate) owner: UserId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SyncOutcome {
    pub(crate) imported: u32,
    pub(crate) skipped: u32,
}

pub(crate) fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    let lower = html.to_lowercase();
    let mut index = 0usize;
    let mut skip_until: Option<&'static str> = None;
    while let Some(character) = chars.next() {
        let position = index;
        index += character.len_utf8();
        if let Some(end) = skip_until {
            if character == '<' && lower[position..].starts_with(end) {
                skip_until = None;
            } else {
                continue;
            }
        }
        if character == '<' {
            let rest = &lower[position..];
            if rest.starts_with("<script") {
                skip_until = Some("</script");
            } else if rest.starts_with("<style") {
                skip_until = Some("</style");
            } else if rest.starts_with("<br") || rest.starts_with("<p") || rest.starts_with("<div")
            {
                text.push('\n');
            }
            for next in chars.by_ref() {
                index += next.len_utf8();
                if next == '>' {
                    break;
                }
            }
            continue;
        }
        text.push(character);
    }
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub(crate) fn synthetic_message_id(email: &FetchedEmail) -> String {
    let seed = format!(
        "{}|{}|{}|{}",
        email
            .date
            .map(|date| date.timestamp().to_string())
            .unwrap_or_default(),
        email
            .from
            .as_ref()
            .map(|from| from.address.to_lowercase())
            .unwrap_or_default(),
        email.subject.trim(),
        email.provider_message_id.as_deref().unwrap_or_default(),
    );
    format!("aphel-{}@sync.invalid", &sha256_hex(seed.as_bytes())[..40])
}
