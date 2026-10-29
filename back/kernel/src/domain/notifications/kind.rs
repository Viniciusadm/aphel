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

    pub fn all() -> Vec<Self> {
        crate::modules::app()
            .catalog()
            .notification_kinds()
            .map(|definition| definition.kind)
            .collect()
    }

    pub fn parse(value: &str) -> Option<Self> {
        crate::modules::app()
            .catalog()
            .notification_kind(value)
            .map(|definition| definition.kind)
    }

    fn definition(&self) -> Option<NotificationKindDefinition> {
        crate::modules::app()
            .catalog()
            .notification_kind(self.0)
            .copied()
    }

    pub fn channels(&self) -> &'static [NotificationChannel] {
        self.definition()
            .map(|definition| definition.channels)
            .unwrap_or(&[])
    }

    pub fn supports(&self, channel: NotificationChannel) -> bool {
        self.channels().contains(&channel)
    }

    pub fn default_enabled(&self, channel: NotificationChannel) -> bool {
        let Some(definition) = self.definition() else {
            return false;
        };
        if !definition.channels.contains(&channel) {
            return false;
        }
        match channel {
            NotificationChannel::InApp => true,
            NotificationChannel::Email => definition.email_default,
            NotificationChannel::Push => definition.push_default,
        }
    }
}

const BUILTIN_NOTIFICATION_KINDS: &[NotificationKindDefinition] = &[NotificationKindDefinition {
    kind: NotificationKind::DAILY_DIGEST,
    channels: EMAIL_ONLY,
    email_default: false,
    push_default: false,
}];

pub fn builtin_notification_kinds() -> &'static [NotificationKindDefinition] {
    BUILTIN_NOTIFICATION_KINDS
}
