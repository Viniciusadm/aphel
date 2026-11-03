use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;

pub(crate) use aphel_sdk::money::{
    Conversion, CurrencyContext, convert_amount, currency_not_enabled, normalize_currency_code,
    organization_today,
};

pub(crate) const RATE_DECIMALS: u32 = 10;
pub(crate) const MAX_RATE_INTEGER_DIGITS: u32 = 14;
pub(crate) const MAX_ENABLED_CURRENCIES: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RateSource {
    Manual,
    Integration,
}

impl RateSource {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Integration => "integration",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "integration" => Some(Self::Integration),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeRate {
    pub id: Uuid,
    pub(crate) currency: String,
    pub(crate) effective_date: NaiveDate,
    pub rate: Decimal,
    pub(crate) source: RateSource,
    pub(crate) provider: Option<String>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

pub(crate) fn normalize_rate(value: Decimal) -> Option<Decimal> {
    let rate = value.round_dp(RATE_DECIMALS).normalize();
    let limit = Decimal::from(10_i64.pow(MAX_RATE_INTEGER_DIGITS));
    (rate > Decimal::ZERO && rate < limit).then_some(rate)
}

pub(crate) const CURRENCY_ENABLED: &str = "currency.enabled";
pub(crate) const CURRENCY_DISABLED: &str = "currency.disabled";
pub(crate) const EXCHANGE_RATE_CREATED: &str = "exchange_rate.created";
pub(crate) const EXCHANGE_RATE_UPDATED: &str = "exchange_rate.updated";
pub(crate) const EXCHANGE_RATE_DELETED: &str = "exchange_rate.deleted";
pub(crate) const EXCHANGE_RATES_REBASED: &str = "exchange_rate.rebased";
pub(crate) const RATES_PROVIDER_UPDATED: &str = "currency.provider_updated";

pub(crate) const EXCHANGE_RATE_NOT_FOUND: &str = "exchange_rate_not_found";
pub(crate) const EXCHANGE_RATE_EXISTS: &str = "exchange_rate_exists";
pub(crate) const RATES_PROVIDER_NOT_CONFIGURED: &str = "exchange_rate_provider_not_configured";
pub(crate) const RATES_PROVIDER_UNAVAILABLE: &str = "exchange_rate_provider_unavailable";

pub(crate) const MANUAL_PROVIDER: &str = "manual";
pub(crate) const FRANKFURTER_PROVIDER: &str = "frankfurter";
pub(crate) const FRANKFURTER_DEFAULT_URL: &str = "https://api.frankfurter.app";
pub(crate) const RATES_PROVIDERS: [&str; 2] = [MANUAL_PROVIDER, FRANKFURTER_PROVIDER];
pub(crate) const MAX_PROVIDER_URL_LENGTH: usize = 255;

pub const CURRENCIES_RECALCULATE_JOB: &str = "money.recalculate";
pub(crate) const EXCHANGE_RATES_SYNC_JOB: &str = "money.sync_rates";
pub(crate) const EXCHANGE_RATES_SYNC_CRON: &str = "0 15 6 * * *";

pub(crate) const EXCHANGE_RATE_RECORD_TYPE: RecordType =
    RecordType::from_static("money.exchange_rate");
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnabledCurrency {
    pub(crate) code: String,
    pub(crate) enabled_at: DateTime<Utc>,
    pub(crate) enabled_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyView {
    pub code: String,
    pub is_base: bool,
    pub(crate) enabled_at: Option<DateTime<Utc>>,
    pub(crate) current_rate: Option<ExchangeRate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencySettings {
    pub base: String,
    pub(crate) today: NaiveDate,
    pub currencies: Vec<CurrencyView>,
    pub(crate) provider: RatesProviderSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetCurrenciesCommand {
    pub currencies: Vec<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateExchangeRateCommand {
    pub currency: String,
    pub effective_date: Option<String>,
    pub rate: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateExchangeRateCommand {
    pub(crate) effective_date: Option<String>,
    pub(crate) rate: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExchangeRateFilter {
    pub(crate) currency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RatesProviderSettings {
    pub(crate) provider: String,
    pub(crate) url: Option<String>,
    pub(crate) last_synced_at: Option<DateTime<Utc>>,
}

impl Default for RatesProviderSettings {
    fn default() -> Self {
        Self {
            provider: MANUAL_PROVIDER.to_owned(),
            url: None,
            last_synced_at: None,
        }
    }
}

impl RatesProviderSettings {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateRatesProviderCommand {
    pub(crate) provider: String,
    pub(crate) url: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncOutcome {
    pub(crate) provider: String,
    pub(crate) effective_date: NaiveDate,
    pub(crate) created: Vec<String>,
    pub(crate) updated: Vec<String>,
    pub(crate) unchanged: Vec<String>,
    pub(crate) kept_manual: Vec<String>,
    pub(crate) failed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RebaseOutcome {
    pub(crate) previous_base: String,
    pub(crate) base: String,
    pub(crate) converted: usize,
    pub(crate) created: usize,
    pub(crate) removed: usize,
}
