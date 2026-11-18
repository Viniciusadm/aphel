use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::domain::{Conversion, organization_today};
use crate::store::CurrencyStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

pub(crate) use aphel_sdk::money::CurrencyConverter;

pub(crate) struct RateCurrencyConverter;

#[async_trait]
impl CurrencyConverter for RateCurrencyConverter {
    async fn conversion(&self, ctx: &Ctx<'_>, currency: &str) -> Result<Conversion, AppError> {
        let store = CurrencyStore::of(ctx);
        let context = store.context().await?;
        if context.base == currency {
            return Ok(Conversion {
                base: context.base,
                currency: currency.to_owned(),
                enabled: true,
                rate: Some(Decimal::ONE),
            });
        }
        let enabled = store.is_enabled(currency).await?;
        let today = organization_today(&context.timezone, ctx.now());
        let rate = store.rate_on(currency, today).await?.map(|rate| rate.rate);
        Ok(Conversion {
            base: context.base,
            currency: currency.to_owned(),
            enabled,
            rate,
        })
    }
}

#[async_trait]
impl aphel_sdk::money::MoneyRates for RateCurrencyConverter {
    async fn currency_enabled(&self, pool: &Db, currency: &str) -> Result<bool, AppError> {
        CurrencyStore::new(pool.clone()).is_enabled(currency).await
    }
    async fn context(
        &self,
        pool: &Db,
    ) -> Result<Option<aphel_sdk::money::CurrencyContext>, AppError> {
        CurrencyStore::new(pool.clone()).find_context().await
    }
    async fn rate_on(
        &self,
        pool: &Db,
        currency: &str,
        date: chrono::NaiveDate,
    ) -> Result<Option<Decimal>, AppError> {
        Ok(CurrencyStore::new(pool.clone())
            .rate_on(currency, date)
            .await?
            .map(|rate| rate.rate))
    }
}
