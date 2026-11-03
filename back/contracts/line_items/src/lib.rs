use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, contract, extension_point,
};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;
pub const CONTRACT: Contract = contract!("line-items", "1.0.0");
pub const ATTACHABLE: ExtensionPoint<LineItemMapping> = extension_point!("line_items.attachable");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "line_items.attachable",
            KeyKind::ExtensionPoint,
            "Currency and amount fields of line-item hosts",
        ),
        KeyEntry::new(
            "line_items.totals_changed",
            KeyKind::Hook(aphel_contract::HookKind::Action),
            "Line changes in their owner's transaction",
        ),
    ],
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItemMapping {
    pub currency_field: String,
    pub amount_field: Option<String>,
    pub exchange_rate_field: Option<String>,
}
pub const MAX_TERM_MONTHS: u16 = 600;

pub fn parse_term_months(text: &str) -> Option<u16> {
    let text = text.trim();
    let value = text.parse::<u16>().ok().or_else(|| {
        text.parse::<f64>()
            .ok()
            .filter(|value| value.fract() == 0.0 && *value >= 0.0 && *value <= f64::from(u16::MAX))
            .map(|value| value as u16)
    })?;
    (1..=MAX_TERM_MONTHS).contains(&value).then_some(value)
}

pub fn term_violation() -> String {
    format!("must be a whole number of months between 1 and {MAX_TERM_MONTHS}")
}

pub const MAX_UNIT_LENGTH: usize = 32;
pub const MONEY_DECIMALS: u32 = 2;
pub const MAX_MONEY_DIGITS: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    #[default]
    None,
    Monthly,
    Quarterly,
    Yearly,
}

impl Recurrence {
    pub const ALL: [Self; 4] = [Self::None, Self::Monthly, Self::Quarterly, Self::Yearly];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::Yearly => "yearly",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|recurrence| recurrence.as_str() == value)
    }
}

pub fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub fn round_money(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(MONEY_DECIMALS, RoundingStrategy::MidpointAwayFromZero)
}

pub fn money_limit() -> Decimal {
    Decimal::from(10_i64.pow(MAX_MONEY_DIGITS))
}

pub fn normalize_money(value: Decimal) -> Option<Decimal> {
    let amount = round_money(value);
    (amount >= Decimal::ZERO && amount < money_limit()).then_some(amount)
}

pub const MAX_ITEM_NAME_LENGTH: usize = 255;
pub const MAX_LINES_PER_RECORD: usize = 200;
pub const QUANTITY_DECIMALS: u32 = 4;
pub const MAX_QUANTITY_DIGITS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscountKind {
    #[default]
    None,
    Percent,
    Amount,
}

impl DiscountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Percent => "percent",
            Self::Amount => "amount",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "percent" => Some(Self::Percent),
            "amount" => Some(Self::Amount),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountMode {
    #[default]
    Manual,
    Items,
}

impl AmountMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Items => "items",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "items" => Some(Self::Items),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LineData {
    pub product_id: Option<Uuid>,
    pub name: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub discount_kind: DiscountKind,
    pub discount_value: Decimal,
    #[serde(default)]
    pub tax_percent: Decimal,
    pub recurrence: Recurrence,
    pub term_months: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineTotalError {
    PercentOutOfRange,
    TaxOutOfRange,
    TooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineAmounts {
    pub gross: Decimal,
    pub discount: Decimal,
    pub net: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
}

impl LineData {
    pub fn amounts(&self) -> Result<LineAmounts, LineTotalError> {
        let gross = self
            .quantity
            .checked_mul(self.unit_price)
            .map(round_money)
            .filter(|gross| *gross < money_limit())
            .ok_or(LineTotalError::TooLarge)?;
        if self.quantity <= Decimal::ZERO
            || self.unit_price < Decimal::ZERO
            || self.discount_value < Decimal::ZERO
        {
            return Err(LineTotalError::TooLarge);
        }
        let discount = line_discount(gross, self.discount_kind, self.discount_value)
            .ok_or(LineTotalError::PercentOutOfRange)?;
        if !(Decimal::ZERO..=Decimal::ONE_HUNDRED).contains(&self.tax_percent) {
            return Err(LineTotalError::TaxOutOfRange);
        }
        let net = gross - discount;
        let tax = round_money(net * self.tax_percent / Decimal::ONE_HUNDRED);
        let total = net
            .checked_add(tax)
            .filter(|total| *total < money_limit())
            .ok_or(LineTotalError::TooLarge)?;
        Ok(LineAmounts {
            gross,
            discount,
            net,
            tax,
            total,
        })
    }
}

pub fn quantity_limit() -> Decimal {
    Decimal::from(10_i64.pow(MAX_QUANTITY_DIGITS))
}

pub fn normalize_quantity(value: Decimal) -> Option<Decimal> {
    let quantity =
        value.round_dp_with_strategy(QUANTITY_DECIMALS, RoundingStrategy::MidpointAwayFromZero);
    (quantity > Decimal::ZERO && quantity < quantity_limit()).then(|| quantity.normalize())
}

pub fn parse_money(text: &str) -> Option<Decimal> {
    let text = text.trim();
    Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())
        .and_then(normalize_money)
}

pub fn line_discount(gross: Decimal, kind: DiscountKind, value: Decimal) -> Option<Decimal> {
    if gross < Decimal::ZERO || value < Decimal::ZERO {
        return None;
    }
    let discount = match kind {
        DiscountKind::None => Decimal::ZERO,
        DiscountKind::Percent if value > Decimal::ONE_HUNDRED => return None,
        DiscountKind::Percent => round_money(gross.checked_mul(value)? / Decimal::ONE_HUNDRED),
        DiscountKind::Amount => value,
    };
    Some(discount.min(gross))
}
