use crate::{AppError, Ctx, fields::EntityTraits};
pub use aphel_contract_money::*;
pub use aphel_kernel::application::shared::normalize_currency_code;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::MySqlPool;
pub const KEY: &str = "money";
pub const READ_PERMISSION: crate::access::Permission =
    crate::access::Permission::from_static("money.read");
#[async_trait]
pub trait CurrencyConverter: Send + Sync {
    async fn conversion(&self, ctx: &Ctx<'_>, currency: &str) -> Result<Conversion, AppError>;
}
#[async_trait]
pub trait MoneyRates: Send + Sync {
    async fn currency_enabled(&self, pool: &MySqlPool, currency: &str) -> Result<bool, AppError>;
    async fn context(&self, pool: &MySqlPool) -> Result<Option<CurrencyContext>, AppError>;
    async fn rate_on(
        &self,
        pool: &MySqlPool,
        currency: &str,
        date: NaiveDate,
    ) -> Result<Option<Decimal>, AppError>;
}
#[async_trait]
pub trait MoneyRecalculationSource: Send + Sync {
    async fn recalculate(&self, pool: &MySqlPool, currency: Option<&str>) -> Result<u64, AppError>;
    async fn rebase_completed(&self, _pool: &MySqlPool, _factor: Decimal) -> Result<u64, AppError> {
        Ok(0)
    }
}
pub trait MoneyTraits {
    fn monetary(self, mapping: MonetaryMapping) -> Self;
}
impl MoneyTraits for EntityTraits {
    fn monetary(self, mapping: MonetaryMapping) -> Self {
        self.with_trait_data(
            AMOUNT.key(),
            serde_json::to_value(mapping).expect("monetary mapping"),
        )
    }
}
pub fn organization_today(timezone: &str, now: DateTime<Utc>) -> NaiveDate {
    match timezone.parse::<chrono_tz::Tz>() {
        Ok(zone) => now.with_timezone(&zone).date_naive(),
        Err(_) => now.date_naive(),
    }
}
pub fn currency_not_enabled(currency: &str) -> AppError {
    AppError::invalid_fields_code(
        "currency_not_enabled",
        "the currency is not enabled for the organization",
        vec![aphel_kernel::errors::FieldViolation::new(
            "currency",
            format!("{currency} is not one of the enabled currencies"),
        )],
    )
}
pub fn base_currency_fields() -> Result<Vec<crate::fields::SystemField>, AppError> {
    use aphel_kernel::application::fields::{LocalizedLabels, builtin::field};
    use aphel_kernel::domain::fields::FieldType;
    Ok(vec![
        field(
            BASE_AMOUNT_FIELD,
            LocalizedLabels(std::collections::BTreeMap::from([
                ("en".into(), "Base currency amount".into()),
                ("pt-BR".into(), "Valor na moeda base".into()),
            ])),
            FieldType::Currency,
        )?,
        field(
            "exchange_rate",
            LocalizedLabels(std::collections::BTreeMap::from([
                ("en".into(), "Exchange rate".into()),
                ("pt-BR".into(), "Taxa de câmbio".into()),
            ])),
            FieldType::Number,
        )?,
    ])
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
#[schema(as = DecimalInput, example = "15000.00")]
pub enum DecimalInputSchema {
    Text(String),
    Number(f64),
}
