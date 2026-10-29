use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::domain::{Conversion, organization_today};
use crate::store::CurrencyStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

#[async_trait]
pub trait CurrencyConverter: Send + Sync {
    async fn conversion(&self, ctx: &Ctx<'_>, currency: &str) -> Result<Conversion, AppError>;
}

pub struct RateCurrencyConverter;

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
