use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::domain::notifications::{
    NotificationChannel, NotificationKind, NotificationKindDefinition,
};

pub fn intern(value: &str) -> &'static str {
    static VALUES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut values = VALUES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = values.get(value) {
        return existing;
    }
    let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
    values.insert(leaked);
    leaked
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
