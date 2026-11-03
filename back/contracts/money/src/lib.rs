use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, contract, extension_point,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
pub const CONTRACT: Contract = contract!("money", "1.0.0");
pub const AMOUNT: ExtensionPoint<MonetaryMapping> = extension_point!("money.amount");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "money.amount",
            KeyKind::ExtensionPoint,
            "Owner-declared monetary amount and conversion fields",
        ),
        KeyEntry::new(
            "money.recalculation",
            KeyKind::ExtensionPoint,
            "Owner-provided conversion recalculation",
        ),
    ],
};
pub const BASE_AMOUNT_FIELD: &str = "amount_base";
pub const BASE_AMOUNT_METRIC: &str = "sum:amount_base";
pub const RECALCULATE_JOB: &str = "money.recalculate";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyContext {
    pub base: String,
    pub timezone: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversion {
    pub base: String,
    pub currency: String,
    pub enabled: bool,
    pub rate: Option<Decimal>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetaryMapping {
    pub amount_field: String,
    pub currency_field: String,
    pub base_amount_field: String,
    pub exchange_rate_field: String,
}
pub fn convert_amount(amount: Decimal, rate: Decimal) -> Option<Decimal> {
    amount.checked_mul(rate).map(|value| value.round_dp(2))
}
pub fn base_amount(amount: Option<Decimal>, rate: Option<Decimal>) -> Option<Decimal> {
    amount
        .zip(rate)
        .and_then(|(amount, rate)| convert_amount(amount, rate))
}
