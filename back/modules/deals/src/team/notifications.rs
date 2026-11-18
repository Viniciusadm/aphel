use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::platform::database::Db;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::team::domain::DEAL_MEMBER_ADDED;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationClient;
use aphel_sdk::notifications::{NotificationIntent, dispatch};
use aphel_sdk::notifications::{NotificationKind, NotificationKindDefinition};
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::{Actor, DomainEvent, EventConsumer, RecordType};
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;

pub(crate) const DEAL_TEAM_NOTIFICATIONS_CONSUMER: &str = "deals.team_notifications";
pub(crate) const DEAL_TEAM_MEMBER_ADDED: NotificationKind =
    NotificationKind::from_static("deal_team_member_added");
pub(crate) const DEAL_TEAM_NOTIFICATION_KIND: NotificationKindDefinition =
    NotificationKindDefinition::new(DEAL_TEAM_MEMBER_ADDED).push_by_default();

pub(crate) struct DealTeamNotificationConsumer {
    pool: Db,
    ports: NotificationClient,
}

impl DealTeamNotificationConsumer {
    pub(crate) fn shared(pool: Db) -> Arc<dyn EventConsumer> {
        Arc::new(Self {
            ports: NotificationClient::new(
                pool.clone(),
                Arc::new(RecordTypeRegistry::builtin()),
                Arc::new(MySqlJobQueue::new(pool.clone())),
            ),
            pool,
        })
    }

    async fn deal_title(&self, deal_id: &str) -> Result<Option<String>, AppError> {
        crate::store::deal_rows::title(&self.pool, deal_id).await
    }
}

fn text<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

#[async_trait]
impl EventConsumer for DealTeamNotificationConsumer {
    fn name(&self) -> &str {
        DEAL_TEAM_NOTIFICATIONS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == DEAL_MEMBER_ADDED
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
        let Some(recipient) = text(payload, "user_id").and_then(|id| UserId::new(id).ok()) else {
            return Ok(());
        };
        let deal_id = record.record_id.to_string();
        let Some(title) = self.deal_title(&deal_id).await? else {
            return Ok(());
        };
        let actor = match &event.actor {
            Actor::User(id) => Some(id.clone()),
            _ => None,
        };
        let role = text(payload, "role_name").map(str::to_owned);
        let intent = NotificationIntent::new(
            recipient,
            DEAL_TEAM_MEMBER_ADDED,
            format!("Você entrou na equipe da oportunidade {title}"),
            format!("event:{}:{}", event.id, DEAL_TEAM_MEMBER_ADDED.as_str()),
        )
        .with_record(Some(record))
        .with_actor(actor)
        .with_body(role.clone())
        .with_data(json!({
            "deal_id": deal_id,
            "deal_title": title,
            "role_name": role,
            "credit_percent": text(payload, "credit_percent"),
        }));
        dispatch(&self.ports, intent).await?;
        Ok(())
    }
}
