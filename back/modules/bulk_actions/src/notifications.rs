use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{BULK_OPERATION_COMPLETED, BULK_OPERATION_FAILED};
use aphel_sdk::platform::application::custom_objects::{CatalogEventConsumer, RecordTypeCatalog};

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationClient;
use aphel_sdk::notifications::record_type_label;
use aphel_sdk::notifications::{ALL_CHANNELS, NotificationKind, NotificationKindDefinition};
use aphel_sdk::notifications::{NotificationIntent, dispatch};
use aphel_sdk::platform::application::mail::MailLocale;
use aphel_sdk::platform::application::shared::{DomainEvent, EventConsumer};
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;

pub(crate) const BULK_NOTIFICATIONS_CONSUMER: &str = "bulk_actions.notifications";
pub(crate) const BULK_OPERATION_FINISHED: NotificationKind =
    NotificationKind::from_static("bulk_operation_finished");
pub(crate) const BULK_OPERATION_FAILED_KIND: NotificationKind =
    NotificationKind::from_static("bulk_operation_failed");
pub(crate) const BULK_OPERATIONS_LINK: &str = "/data/bulk-operations";

pub(crate) fn notification_kinds() -> [NotificationKindDefinition; 2] {
    [BULK_OPERATION_FINISHED, BULK_OPERATION_FAILED_KIND].map(|kind| NotificationKindDefinition {
        kind,
        channels: ALL_CHANNELS,
        email_default: false,
        push_default: false,
    })
}

pub(crate) struct BulkNotificationConsumer {
    ports: NotificationClient,
}

impl BulkNotificationConsumer {
    pub(crate) fn shared(
        pool: MySqlPool,
        catalog: Arc<RecordTypeCatalog>,
    ) -> Arc<dyn EventConsumer> {
        CatalogEventConsumer::shared(
            catalog,
            Arc::new(move |registry| {
                Arc::new(Self {
                    ports: NotificationClient::new(
                        pool.clone(),
                        registry,
                        Arc::new(MySqlJobQueue::new(pool.clone())),
                    ),
                }) as Arc<dyn EventConsumer>
            }),
        )
    }
}

fn title(kind: NotificationKind, record_type: &str) -> String {
    let label = record_type_label(record_type, MailLocale::default());
    if kind == BULK_OPERATION_FINISHED {
        format!("Ação em massa concluída: {label}")
    } else {
        format!("Ação em massa falhou: {label}")
    }
}

#[async_trait]
impl EventConsumer for BulkNotificationConsumer {
    fn name(&self) -> &str {
        BULK_NOTIFICATIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == BULK_OPERATION_COMPLETED || event_type == BULK_OPERATION_FAILED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let payload = &event.payload;
        if event.event_type == BULK_OPERATION_COMPLETED
            && payload.get("background").and_then(Value::as_bool) != Some(true)
        {
            return Ok(());
        }
        let Some(requested_by) = payload
            .get("requested_by")
            .and_then(Value::as_str)
            .and_then(|id| UserId::new(id).ok())
        else {
            return Ok(());
        };
        let kind = if event.event_type == BULK_OPERATION_COMPLETED {
            BULK_OPERATION_FINISHED
        } else {
            BULK_OPERATION_FAILED_KIND
        };
        let record_type = payload
            .get("record_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let title = title(kind, record_type);
        let operation_id = payload.get("operation_id").and_then(Value::as_str);
        let intent = NotificationIntent::new(
            requested_by,
            kind,
            title.clone(),
            format!("event:{}:{}", event.id, kind.as_str()),
        )
        .with_link(Some(match operation_id {
            Some(id) => format!("{BULK_OPERATIONS_LINK}?operation={id}"),
            None => BULK_OPERATIONS_LINK.to_owned(),
        }))
        .with_data(json!({
            "title": title,
            "operation_id": operation_id,
            "record_type": record_type,
            "action": payload.get("action"),
            "total": payload.get("total"),
            "succeeded": payload.get("succeeded"),
            "skipped": payload.get("skipped"),
            "failed": payload.get("failed"),
        }));
        dispatch(&self.ports, intent).await?;
        Ok(())
    }
}
