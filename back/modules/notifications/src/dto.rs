use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::application::{
    DEFAULT_NOTIFICATION_PAGE_SIZE, MAX_NOTIFICATION_PAGE_SIZE, NotificationFilter,
    NotificationPage, NotificationView, PreferenceSetting, PreferenceUpdate,
};
use aphel_kernel::api::dto::common::timestamp as instant;
use aphel_kernel::api::dto::files::UserReferenceResponse;
use aphel_kernel::errors::{AppError, FieldViolation};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = NotificationChannel)]
pub enum NotificationChannelSchema {
    InApp,
    Email,
    Push,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct NotificationsQueryParams {
    pub unread: Option<bool>,
    #[param(example = "313737333834363030303030303030307c30313930...")]
    pub cursor: Option<String>,
    #[param(minimum = 1, maximum = 100, example = 25)]
    pub per_page: Option<u32>,
}

impl NotificationsQueryParams {
    pub fn filter(&self) -> NotificationFilter {
        NotificationFilter {
            unread_only: self.unread.unwrap_or(false),
        }
    }

    pub fn per_page(&self) -> Result<u32, AppError> {
        match self.per_page {
            None => Ok(DEFAULT_NOTIFICATION_PAGE_SIZE),
            Some(value) if (1..=MAX_NOTIFICATION_PAGE_SIZE).contains(&value) => Ok(value),
            Some(_) => Err(AppError::invalid_fields(
                "invalid pagination parameters",
                vec![FieldViolation::new(
                    "per_page",
                    format!("must be an integer between 1 and {MAX_NOTIFICATION_PAGE_SIZE}"),
                )],
            )),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationPageMeta {
    #[schema(example = "313737333834363030303030303030307c30313930")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationPageResponse {
    pub data: Vec<NotificationResponse>,
    pub meta: NotificationPageMeta,
}

impl From<NotificationPage> for NotificationPageResponse {
    fn from(page: NotificationPage) -> Self {
        Self {
            data: page.items.into_iter().map(Into::into).collect(),
            meta: NotificationPageMeta {
                next_cursor: page.next_cursor,
            },
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: String,
    #[schema(example = "task_assigned")]
    pub kind: String,
    #[schema(example = "Call Acme")]
    pub title: String,
    pub body: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub actor: Option<UserReferenceResponse>,
    #[schema(example = "/tasks/0190a8f2-7c1e-7b3a-9f00-0000000000a1")]
    pub link: Option<String>,
    #[schema(value_type = Object)]
    pub data: Value,
    pub read: bool,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub read_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
}

impl From<NotificationView> for NotificationResponse {
    fn from(view: NotificationView) -> Self {
        Self {
            id: view.id.to_string(),
            kind: view.kind.as_str().to_owned(),
            title: view.title,
            body: view.body,
            record_type: view
                .record
                .as_ref()
                .map(|record| record.record_type.as_str().to_owned()),
            record_id: view
                .record
                .as_ref()
                .map(|record| record.record_id.to_string()),
            actor: view.actor.map(Into::into),
            link: view.link,
            data: view.data,
            read: view.read_at.is_some(),
            read_at: view.read_at.map(instant),
            created_at: instant(view.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MarkAllReadResponse {
    pub updated: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NotificationPreferenceItem {
    #[schema(example = "task_assigned")]
    pub kind: String,
    #[schema(value_type = NotificationChannelSchema)]
    pub channel: String,
    pub enabled: bool,
}

impl From<PreferenceSetting> for NotificationPreferenceItem {
    fn from(setting: PreferenceSetting) -> Self {
        Self {
            kind: setting.kind.as_str().to_owned(),
            channel: setting.channel.as_str().to_owned(),
            enabled: setting.enabled,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationPreferencesResponse {
    pub preferences: Vec<NotificationPreferenceItem>,
}

impl From<Vec<PreferenceSetting>> for NotificationPreferencesResponse {
    fn from(settings: Vec<PreferenceSetting>) -> Self {
        Self {
            preferences: settings.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateNotificationPreferencesRequest {
    pub preferences: Vec<NotificationPreferenceItem>,
}

impl UpdateNotificationPreferencesRequest {
    pub fn into_updates(self) -> Vec<PreferenceUpdate> {
        self.preferences
            .into_iter()
            .map(|item| PreferenceUpdate {
                kind: item.kind,
                channel: item.channel,
                enabled: item.enabled,
            })
            .collect()
    }
}
