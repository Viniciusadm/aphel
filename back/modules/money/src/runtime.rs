use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::{EXCHANGE_RATES_SYNC_CRON, EXCHANGE_RATES_SYNC_JOB};
use crate::provider::configured_provider;
use crate::service::{RatePorts, recalculation_job};
use crate::sync::{rebase, sync_rates};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{
    Actor, DomainEvent, EventConsumer, JobContext, JobHandler, JobQueue,
};
use aphel_sdk::platform::database::Db;

pub(crate) const CURRENCIES_BASE_CONSUMER: &str = "money.base_currency";
const ORGANIZATION_UPDATED: &str = "organization.updated";

pub(crate) fn exchange_rates_sync_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(EXCHANGE_RATES_SYNC_JOB, EXCHANGE_RATES_SYNC_CRON)
        .expect("the exchange rates sync cron expression is valid")
        .with_max_attempts(2)
}

pub(crate) struct ExchangeRatesSyncHandler {
    ports: RatePorts,
}

impl ExchangeRatesSyncHandler {
    pub(crate) fn new(pool: Db) -> Self {
        Self {
            ports: RatePorts::system(pool),
        }
    }
}

#[async_trait]
impl JobHandler for ExchangeRatesSyncHandler {
    fn kind(&self) -> &str {
        EXCHANGE_RATES_SYNC_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        rebase(&self.ports, &Actor::System).await?;
        let settings = self.ports.store.provider_settings().await?;
        let Some(provider) = configured_provider(&settings) else {
            return Ok(());
        };
        if self.ports.store.find_context().await?.is_none() {
            return Ok(());
        }
        let outcome = sync_rates(&self.ports, &Actor::System, None, provider.as_ref()).await?;
        if !outcome.failed.is_empty() {
            tracing::warn!(
                provider = %outcome.provider,
                failed = ?outcome.failed,
                "some exchange rates could not be fetched"
            );
        }
        Ok(())
    }
}

pub(crate) fn exchange_rates_sync_handler(pool: Db) -> Arc<dyn JobHandler> {
    Arc::new(ExchangeRatesSyncHandler::new(pool))
}

pub(crate) struct BaseCurrencyConsumer {
    ports: RatePorts,
}

impl BaseCurrencyConsumer {
    pub(crate) fn new(pool: Db) -> Self {
        Self {
            ports: RatePorts::system(pool),
        }
    }
}

#[async_trait]
impl EventConsumer for BaseCurrencyConsumer {
    fn name(&self) -> &str {
        CURRENCIES_BASE_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        event_type == ORGANIZATION_UPDATED
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let currency_changed = event
            .payload
            .get("changed_fields")
            .and_then(Value::as_array)
            .is_some_and(|fields| fields.iter().any(|field| field == "currency"));
        if !currency_changed {
            return Ok(());
        }
        if rebase(&self.ports, &event.actor).await?.is_none() {
            self.ports.jobs.enqueue_now(recalculation_job(None)).await?;
        }
        Ok(())
    }
}

pub(crate) struct MoneyRecalculateHandler {
    pool: Db,
}
impl MoneyRecalculateHandler {
    pub(crate) fn new(pool: Db) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl JobHandler for MoneyRecalculateHandler {
    fn kind(&self) -> &str {
        aphel_sdk::money::RECALCULATE_JOB
    }
    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let currency = payload
            .get("currency")
            .and_then(Value::as_str)
            .and_then(aphel_sdk::money::normalize_currency_code);
        for source in aphel_sdk::platform::modules::app()
            .contributions::<dyn aphel_sdk::money::MoneyRecalculationSource>()
        {
            source.recalculate(&self.pool, currency.as_deref()).await?;
        }
        Ok(())
    }
}

pub(crate) struct MoneyRebaseCompletedHandler {
    pool: Db,
}
impl MoneyRebaseCompletedHandler {
    pub(crate) fn new(pool: Db) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl JobHandler for MoneyRebaseCompletedHandler {
    fn kind(&self) -> &str {
        aphel_sdk::money::REBASE_COMPLETED_JOB
    }
    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(factor) = payload
            .get("factor")
            .and_then(Value::as_str)
            .and_then(|factor| factor.parse::<rust_decimal::Decimal>().ok())
        else {
            return Ok(());
        };
        for source in aphel_sdk::platform::modules::app()
            .contributions::<dyn aphel_sdk::money::MoneyRecalculationSource>()
        {
            source.rebase_completed(&self.pool, factor).await?;
        }
        Ok(())
    }
}
