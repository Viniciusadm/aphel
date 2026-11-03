use aphel_contract_crm::CrmRecordTypes;
use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::{DEAL_CREATED, DEAL_STAGE_CHANGED, StageTransitionEntry};
use crate::hooks::AmountCalculator;
use crate::store::DealStore;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::{DomainEvent, EventConsumer, RecordType};
use aphel_kernel::errors::AppError;
use aphel_sdk::money::MoneyRates;
use aphel_sdk::money::organization_today;

pub(crate) const DEALS_LAST_ACTIVITY_CONSUMER: &str = "deals.last_activity";
pub(crate) const DEAL_STAGE_HISTORY_CONSUMER: &str = "reports.deal_stage_history";

pub(crate) const TRACKED_ACTIVITY_EVENTS: [&str; 4] = [
    "activity.created",
    "activity.updated",
    "activity.completed",
    "activity.reopened",
];

fn store(pool: MySqlPool) -> DealStore {
    DealStore::new(pool, Arc::new(RecordTypeRegistry::builtin()))
}

fn deal_reference(value: &Value) -> Option<Uuid> {
    let is_deal =
        value.get("record_type").and_then(Value::as_str) == Some(RecordType::DEAL.as_str());
    if !is_deal {
        return None;
    }
    value
        .get("record_id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

pub(crate) fn linked_deals(payload: &Value) -> BTreeSet<Uuid> {
    let mut deals: BTreeSet<Uuid> = deal_reference(payload).into_iter().collect();
    if let Some(links) = payload.get("links").and_then(Value::as_array) {
        deals.extend(links.iter().filter_map(deal_reference));
    }
    deals
}

pub(crate) fn activity_instant(event: &DomainEvent) -> DateTime<Utc> {
    event
        .payload
        .get("occurred_at")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(event.occurred_at)
}

pub(crate) struct DealLastActivityConsumer {
    deals: DealStore,
}

impl DealLastActivityConsumer {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { deals: store(pool) }
    }
}

#[async_trait]
impl EventConsumer for DealLastActivityConsumer {
    fn name(&self) -> &str {
        DEALS_LAST_ACTIVITY_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        TRACKED_ACTIVITY_EVENTS.contains(&event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let at = activity_instant(event);
        for deal_id in linked_deals(&event.payload) {
            self.deals.touch_last_activity(deal_id, at).await?;
        }
        Ok(())
    }
}

fn stage_id(payload: &Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(|stage| stage.get("id"))
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn event_deal_id(event: &DomainEvent) -> Option<Uuid> {
    event
        .record
        .as_ref()
        .filter(|record| record.record_type == RecordType::DEAL)
        .map(|record| record.record_id)
}

pub(crate) struct DealStageHistoryConsumer {
    deals: DealStore,
}

impl DealStageHistoryConsumer {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { deals: store(pool) }
    }
}

#[async_trait]
impl EventConsumer for DealStageHistoryConsumer {
    fn name(&self) -> &str {
        DEAL_STAGE_HISTORY_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == DEAL_CREATED || event_type == DEAL_STAGE_CHANGED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(deal_id) = event_deal_id(event) else {
            return Ok(());
        };
        if event.event_type == DEAL_CREATED {
            if let Some(stage) = stage_id(&event.payload, "stage") {
                self.deals.record_initial_stage(deal_id, stage).await?;
            }
            return Ok(());
        }
        let Some(stage) = stage_id(&event.payload, "to") else {
            return Ok(());
        };
        let from_stage_id = stage_id(&event.payload, "from");
        if let Some(from) = from_stage_id {
            self.deals.record_initial_stage(deal_id, from).await?;
        }
        self.deals
            .record_stage_transition(&StageTransitionEntry {
                event_id: event.id,
                deal_id,
                from_stage_id,
                stage_id: stage,
                entered_at: event.occurred_at,
            })
            .await
    }
}

pub(crate) struct CurrencyRecalculateHandler {
    pool: MySqlPool,
}

impl CurrencyRecalculateHandler {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn recalculate(&self, currency: Option<&str>) -> Result<u64, AppError> {
        let currencies = aphel_kernel::modules::app()
            .services()
            .require::<dyn MoneyRates>()?;
        let Some(context) = currencies.context(&self.pool).await? else {
            return Ok(0);
        };
        let deals = store(self.pool.clone());
        let today = organization_today(&context.timezone, Utc::now());
        let codes = match currency {
            Some(currency) => vec![currency.to_owned()],
            None => deals.open_currencies().await?,
        };
        let calculator = aphel_kernel::modules::app()
            .services()
            .require::<dyn AmountCalculator>()?;
        let mut updated = 0;
        for code in codes {
            let rate = if code == context.base {
                Some(Decimal::ONE)
            } else {
                currencies.rate_on(&self.pool, &code, today).await?
            };
            let mut tx =
                aphel_kernel::database::transaction::MySqlTx::from(self.pool.begin().await?);
            updated += deals.apply_open_rate(&mut tx, &code, rate).await?;
            calculator.sync_currency_rate(&mut tx, &code, rate).await?;
            tx.commit().await?;
        }
        Ok(updated)
    }
}

pub(crate) struct DealRecalculation;
#[async_trait]
impl aphel_sdk::money::MoneyRecalculationSource for DealRecalculation {
    async fn recalculate(&self, pool: &MySqlPool, currency: Option<&str>) -> Result<u64, AppError> {
        CurrencyRecalculateHandler::new(pool.clone())
            .recalculate(currency)
            .await
    }
}
