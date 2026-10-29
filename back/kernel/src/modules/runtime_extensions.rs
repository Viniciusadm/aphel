use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock, RwLock};

use crate::domain::notifications::{
    NotificationChannel, NotificationKind, NotificationKindDefinition,
};
use crate::modules::builder::DashboardWidget;

#[derive(Default)]
struct Extensions {
    notification_kinds: BTreeMap<String, Vec<NotificationKindDefinition>>,
    widgets: BTreeMap<String, Vec<DashboardWidget>>,
}

fn extensions() -> &'static RwLock<Extensions> {
    static EXTENSIONS: OnceLock<RwLock<Extensions>> = OnceLock::new();
    EXTENSIONS.get_or_init(Default::default)
}

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

pub fn set_notification_kinds(owner: &str, kinds: Vec<NotificationKindDefinition>) {
    if let Ok(mut extensions) = extensions().write() {
        if kinds.is_empty() {
            extensions.notification_kinds.remove(owner);
        } else {
            extensions
                .notification_kinds
                .insert(owner.to_owned(), kinds);
        }
    }
}

pub fn set_widgets(owner: &str, widgets: Vec<DashboardWidget>) {
    if let Ok(mut extensions) = extensions().write() {
        if widgets.is_empty() {
            extensions.widgets.remove(owner);
        } else {
            extensions.widgets.insert(owner.to_owned(), widgets);
        }
    }
}

pub fn notification_kinds() -> Vec<NotificationKindDefinition> {
    extensions()
        .read()
        .map(|extensions| {
            extensions
                .notification_kinds
                .values()
                .flatten()
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

pub fn notification_kind(kind: &str) -> Option<NotificationKindDefinition> {
    extensions().read().ok().and_then(|extensions| {
        extensions
            .notification_kinds
            .values()
            .flatten()
            .find(|definition| definition.kind.as_str() == kind)
            .copied()
    })
}

pub fn widgets() -> Vec<DashboardWidget> {
    extensions()
        .read()
        .map(|extensions| extensions.widgets.values().flatten().cloned().collect())
        .unwrap_or_default()
}
