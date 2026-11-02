use crate::domain::{NotificationChannel, NotificationKind, NotificationKindDefinition};
pub use aphel_kernel::modules::intern::intern;
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

pub fn runtime_notification_kind(
    kind: &str,
    channels: &'static [NotificationChannel],
    email_default: bool,
    push_default: bool,
) -> NotificationKindDefinition {
    NotificationKindDefinition {
        kind: NotificationKind::from_static(intern(kind)),
        channels,
        email_default,
        push_default,
    }
}
