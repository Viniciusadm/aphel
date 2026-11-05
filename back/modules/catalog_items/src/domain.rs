pub use aphel_sdk::line_items::{
    AmountMode, DiscountKind, LineAmounts, LineData, LineTotalError, MAX_ITEM_NAME_LENGTH,
    MAX_LINES_PER_RECORD, MAX_TERM_MONTHS, MAX_UNIT_LENGTH, MONEY_DECIMALS, QUANTITY_DECIMALS,
    Recurrence, money, money_limit, normalize_money, normalize_quantity, parse_money,
    parse_term_months, round_money, term_violation,
};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::records::{
    InputValidator, MAX_LONG_TEXT_LENGTH, Patch, bool_patch, insert_text, value_patch,
};
use aphel_sdk::platform::application::fields::decimal_json;
use aphel_sdk::platform::application::fields::permissions::provided_key;

pub use aphel_sdk::money::normalize_currency_code as normalize_currency;

pub(crate) const MAX_PRODUCT_NAME_LENGTH: usize = 255;
pub(crate) const MAX_PRODUCT_CODE_LENGTH: usize = 64;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductData {
    pub name: String,
    pub(crate) code: Option<String>,
    pub(crate) description: Option<String>,
    pub price: Decimal,
    pub currency: String,
    pub unit: Option<String>,
    pub recurrence: Recurrence,
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
            recurrence: Recurrence::None,
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

pub(crate) const PRODUCT_CREATED: &str = "product.created";
pub(crate) const PRODUCT_UPDATED: &str = "product.updated";
pub(crate) const PRODUCT_DELETED: &str = "product.deleted";

pub(crate) const INVALID_PRODUCT: &str = "invalid product";

pub use aphel_sdk::catalog_items::product_not_found;

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
        data.unit = validator.text("unit", value, MAX_UNIT_LENGTH);
    }
    if let Some(value) = input.recurrence {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.recurrence = Recurrence::None,
            Some(text) => match Recurrence::parse(text) {
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
    if data.recurrence == Recurrence::None {
        data.term_months = None;
    }
    if let Some(Some(active)) = input.is_active {
        data.is_active = active;
    }
    data
}

pub use aphel_sdk::access::require_organization_record_scope as require_catalog;
