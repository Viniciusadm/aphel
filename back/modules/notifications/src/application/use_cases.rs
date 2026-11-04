use aphel_sdk::notifications::NotificationKindExt;
use std::collections::BTreeMap;

use uuid::Uuid;

use crate::application::{
    NotificationFilter, NotificationPage, NotificationPorts, NotificationPreferenceRepository,
    NotificationRepository, PreferenceSetting,
};
use crate::domain::{NotificationChannel, NotificationCursor, NotificationKind};
use aphel_kernel::application::security::Clock;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const NOTIFICATION_NOT_FOUND: &str = "notification_not_found";
pub const MAX_PREFERENCE_UPDATES: usize = 100;
pub const MAX_NOTIFICATION_PAGE_SIZE: u32 = 100;
pub const DEFAULT_NOTIFICATION_PAGE_SIZE: u32 = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceUpdate {
    pub kind: String,
    pub channel: String,
    pub enabled: bool,
}

pub fn notification_not_found() -> AppError {
    AppError::not_found_code(NOTIFICATION_NOT_FOUND, "notification was not found")
}

pub fn effective_preferences(stored: &[PreferenceSetting]) -> Vec<PreferenceSetting> {
    let overrides: BTreeMap<(NotificationKind, NotificationChannel), bool> = stored
        .iter()
        .map(|setting| ((setting.kind, setting.channel), setting.enabled))
        .collect();
    NotificationKind::all()
        .into_iter()
        .flat_map(|kind| kind.channels().iter().map(move |channel| (kind, *channel)))
        .map(|(kind, channel)| PreferenceSetting {
            kind,
            channel,
            enabled: overrides
                .get(&(kind, channel))
                .copied()
                .unwrap_or_else(|| kind.default_enabled(channel)),
        })
        .collect()
}

pub async fn preferences_of<R>(
    preferences: &R,
    user_id: &UserId,
) -> Result<Vec<PreferenceSetting>, AppError>
where
    R: NotificationPreferenceRepository,
{
    let stored = preferences.stored(user_id).await?;
    Ok(effective_preferences(&stored))
}

pub struct NotificationUseCases<P> {
    ports: P,
}

impl<P> NotificationUseCases<P>
where
    P: NotificationPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        filter: NotificationFilter,
        cursor: Option<&str>,
        per_page: u32,
    ) -> Result<NotificationPage, AppError> {
        let after = match cursor.filter(|value| !value.is_empty()) {
            None => None,
            Some(value) => Some(NotificationCursor::decode(value).ok_or_else(|| {
                AppError::invalid_fields(
                    "invalid cursor",
                    vec![FieldViolation::new(
                        "cursor",
                        "must be the meta.next_cursor of a previous page",
                    )],
                )
            })?),
        };
        let per_page = per_page.clamp(1, MAX_NOTIFICATION_PAGE_SIZE);
        let mut items = self
            .ports
            .notifications()
            .list(scope, filter, after, per_page + 1)
            .await?;
        let has_more = items.len() > per_page as usize;
        items.truncate(per_page as usize);
        let next_cursor = if has_more {
            items
                .last()
                .map(|item| NotificationCursor::new(item.created_at, item.id).encode())
        } else {
            None
        };
        Ok(NotificationPage { items, next_cursor })
    }

    pub async fn unread_count(&self, scope: &AccessScope) -> Result<u64, AppError> {
        self.ports.notifications().count_unread(scope).await
    }

    pub async fn mark_read(&self, scope: &AccessScope, id: &str) -> Result<(), AppError> {
        let id = Uuid::parse_str(id).map_err(|_| notification_not_found())?;
        let now = self.ports.clock().now();
        if self.ports.notifications().mark_read(scope, id, now).await? {
            Ok(())
        } else {
            Err(notification_not_found())
        }
    }

    pub async fn mark_all_read(&self, scope: &AccessScope) -> Result<u64, AppError> {
        let now = self.ports.clock().now();
        self.ports.notifications().mark_all_read(scope, now).await
    }

    pub async fn preferences(
        &self,
        scope: &AccessScope,
    ) -> Result<Vec<PreferenceSetting>, AppError> {
        preferences_of(self.ports.preferences(), scope.user_id()).await
    }

    pub async fn update_preferences(
        &self,
        scope: &AccessScope,
        updates: Vec<PreferenceUpdate>,
    ) -> Result<Vec<PreferenceSetting>, AppError> {
        if updates.len() > MAX_PREFERENCE_UPDATES {
            return Err(AppError::invalid_fields(
                "invalid notification preferences",
                vec![FieldViolation::new(
                    "preferences",
                    format!("must have at most {MAX_PREFERENCE_UPDATES} items"),
                )],
            ));
        }
        let mut violations = Vec::new();
        let mut settings: BTreeMap<(NotificationKind, NotificationChannel), bool> = BTreeMap::new();
        for (index, update) in updates.iter().enumerate() {
            let kind = NotificationKind::parse(update.kind.trim());
            let channel = NotificationChannel::parse(update.channel.trim());
            match (kind, channel) {
                (None, _) => violations.push(FieldViolation::new(
                    format!("preferences.{index}.kind"),
                    "unknown notification kind",
                )),
                (Some(_), None) => violations.push(FieldViolation::new(
                    format!("preferences.{index}.channel"),
                    "must be in_app, email or push",
                )),
                (Some(kind), Some(channel)) if !kind.supports(channel) => {
                    violations.push(FieldViolation::new(
                        format!("preferences.{index}.channel"),
                        "this notification kind does not support the channel",
                    ))
                }
                (Some(kind), Some(channel)) => {
                    settings.insert((kind, channel), update.enabled);
                }
            }
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid notification preferences",
                violations,
            ));
        }
        let settings: Vec<PreferenceSetting> = settings
            .into_iter()
            .map(|((kind, channel), enabled)| PreferenceSetting {
                kind,
                channel,
                enabled,
            })
            .collect();
        if !settings.is_empty() {
            let now = self.ports.clock().now();
            self.ports.preferences().save(scope, &settings, now).await?;
        }
        self.preferences(scope).await
    }
}
