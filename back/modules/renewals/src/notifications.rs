use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::domain::DEAL_RENEWAL_CREATED;
use ::notifications::application::{NotificationIntent, dispatch};
use ::notifications::domain::{NotificationKind, NotificationKindDefinition};
use ::notifications::infrastructure::MySqlNotificationPorts;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, RecordType};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;

pub(crate) const RENEWAL_NOTIFICATIONS_CONSUMER: &str = "renewals.notifications";
pub(crate) const RENEWAL_CREATED: NotificationKind =
    NotificationKind::from_static("renewal_created");
pub(crate) const RENEWAL_NOTIFICATION_KIND: NotificationKindDefinition =
    NotificationKindDefinition::new(RENEWAL_CREATED).email_by_default();

pub(crate) struct RenewalNotificationConsumer {
    pool: MySqlPool,
    ports: MySqlNotificationPorts,
}

impl RenewalNotificationConsumer {
    pub(crate) fn shared(pool: MySqlPool) -> Arc<dyn EventConsumer> {
        Arc::new(Self {
            ports: MySqlNotificationPorts::new(
                pool.clone(),
                Arc::new(RecordTypeRegistry::builtin()),
                Arc::new(MySqlJobQueue::new(pool.clone())),
            ),
            pool,
        })
    }

    async fn renewal(&self, deal_id: &str) -> Result<Option<(String, Option<String>)>, AppError> {
        Ok(
            sqlx::query_as("SELECT title, owner_id FROM deals WHERE id = ? AND deleted_at IS NULL")
                .bind(deal_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}

fn text<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

#[async_trait]
impl EventConsumer for RenewalNotificationConsumer {
    fn name(&self) -> &str {
        RENEWAL_NOTIFICATIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == DEAL_RENEWAL_CREATED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = event
            .record
            .clone()
            .filter(|record| record.record_type == RecordType::DEAL)
        else {
            return Ok(());
        };
        let payload = &event.payload;
        let deal_id = record.record_id.to_string();
        let Some((title, owner)) = self.renewal(&deal_id).await? else {
            return Ok(());
        };
        let Some(recipient) = owner
            .as_deref()
            .or_else(|| text(payload, "owner_id"))
            .and_then(|id| UserId::new(id).ok())
        else {
            return Ok(());
        };
        let contract_end = text(payload, "contract_end").map(str::to_owned);
        let body = contract_end
            .as_deref()
            .map(|date| format!("O contrato termina em {date}"));
        let intent = NotificationIntent::new(
            recipient,
            RENEWAL_CREATED,
            format!("Renovação criada: {title}"),
            format!("event:{}:{}", event.id, RENEWAL_CREATED.as_str()),
        )
        .with_record(Some(record))
        .with_body(body)
        .with_data(json!({
            "renewal_deal_id": deal_id,
            "original_deal_id": text(payload, "original_deal_id"),
            "original_title": text(payload, "original_title"),
            "contract_end": contract_end,
        }));
        dispatch(&self.ports, intent).await?;
        Ok(())
    }
}
