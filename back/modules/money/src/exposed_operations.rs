use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::{Conversion, convert_amount};
use crate::hooks::CurrencyConverter;
use crate::service::require_reader;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{OperationFuture, operation_input};

#[derive(Deserialize)]
struct ConvertInput {
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    amount: Option<Value>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid conversion",
        vec![FieldViolation::new(field, message.to_owned())],
    )
}

fn currency_code(value: &str, field: &str) -> Result<String, AppError> {
    let code = value.trim().to_ascii_uppercase();
    if code.len() == 3 && code.chars().all(|character| character.is_ascii_uppercase()) {
        Ok(code)
    } else {
        Err(invalid(field, "must be a 3-letter ISO currency code"))
    }
}

fn parse_amount(value: &Value) -> Result<Decimal, AppError> {
    let text = match value {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.trim().to_owned(),
        _ => return Err(invalid("amount", "must be a number")),
    };
    Decimal::from_str(&text)
        .or_else(|_| Decimal::from_scientific(&text))
        .map_err(|_| invalid("amount", "must be a number"))
}

fn rate_info(conversion: &Conversion) -> Value {
    json!({
        "base": conversion.base,
        "currency": conversion.currency,
        "enabled": conversion.enabled,
        "rate": conversion.rate.map(|rate| rate.to_string()),
    })
}

fn convert<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ConvertInput = operation_input(input)?;
        require_reader(ctx.scope)?;
        let converter = ctx.services().require::<dyn CurrencyConverter>()?;
        let Some(amount) = input.amount else {
            let currency = input
                .currency
                .or(input.from)
                .ok_or_else(|| invalid("currency", "is required"))?;
            let conversion = converter
                .conversion(ctx, &currency_code(&currency, "currency")?)
                .await?;
            return Ok(rate_info(&conversion));
        };
        let amount = parse_amount(&amount)?;
        let from = input
            .from
            .or(input.currency)
            .ok_or_else(|| invalid("from", "is required"))?;
        let from = converter
            .conversion(ctx, &currency_code(&from, "from")?)
            .await?;
        let to = match input.to {
            Some(to) => {
                converter
                    .conversion(ctx, &currency_code(&to, "to")?)
                    .await?
            }
            None => converter.conversion(ctx, &from.base).await?,
        };
        let rate = match (from.rate, to.rate) {
            (Some(from_rate), Some(to_rate)) if !to_rate.is_zero() => {
                from_rate.checked_div(to_rate)
            }
            _ => None,
        };
        let converted = rate.and_then(|rate| convert_amount(amount, rate));
        Ok(json!({
            "amount": amount.to_string(),
            "from": rate_info(&from),
            "to": rate_info(&to),
            "base": from.base,
            "rate": rate.map(|rate| rate.normalize().to_string()),
            "converted": converted.map(|value| value.to_string()),
        }))
    })
}

const OP_CURRENCIES_CONVERT: OperationKey<Value, Value> = operation!("currencies.convert");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_CURRENCIES_CONVERT, OperationSpec::read("Today's exchange rate of a currency to the base currency: { currency } -> { base, currency, enabled, rate }; or convert an amount: { amount, from, to? (default: base currency) } -> { amount, from, to, base, rate, converted } (decimals as strings, rate/converted null when a rate is missing)"), convert);
}
