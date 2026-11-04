use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};

pub const CONTRACT: Contract = contract!("notifications", "1.0.0");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "notifications.source",
            KeyKind::ExtensionPoint,
            "Owner-provided notification intents",
        ),
        KeyEntry::new(
            "notifications.texts",
            KeyKind::ExtensionPoint,
            "Owner-provided notification presentation",
        ),
    ],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NotificationChannel {
    InApp,
    Email,
    Push,
}

impl NotificationChannel {
    pub const ALL: [Self; 3] = [Self::InApp, Self::Email, Self::Push];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
            Self::Push => "push",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|channel| channel.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotificationKind(&'static str);

pub const ALL_CHANNELS: &[NotificationChannel] = &[
    NotificationChannel::InApp,
    NotificationChannel::Email,
    NotificationChannel::Push,
];
pub const EMAIL_ONLY: &[NotificationChannel] = &[NotificationChannel::Email];
pub const IN_APP_AND_PUSH: &[NotificationChannel] =
    &[NotificationChannel::InApp, NotificationChannel::Push];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationKindDefinition {
    pub kind: NotificationKind,
    pub channels: &'static [NotificationChannel],
    pub email_default: bool,
    pub push_default: bool,
}

impl NotificationKindDefinition {
    pub const fn new(kind: NotificationKind) -> Self {
        Self {
            kind,
            channels: ALL_CHANNELS,
            email_default: false,
            push_default: false,
        }
    }

    pub const fn channels(mut self, channels: &'static [NotificationChannel]) -> Self {
        self.channels = channels;
        self
    }

    pub const fn email_by_default(mut self) -> Self {
        self.email_default = true;
        self
    }

    pub const fn push_by_default(mut self) -> Self {
        self.push_default = true;
        self
    }
}

impl NotificationKind {
    pub const DAILY_DIGEST: Self = Self("daily_digest");
    pub const AUTOMATION_MESSAGE: Self = Self("automation_message");
    pub const AUTOMATION_EMAIL: Self = Self("automation_email");
    pub const AUTOMATION_SKIPPED: Self = Self("automation_skipped");

    pub const fn from_static(value: &'static str) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

pub fn channel_set(in_app: bool, email: bool, push: bool) -> &'static [NotificationChannel] {
    use NotificationChannel::{Email, InApp, Push};
    match (in_app, email, push) {
        (true, true, true) => &[InApp, Email, Push],
        (true, true, false) => &[InApp, Email],
        (true, false, true) => &[InApp, Push],
        (false, true, true) => &[Email, Push],
        (true, false, false) => &[InApp],
        (false, true, false) => &[Email],
        (false, false, true) => &[Push],
        (false, false, false) => &[],
    }
}
