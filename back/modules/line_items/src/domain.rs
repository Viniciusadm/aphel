use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};

use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::decimal_json;
use aphel_kernel::application::shared::{FieldChange, RecordType};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::Patch;
use aphel_sdk::line_items::{AmountMode, LineData, LineItem, LineTotalError};

use aphel_kernel::application::shared::money_text;

pub(crate) const LINE_ITEM_NOT_FOUND: &str = "line_item_not_found";
pub(crate) const PRODUCT_CURRENCY_MISMATCH: &str = "product_currency_mismatch";
pub(crate) const LINE_ITEMS_LIMIT_REACHED: &str = "line_items_limit_reached";
pub(crate) const RECORD_HAS_NO_ITEMS: &str = "record_has_no_items";
pub(crate) const RECORD_CURRENCY_HAS_ITEMS: &str = "record_currency_has_items";

pub(crate) const LINE_ITEM_ADDED: &str = "line_items.item_added";
pub(crate) const LINE_ITEM_UPDATED: &str = "line_items.item_updated";
pub(crate) const LINE_ITEM_REMOVED: &str = "line_items.item_removed";
pub(crate) const LINE_ITEMS_REORDERED: &str = "line_items.items_reordered";
pub(crate) const LINE_ITEM_SETTINGS_UPDATED: &str = "line_item_settings.updated";
pub(crate) const LINE_ITEM_SETTINGS_RECORD_TYPE: RecordType =
    RecordType::from_static("line_items.line_item_settings");

pub(crate) const INVALID_LINE_ITEM: &str = "invalid line item";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineItemInput {
    pub product_id: Patch<String>,
    pub name: Patch<String>,
    pub unit: Patch<String>,
    pub quantity: Patch<String>,
    pub unit_price: Patch<String>,
    pub discount_type: Patch<String>,
    pub discount_value: Patch<String>,
    pub tax_percent: Patch<String>,
    pub recurrence: Patch<String>,
    pub term_months: Patch<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveLineItemCommand {
    pub input: LineItemInput,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReorderLineItemsCommand {
    pub(crate) item_ids: Vec<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateLineItemSettingsCommand {
    pub(crate) amount_mode: Option<AmountMode>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItemLine {
    pub item: LineItem,
    pub(crate) gross: Decimal,
    pub(crate) discount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItemsSummary {
    pub(crate) record: aphel_kernel::application::shared::RecordRef,
    pub(crate) currency: String,
    pub items: Vec<LineItemLine>,
    pub(crate) subtotal: Decimal,
    pub(crate) discount_total: Decimal,
    pub(crate) total: Decimal,
    pub(crate) record_amount: Option<Decimal>,
    pub(crate) amount_mode: AmountMode,
    pub(crate) amount_managed: bool,
    pub(crate) can_apply_total: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LineItemSettings {
    pub(crate) amount_mode: AmountMode,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct HostRecord {
    pub record: aphel_kernel::application::shared::RecordRef,
    pub amount: Option<Decimal>,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
}

pub(crate) fn item_not_found() -> AppError {
    AppError::not_found_code(LINE_ITEM_NOT_FOUND, "line item was not found")
}

pub(crate) fn item_snapshot(item: &LineItem) -> Value {
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
        "tax_percent": decimal_json(item.data.tax_percent),
        "total": decimal_json(item.total),
    })
}

pub(crate) fn item_changes(old: &Value, new: &Value) -> Result<Vec<FieldChange>, AppError> {
    Ok(diff_values(old, new, &DiffOptions::default())?
        .into_iter()
        .map(|change| FieldChange::new(format!("item.{}", change.field), change.old, change.new))
        .collect())
}

pub(crate) fn line(item: LineItem) -> LineItemLine {
    let amounts = item.data.amounts().ok();
    LineItemLine {
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

pub(crate) fn line_total(data: &LineData) -> Result<Decimal, AppError> {
    match data.amounts() {
        Ok(amounts) => Ok(amounts.total),
        Err(LineTotalError::PercentOutOfRange) => Err(AppError::invalid_fields(
            INVALID_LINE_ITEM,
            vec![FieldViolation::new(
                "discount_value",
                "must be a percentage between 0 and 100",
            )],
        )),
        Err(LineTotalError::TaxOutOfRange) => Err(AppError::invalid_fields(
            INVALID_LINE_ITEM,
            vec![FieldViolation::new(
                "tax_percent",
                "must be between 0 and 100",
            )],
        )),
        Err(LineTotalError::TooLarge) => Err(AppError::invalid_fields(
            INVALID_LINE_ITEM,
            vec![FieldViolation::new(
                "unit_price",
                "the line total is too large",
            )],
        )),
    }
}

pub fn item_command(data: &LineData, with_product: bool) -> SaveLineItemCommand {
    SaveLineItemCommand {
        input: LineItemInput {
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
            tax_percent: Some(Some(data.tax_percent.to_string())),
            recurrence: Some(Some(data.recurrence.as_str().to_owned())),
            term_months: Some(data.term_months.map(|months| months.to_string())),
        },
        ip: None,
        expected_updated_at: None,
    }
}
