use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::decimal_json;
use aphel_kernel::application::shared::{FieldChange, RecordType};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::Patch;
use products::domain::{DealAmountMode, DealItem, DealItemData, LineTotalError};

use aphel_kernel::application::shared::money_text;

pub(crate) const DEAL_ITEM_NOT_FOUND: &str = "deal_item_not_found";
pub(crate) const PRODUCT_CURRENCY_MISMATCH: &str = "product_currency_mismatch";
pub(crate) const DEAL_ITEMS_LIMIT_REACHED: &str = "deal_items_limit_reached";
pub(crate) const DEAL_HAS_NO_ITEMS: &str = "deal_has_no_items";
pub(crate) const DEAL_CURRENCY_HAS_ITEMS: &str = "deal_currency_has_items";

pub(crate) const DEAL_ITEM_ADDED: &str = "deal.item_added";
pub(crate) const DEAL_ITEM_UPDATED: &str = "deal.item_updated";
pub(crate) const DEAL_ITEM_REMOVED: &str = "deal.item_removed";
pub(crate) const DEAL_ITEMS_REORDERED: &str = "deal.items_reordered";
pub(crate) const DEAL_ITEM_SETTINGS_UPDATED: &str = "deal_item_settings.updated";
pub(crate) const DEAL_ITEM_SETTINGS_RECORD_TYPE: RecordType =
    RecordType::from_static("deal_item_settings");

pub(crate) const INVALID_DEAL_ITEM: &str = "invalid deal item";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DealItemInput {
    pub product_id: Patch<String>,
    pub name: Patch<String>,
    pub unit: Patch<String>,
    pub quantity: Patch<String>,
    pub unit_price: Patch<String>,
    pub discount_type: Patch<String>,
    pub discount_value: Patch<String>,
    pub recurrence: Patch<String>,
    pub term_months: Patch<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveDealItemCommand {
    pub input: DealItemInput,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReorderDealItemsCommand {
    pub(crate) item_ids: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateDealItemSettingsCommand {
    pub(crate) amount_mode: Option<DealAmountMode>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealItemLine {
    pub item: DealItem,
    pub(crate) gross: Decimal,
    pub(crate) discount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealItemsSummary {
    pub(crate) deal_id: Uuid,
    pub(crate) currency: String,
    pub items: Vec<DealItemLine>,
    pub(crate) subtotal: Decimal,
    pub(crate) discount_total: Decimal,
    pub(crate) total: Decimal,
    pub(crate) deal_amount: Option<Decimal>,
    pub(crate) amount_mode: DealAmountMode,
    pub(crate) amount_managed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DealItemSettings {
    pub(crate) amount_mode: DealAmountMode,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LockedDeal {
    pub(crate) amount: Option<Decimal>,
    pub(crate) currency: String,
}

pub(crate) fn item_not_found() -> AppError {
    AppError::not_found_code(DEAL_ITEM_NOT_FOUND, "deal item was not found")
}

pub(crate) fn optional_decimal(value: Option<Decimal>) -> Value {
    value.map(decimal_json).unwrap_or(Value::Null)
}

pub(crate) fn item_snapshot(item: &DealItem) -> Value {
    json!({
        "product_id": item.data.product_id.map(|id| id.to_string()),
        "name": item.data.name,
        "unit": item.data.unit,
        "quantity": decimal_json(item.data.quantity),
        "unit_price": decimal_json(item.data.unit_price),
        "discount_type": item.data.discount_kind.as_str(),
        "discount_value": decimal_json(item.data.discount_value),
        "recurrence": item.data.recurrence.as_str(),
        "term_months": item.data.term_months,
        "total": decimal_json(item.total),
    })
}

pub(crate) fn item_changes(old: &Value, new: &Value) -> Result<Vec<FieldChange>, AppError> {
    Ok(diff_values(old, new, &DiffOptions::default())?
        .into_iter()
        .map(|change| FieldChange::new(format!("item.{}", change.field), change.old, change.new))
        .collect())
}

pub(crate) fn line(item: DealItem) -> DealItemLine {
    let amounts = item.data.amounts().ok();
    DealItemLine {
        gross: amounts.map_or(item.total, |amounts| amounts.gross),
        discount: amounts.map_or(Decimal::ZERO, |amounts| amounts.discount),
        item,
    }
}

pub(crate) fn parse_decimal(text: &str) -> Option<Decimal> {
    let text = text.trim();
    Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())
}

pub(crate) fn line_total(data: &DealItemData) -> Result<Decimal, AppError> {
    match data.amounts() {
        Ok(amounts) => Ok(amounts.total),
        Err(LineTotalError::PercentOutOfRange) => Err(AppError::invalid_fields(
            INVALID_DEAL_ITEM,
            vec![FieldViolation::new(
                "discount_value",
                "must be a percentage between 0 and 100",
            )],
        )),
        Err(LineTotalError::TooLarge) => Err(AppError::invalid_fields(
            INVALID_DEAL_ITEM,
            vec![FieldViolation::new(
                "unit_price",
                "the line total is too large",
            )],
        )),
    }
}

pub fn item_command(data: &DealItemData, with_product: bool) -> SaveDealItemCommand {
    SaveDealItemCommand {
        input: DealItemInput {
            product_id: Some(
                data.product_id
                    .filter(|_| with_product)
                    .map(|id| id.to_string()),
            ),
            name: Some(Some(data.name.clone())),
            unit: Some(data.unit.clone()),
            quantity: Some(Some(data.quantity.normalize().to_string())),
            unit_price: Some(Some(money_text(data.unit_price))),
            discount_type: Some(Some(data.discount_kind.as_str().to_owned())),
            discount_value: Some(Some(money_text(data.discount_value))),
            recurrence: Some(Some(data.recurrence.as_str().to_owned())),
            term_months: Some(data.term_months.map(|months| months.to_string())),
        },
        ip: None,
        expected_updated_at: None,
    }
}
