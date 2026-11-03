use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::domain::{
    FRANKFURTER_DEFAULT_URL, FRANKFURTER_PROVIDER, RATES_PROVIDER_UNAVAILABLE,
    RatesProviderSettings,
};
use aphel_kernel::errors::AppError;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[async_trait]
pub(crate) trait ExchangeRateProvider: Send + Sync {
    fn key(&self) -> &str;
    async fn rate(&self, base: &str, currency: &str) -> Result<Decimal, AppError>;
}

pub(crate) fn configured_provider(
    settings: &RatesProviderSettings,
) -> Option<Arc<dyn ExchangeRateProvider>> {
    match settings.provider.as_str() {
        FRANKFURTER_PROVIDER => Some(Arc::new(FrankfurterProvider::new(
            settings
                .url
                .as_deref()
                .filter(|url| !url.trim().is_empty())
                .unwrap_or(FRANKFURTER_DEFAULT_URL),
        ))),
        _ => None,
    }
}

fn unavailable(message: impl Into<String>) -> AppError {
    AppError::unavailable_code(RATES_PROVIDER_UNAVAILABLE, message)
}

fn client() -> Result<&'static Client, AppError> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|_| {
            AppError::Internal("could not build the exchange rate HTTP client".to_owned())
        })?;
    Ok(CLIENT.get_or_init(|| client))
}

pub(crate) struct FrankfurterProvider {
    base_url: String,
}

impl FrankfurterProvider {
    pub(crate) fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim().trim_end_matches('/').to_owned(),
        }
    }
}

#[async_trait]
impl ExchangeRateProvider for FrankfurterProvider {
    fn key(&self) -> &str {
        FRANKFURTER_PROVIDER
    }

    async fn rate(&self, base: &str, currency: &str) -> Result<Decimal, AppError> {
        let response = client()?
            .get(format!("{}/latest", self.base_url))
            .query(&[("from", currency), ("to", base)])
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "Aphel-Money/1.0")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    unavailable("the exchange rate provider did not answer in time")
                } else {
                    unavailable("could not reach the exchange rate provider")
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(unavailable(format!(
                "the exchange rate provider answered {status} for {currency}"
            )));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| unavailable("the exchange rate provider sent an invalid answer"))?;
        body.get("rates")
            .and_then(|rates| rates.get(base))
            .and_then(|rate| match rate {
                Value::Number(number) => Decimal::from_str(&number.to_string())
                    .ok()
                    .or_else(|| Decimal::from_scientific(&number.to_string()).ok()),
                Value::String(text) => Decimal::from_str(text).ok(),
                _ => None,
            })
            .ok_or_else(|| {
                unavailable(format!(
                    "the exchange rate provider has no rate from {currency} to {base}"
                ))
            })
    }
}
