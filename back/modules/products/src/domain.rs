use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::decimal_json;
use aphel_kernel::application::fields::permissions::provided_key;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    InputValidator, MAX_LONG_TEXT_LENGTH, Patch, bool_patch, insert_text, permission_denied,
    value_patch,
};

pub use aphel_kernel::application::shared::normalize_currency_code as normalize_currency;

pub(crate) const MAX_TERM_MONTHS: u16 = 600;

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

pub(crate) const MAX_PRODUCT_NAME_LENGTH: usize = 255;
pub(crate) const MAX_PRODUCT_CODE_LENGTH: usize = 64;
pub const MAX_PRODUCT_UNIT_LENGTH: usize = 32;
pub const MONEY_DECIMALS: u32 = 2;
pub(crate) const MAX_MONEY_DIGITS: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProductRecurrence {
    #[default]
    None,
    Monthly,
    Quarterly,
    Yearly,
}

impl ProductRecurrence {
    pub(crate) const ALL: [Self; 4] = [Self::None, Self::Monthly, Self::Quarterly, Self::Yearly];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductData {
    pub name: String,
    pub(crate) code: Option<String>,
    pub(crate) description: Option<String>,
    pub price: Decimal,
    pub currency: String,
    pub unit: Option<String>,
    pub recurrence: ProductRecurrence,
    pub term_months: Option<u16>,
    pub is_active: bool,
}

impl Default for ProductData {
    fn default() -> Self {
        Self {
            name: String::new(),
            code: None,
            description: None,
            price: Decimal::ZERO,
            currency: String::new(),
            unit: None,
            recurrence: ProductRecurrence::None,
            term_months: None,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: Uuid,
    pub data: ProductData,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
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
pub const MAX_ITEMS_PER_DEAL: usize = 200;
pub const QUANTITY_DECIMALS: u32 = 4;
pub(crate) const MAX_QUANTITY_DIGITS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DealAmountMode {
    #[default]
    Manual,
    Items,
}

impl DealAmountMode {
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DealItemData {
    pub product_id: Option<Uuid>,
    pub name: String,
    pub unit: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub discount_kind: DiscountKind,
    pub discount_value: Decimal,
    pub recurrence: ProductRecurrence,
    pub term_months: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineTotalError {
    PercentOutOfRange,
    TooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineAmounts {
    pub gross: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
}

impl DealItemData {
    pub fn amounts(&self) -> Result<LineAmounts, LineTotalError> {
        let gross = self
            .quantity
            .checked_mul(self.unit_price)
            .map(round_money)
            .filter(|gross| *gross < money_limit())
            .ok_or(LineTotalError::TooLarge)?;
        let discount = match self.discount_kind {
            DiscountKind::None => Decimal::ZERO,
            DiscountKind::Percent => {
                if self.discount_value > Decimal::ONE_HUNDRED {
                    return Err(LineTotalError::PercentOutOfRange);
                }
                round_money(gross * self.discount_value / Decimal::ONE_HUNDRED)
            }
            DiscountKind::Amount => self.discount_value,
        };
        let discount = discount.min(gross);
        Ok(LineAmounts {
            gross,
            discount,
            total: gross - discount,
        })
    }
}

pub(crate) fn quantity_limit() -> Decimal {
    Decimal::from(10_i64.pow(MAX_QUANTITY_DIGITS))
}

pub fn normalize_quantity(value: Decimal) -> Option<Decimal> {
    let quantity =
        value.round_dp_with_strategy(QUANTITY_DECIMALS, RoundingStrategy::MidpointAwayFromZero);
    (quantity > Decimal::ZERO && quantity < quantity_limit()).then(|| quantity.normalize())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealItem {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub data: DealItemData,
    pub total: Decimal,
    pub amount_base: Option<Decimal>,
    pub exchange_rate: Option<Decimal>,
    pub position: i32,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn items_total(items: &[DealItem]) -> Decimal {
    items.iter().map(|item| item.total).sum()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProductInput {
    pub name: Patch<String>,
    pub code: Patch<String>,
    pub description: Patch<String>,
    pub price: Patch<String>,
    pub currency: Patch<String>,
    pub unit: Patch<String>,
    pub recurrence: Patch<String>,
    pub term_months: Patch<String>,
    pub is_active: Patch<bool>,
}

impl ProductInput {
    pub(crate) fn from_values(values: &Map<String, Value>) -> Self {
        Self {
            name: value_patch(values, "name"),
            code: value_patch(values, "code"),
            description: value_patch(values, "description"),
            price: value_patch(values, "price"),
            currency: value_patch(values, "currency"),
            unit: value_patch(values, "unit"),
            recurrence: value_patch(values, "recurrence"),
            term_months: value_patch(values, "term_months"),
            is_active: bool_patch(values, "is_active"),
        }
    }

    pub(crate) fn provided_keys(&self) -> Vec<&'static str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "name", self.name.is_some());
        provided_key(&mut keys, "code", self.code.is_some());
        provided_key(&mut keys, "description", self.description.is_some());
        provided_key(&mut keys, "price", self.price.is_some());
        provided_key(&mut keys, "currency", self.currency.is_some());
        provided_key(&mut keys, "unit", self.unit.is_some());
        provided_key(&mut keys, "recurrence", self.recurrence.is_some());
        provided_key(&mut keys, "term_months", self.term_months.is_some());
        provided_key(&mut keys, "is_active", self.is_active.is_some());
        keys
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SaveProductCommand {
    pub input: ProductInput,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductDetails {
    pub product: Product,
    pub(crate) custom_fields: Map<String, Value>,
}

pub(crate) const PRODUCT_NOT_FOUND: &str = "product_not_found";

pub(crate) const PRODUCT_CREATED: &str = "product.created";
pub(crate) const PRODUCT_UPDATED: &str = "product.updated";
pub(crate) const PRODUCT_DELETED: &str = "product.deleted";

pub(crate) const INVALID_PRODUCT: &str = "invalid product";

pub fn product_not_found() -> AppError {
    AppError::not_found_code(PRODUCT_NOT_FOUND, "product was not found")
}

pub fn parse_money(text: &str) -> Option<Decimal> {
    let text = text.trim();
    Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())
        .and_then(normalize_money)
}

pub(crate) fn product_snapshot(data: &ProductData) -> Value {
    let mut map = Map::new();
    insert_text(&mut map, "name", Some(&data.name));
    insert_text(&mut map, "code", data.code.as_deref());
    insert_text(&mut map, "description", data.description.as_deref());
    map.insert("price".to_owned(), decimal_json(data.price));
    insert_text(&mut map, "currency", Some(&data.currency));
    insert_text(&mut map, "unit", data.unit.as_deref());
    insert_text(&mut map, "recurrence", Some(data.recurrence.as_str()));
    map.insert("term_months".to_owned(), json!(data.term_months));
    map.insert("is_active".to_owned(), Value::Bool(data.is_active));
    Value::Object(map)
}

pub(crate) fn apply_product_input(
    base: &ProductData,
    input: ProductInput,
    default_currency: &str,
    validator: &mut InputValidator,
) -> ProductData {
    let mut data = base.clone();
    if let Some(value) = input.name {
        data.name = validator.required_text("name", value, MAX_PRODUCT_NAME_LENGTH);
    }
    if let Some(value) = input.code {
        data.code = validator.text("code", value, MAX_PRODUCT_CODE_LENGTH);
    }
    if let Some(value) = input.description {
        data.description = validator.text("description", value, MAX_LONG_TEXT_LENGTH);
    }
    if let Some(value) = input.price {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.price = Decimal::ZERO,
            Some(text) => match parse_money(text) {
                Some(price) => data.price = price,
                None => validator.violation(
                    "price",
                    format!(
                        "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                    ),
                ),
            },
        }
    }
    if let Some(value) = input.currency {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.currency = default_currency.to_owned(),
            Some(text) => match normalize_currency(text) {
                Some(currency) => data.currency = currency,
                None => {
                    validator.violation("currency", "must be an ISO 4217 code with three letters")
                }
            },
        }
    }
    if let Some(value) = input.unit {
        data.unit = validator.text("unit", value, MAX_PRODUCT_UNIT_LENGTH);
    }
    if let Some(value) = input.recurrence {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.recurrence = ProductRecurrence::None,
            Some(text) => match ProductRecurrence::parse(text) {
                Some(recurrence) => data.recurrence = recurrence,
                None => {
                    validator.violation("recurrence", "must be none, monthly, quarterly or yearly")
                }
            },
        }
    }
    if let Some(value) = input.term_months {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.term_months = None,
            Some(text) => match parse_term_months(text) {
                Some(months) => data.term_months = Some(months),
                None => validator.violation("term_months", term_violation()),
            },
        }
    }
    if data.recurrence == ProductRecurrence::None {
        data.term_months = None;
    }
    if let Some(Some(active)) = input.is_active {
        data.is_active = active;
    }
    data
}

pub fn require_catalog(scope: &AccessScope, permission: &Permission) -> Result<(), AppError> {
    match scope.record_filter(permission)? {
        RecordScopeFilter::Organization => Ok(()),
        _ => Err(permission_denied()),
    }
}
