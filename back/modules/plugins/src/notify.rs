use aphel_sdk::notifications::NotificationKindExt;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::registry::LoadedPlugin;
use crate::runtime::wit;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::state::runtime_state;
use aphel_sdk::notifications::NotificationClient;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::{NotificationIntent, dispatch};

const NOT_DECLARED: &str = "plugin_notification_not_declared";
const INVALID_RECIPIENT: &str = "plugin_notification_recipient_invalid";
const RECIPIENT_CANNOT_READ: &str = "plugin_notification_record_not_readable";
const INVALID_NOTIFICATION: &str = "plugin_notification_invalid";
const MAX_TITLE_LENGTH: usize = 200;
const MAX_BODY_LENGTH: usize = 2000;
const MAX_LINK_LENGTH: usize = 500;

pub fn kind_key(plugin: &str, name: &str) -> String {
    format!("{plugin}_{name}")
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::validation_code(INVALID_NOTIFICATION, message)
}

pub async fn notify(
    plugin: &LoadedPlugin,
    scope: &AccessScope,
    notification: wit::Notification,
) -> Result<(), AppError> {
    if !plugin
        .manifest
        .notifications
        .iter()
        .any(|spec| spec.name == notification.kind)
    {
        return Err(AppError::not_found_code(
            NOT_DECLARED,
            format!(
                "the plugin {} does not declare the notification {}",
                plugin.key, notification.kind
            ),
        ));
    }
    let kind =
        NotificationKind::parse(&kind_key(&plugin.key, &notification.kind)).ok_or_else(|| {
            AppError::unavailable_code(NOT_DECLARED, "the notification kind is not loaded yet")
        })?;
    let title = notification.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > MAX_TITLE_LENGTH {
        return Err(invalid(format!(
            "the title must have 1 to {MAX_TITLE_LENGTH} characters"
        )));
    }
    let body = notification
        .body
        .map(|body| body.trim().to_owned())
        .filter(|body| !body.is_empty());
    if body
        .as_ref()
        .is_some_and(|body| body.chars().count() > MAX_BODY_LENGTH)
    {
        return Err(invalid(format!(
            "the body must have up to {MAX_BODY_LENGTH} characters"
        )));
    }
    if notification.link.as_ref().is_some_and(|link| {
        !link.starts_with('/') || link.starts_with("//") || link.len() > MAX_LINK_LENGTH
    }) {
        return Err(invalid(
            "the link must be a path inside Aphel, such as /deals/<id>",
        ));
    }
    let data: Value = serde_json::from_str(&notification.data_json)
        .map_err(|_| invalid("data-json must be valid JSON"))?;
    let record = match (&notification.record_type, &notification.record_id) {
        (Some(record_type), Some(record_id)) => Some(RecordRef::parse(record_type, record_id)?),
        (None, None) => None,
        _ => return Err(invalid("record-type and record-id must be sent together")),
    };
    let recipient = UserId::new(notification.recipient_user_id.clone()).map_err(|_| {
        AppError::validation_code(INVALID_RECIPIENT, "the recipient must be a user id")
    })?;
    let state = runtime_state()?;
    let ports = NotificationClient::new(
        state.db()?,
        state.registry().await?,
        Arc::new(MySqlJobQueue::new(state.db()?)),
    );
    if ports.directory()?.recipient(&recipient).await?.is_none() {
        return Err(AppError::validation_code(
            INVALID_RECIPIENT,
            "the recipient is not an active user",
        ));
    }
    if let Some(record) = &record
        && !ports.directory()?.can_read(&recipient, record).await?
    {
        return Err(AppError::forbidden_code(
            RECIPIENT_CANNOT_READ,
            "the recipient cannot read the record of the notification",
        ));
    }
    let dedupe_key = notification.dedupe_key.unwrap_or_else(|| {
        format!(
            "{}:{}:{}:{}",
            plugin.key,
            notification.kind,
            recipient.as_str(),
            Uuid::now_v7()
        )
    });
    let actor =
        (!scope.is_system() && scope.plugin_key().is_none()).then(|| scope.user_id().clone());
    let intent = NotificationIntent::new(
        recipient,
        kind,
        title,
        format!("plugin:{}:{dedupe_key}", plugin.key),
    )
    .with_body(body)
    .with_record(record)
    .with_link(notification.link)
    .with_actor(actor)
    .with_data(data);
    dispatch(&ports, intent).await?;
    Ok(())
}
