pub use aphel_sdk::notifications::{
    ALL_CHANNELS, EMAIL_ONLY, IN_APP_AND_PUSH, NotificationChannel, NotificationKind,
    NotificationKindDefinition, NotificationKindExt,
};

const BUILTIN_NOTIFICATION_KINDS: &[NotificationKindDefinition] = &[NotificationKindDefinition {
    kind: NotificationKind::DAILY_DIGEST,
    channels: EMAIL_ONLY,
    email_default: false,
    push_default: false,
}];

pub fn builtin_notification_kinds() -> &'static [NotificationKindDefinition] {
    BUILTIN_NOTIFICATION_KINDS
}
