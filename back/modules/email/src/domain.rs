pub(crate) const EMAIL_PREVIEW_LENGTH: usize = 280;
pub use aphel_sdk::email::MAX_ADDRESS_LENGTH;
pub(crate) const DEFAULT_HISTORY_DAYS: u32 = 90;
pub(crate) const MAX_HISTORY_DAYS: u32 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum EmailVisibility {
    Private,
    #[default]
    Metadata,
    Shared,
}

impl EmailVisibility {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Metadata => "metadata",
            Self::Shared => "shared",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "private" => Some(Self::Private),
            "metadata" => Some(Self::Metadata),
            "shared" => Some(Self::Shared),
            _ => None,
        }
    }

    pub(crate) fn access_for(&self, is_owner: bool) -> EmailAccess {
        if is_owner {
            return EmailAccess::Full;
        }
        match self {
            Self::Private => EmailAccess::None,
            Self::Metadata => EmailAccess::Metadata,
            Self::Shared => EmailAccess::Full,
        }
    }
}

pub(crate) fn effective_visibility(
    thread: Option<EmailVisibility>,
    mailbox_default: EmailVisibility,
) -> EmailVisibility {
    thread.unwrap_or(mailbox_default)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum EmailAccess {
    None,
    Metadata,
    Full,
}

impl EmailAccess {
    pub(crate) fn can_see(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub(crate) fn can_read_content(&self) -> bool {
        matches!(self, Self::Full)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MailboxStatus {
    Active,
    Syncing,
    Error,
    Disconnected,
}

impl MailboxStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Syncing => "syncing",
            Self::Error => "error",
            Self::Disconnected => "disconnected",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "syncing" => Some(Self::Syncing),
            "error" => Some(Self::Error),
            "disconnected" => Some(Self::Disconnected),
            _ => None,
        }
    }

    pub(crate) fn can_sync(&self) -> bool {
        matches!(self, Self::Active | Self::Syncing | Self::Error)
    }

    pub(crate) fn can_send(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EmailDirection {
    Inbound,
    Outbound,
}

impl EmailDirection {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "inbound" => Some(Self::Inbound),
            "outbound" => Some(Self::Outbound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum EmailKind {
    #[default]
    Message,
    Bounce,
    AutoReply,
}

impl EmailKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Bounce => "bounce",
            Self::AutoReply => "auto_reply",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "message" => Some(Self::Message),
            "bounce" => Some(Self::Bounce),
            "auto_reply" => Some(Self::AutoReply),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EmailStatus {
    Received,
    Draft,
    Scheduled,
    Sending,
    Sent,
    Failed,
    Cancelled,
}

impl EmailStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::Draft => "draft",
            Self::Scheduled => "scheduled",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "received" => Some(Self::Received),
            "draft" => Some(Self::Draft),
            "scheduled" => Some(Self::Scheduled),
            "sending" => Some(Self::Sending),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ParticipantRole {
    From,
    To,
    Cc,
    Bcc,
    ReplyTo,
}

impl ParticipantRole {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
            Self::Cc => "cc",
            Self::Bcc => "bcc",
            Self::ReplyTo => "reply_to",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "from" => Some(Self::From),
            "to" => Some(Self::To),
            "cc" => Some(Self::Cc),
            "bcc" => Some(Self::Bcc),
            "reply_to" => Some(Self::ReplyTo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LinkOrigin {
    Automatic,
    Manual,
}

impl LinkOrigin {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Manual => "manual",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "automatic" => Some(Self::Automatic),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

pub use aphel_sdk::email::normalize_address;

pub(crate) fn address_domain(address: &str) -> Option<&str> {
    address
        .rsplit_once('@')
        .map(|(_, domain)| domain)
        .filter(|domain| !domain.is_empty())
}

pub(crate) fn normalize_message_id(value: &str) -> Option<String> {
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

pub(crate) fn parse_references(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter_map(normalize_message_id)
        .collect()
}

pub(crate) fn normalize_subject(subject: &str) -> String {
    let mut rest = subject.trim();
    loop {
        let lower = rest.to_lowercase();
        let prefix = ["re:", "fw:", "fwd:", "res:", "enc:", "aw:", "wg:", "sv:"]
            .iter()
            .find(|prefix| lower.starts_with(*prefix));
        match prefix {
            Some(prefix) => rest = rest[prefix.len()..].trim_start(),
            None => break,
        }
    }
    rest.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn reply_subject(subject: &str) -> String {
    let normalized = normalize_subject(subject);
    if normalized.is_empty() {
        return "Re:".to_owned();
    }
    format!("Re: {normalized}")
}

pub(crate) fn email_preview(text: &str) -> String {
    let collapsed = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('>'))
        .collect::<Vec<_>>()
        .join(" ");
    let collapsed = collapsed.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview: String = collapsed.chars().take(EMAIL_PREVIEW_LENGTH).collect();
    if collapsed.chars().count() > EMAIL_PREVIEW_LENGTH {
        preview.push('…');
    }
    preview
}

pub(crate) fn classify_message(
    from_address: Option<&str>,
    content_type: Option<&str>,
    auto_submitted: Option<&str>,
) -> EmailKind {
    let content_type = content_type.unwrap_or_default().to_lowercase();
    let from_local = from_address
        .and_then(|address| address.split('@').next())
        .unwrap_or_default()
        .to_lowercase();
    if (content_type.contains("multipart/report") && content_type.contains("delivery-status"))
        || from_local == "mailer-daemon"
        || from_local == "postmaster"
    {
        return EmailKind::Bounce;
    }
    match auto_submitted.map(|value| value.trim().to_lowercase()) {
        Some(value) if !value.is_empty() && value != "no" => EmailKind::AutoReply,
        _ => EmailKind::Message,
    }
}

pub(crate) fn validate_history_days(days: u32) -> Result<u32, &'static str> {
    if days == 0 || days > MAX_HISTORY_DAYS {
        return Err("must be between 1 and 3650");
    }
    Ok(days)
}
