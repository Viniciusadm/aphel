use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::Patch;

pub(crate) const PRICE_LIST_CREATED: &str = "price_list.created";
pub(crate) const PRICE_LIST_UPDATED: &str = "price_list.updated";
pub(crate) const PRICE_LIST_DELETED: &str = "price_list.deleted";
pub(crate) const PRICE_LIST_ITEMS_REPLACED: &str = "price_list.items_replaced";
pub(crate) const DISCOUNT_LIMITS_UPDATED: &str = "discount_limits.updated";

pub(crate) const PRICE_LIST_NOT_FOUND: &str = "price_list_not_found";
pub(crate) const PRICE_LIST_KEY_TAKEN: &str = "price_list_key_taken";

pub(crate) const PRICE_LIST_RECORD_TYPE: RecordType = RecordType::from_static("price_list");

pub(crate) const MAX_PRICE_LISTS: u64 = 500;
pub(crate) const MAX_PRICE_LIST_NAME_LENGTH: usize = 140;
pub(crate) const MAX_PRICE_LIST_DESCRIPTION_LENGTH: usize = 2_000;
pub(crate) const MAX_PRICE_LIST_COMPANIES: usize = 500;
pub(crate) const MAX_PRICE_LIST_ITEMS: usize = 5_000;
pub(crate) const DISCOUNT_PERCENT_DECIMALS: u32 = 2;

pub(crate) fn price_list_not_found() -> AppError {
    AppError::not_found_code(PRICE_LIST_NOT_FOUND, "price list was not found")
}

pub(crate) fn price_list_key_taken() -> AppError {
    AppError::conflict_code(
        PRICE_LIST_KEY_TAKEN,
        "another price list already uses this key",
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriceListAppliesTo {
    All,
    Segment,
    Companies,
}

impl PriceListAppliesTo {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Segment => "segment",
            Self::Companies => "companies",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "segment" => Some(Self::Segment),
            "companies" => Some(Self::Companies),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceList {
    pub id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) currency: String,
    pub(crate) applies_to: PriceListAppliesTo,
    pub(crate) segment_id: Option<Uuid>,
    pub(crate) company_ids: Vec<Uuid>,
    pub(crate) valid_from: Option<NaiveDate>,
    pub(crate) valid_to: Option<NaiveDate>,
    pub(crate) is_active: bool,
    pub(crate) is_default: bool,
    pub(crate) item_count: u64,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl PriceList {
    pub(crate) fn is_valid_on(&self, on: NaiveDate) -> bool {
        self.is_active
            && self.valid_from.is_none_or(|from| from <= on)
            && self.valid_to.is_none_or(|to| to >= on)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceListItem {
    pub(crate) id: Uuid,
    pub(crate) product_id: Uuid,
    pub(crate) product_name: String,
    pub(crate) product_code: Option<String>,
    pub(crate) product_price: Decimal,
    pub(crate) product_currency: String,
    pub(crate) min_quantity: Decimal,
    pub(crate) unit_price: Decimal,
    pub(crate) position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewPriceListItem {
    pub(crate) id: Uuid,
    pub(crate) product_id: Uuid,
    pub(crate) min_quantity: Decimal,
    pub(crate) unit_price: Decimal,
    pub(crate) position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PriceListCandidate {
    pub(crate) list: PriceList,
    pub(crate) matches_company: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PriceListFilter {
    pub(crate) currency: Option<String>,
    pub(crate) active: Option<bool>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavePriceListCommand {
    pub key: Option<String>,
    pub name: Option<String>,
    pub description: Patch<String>,
    pub currency: Option<String>,
    pub applies_to: Option<String>,
    pub segment_id: Patch<String>,
    pub company_ids: Option<Vec<String>>,
    pub valid_from: Patch<String>,
    pub valid_to: Patch<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PriceListItemInput {
    pub product_id: String,
    pub unit_price: String,
    pub min_quantity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplacePriceListItemsCommand {
    pub items: Vec<PriceListItemInput>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleDiscountLimit {
    pub(crate) role_id: Uuid,
    pub(crate) role_key: String,
    pub(crate) role_name: String,
    pub(crate) max_discount_percent: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DiscountLimitInput {
    pub(crate) role_id: String,
    pub(crate) max_discount_percent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SetDiscountLimitsCommand {
    pub(crate) limits: Vec<DiscountLimitInput>,
    pub(crate) ip: Option<String>,
}

pub(crate) fn select_tier(
    items: &[PriceListItem],
    product_id: Uuid,
    quantity: Decimal,
) -> Option<&PriceListItem> {
    let tiers: Vec<&PriceListItem> = items
        .iter()
        .filter(|item| item.product_id == product_id)
        .collect();
    tiers
        .iter()
        .filter(|item| item.min_quantity <= quantity)
        .max_by(|left, right| left.min_quantity.cmp(&right.min_quantity))
        .or_else(|| {
            tiers
                .iter()
                .min_by(|left, right| left.min_quantity.cmp(&right.min_quantity))
        })
        .copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceListSelection {
    pub currency: String,
    pub company_id: Option<Uuid>,
    pub on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRequest {
    pub price_list_id: Uuid,
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPrice {
    pub(crate) price_list_id: Uuid,
    pub(crate) product_id: Uuid,
    pub unit_price: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceListSummary {
    pub id: Uuid,
    pub(crate) name: String,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BundlePricing {
    Fixed,
    Sum,
}

impl BundlePricing {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Sum => "sum",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "fixed" => Some(Self::Fixed),
            "sum" => Some(Self::Sum),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleComponent {
    pub product_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleExpansion {
    pub(crate) product_id: Uuid,
    pub(crate) pricing: BundlePricing,
    pub unit_price: Decimal,
    pub currency: String,
    pub components: Vec<BundleComponent>,
}

pub(crate) const PRODUCT_BUNDLE_UPDATED: &str = "product_bundle.updated";
pub(crate) const PRODUCT_BUNDLE_REMOVED: &str = "product_bundle.removed";

pub(crate) const PRODUCT_BUNDLE_NOT_FOUND: &str = "product_bundle_not_found";
pub(crate) const PRODUCT_BUNDLE_NESTED: &str = "product_bundle_nested";

pub(crate) const MAX_BUNDLE_COMPONENTS: usize = 50;

pub(crate) fn product_bundle_not_found() -> AppError {
    AppError::not_found_code(PRODUCT_BUNDLE_NOT_FOUND, "the product is not a bundle")
}

pub(crate) fn product_bundle_nested() -> AppError {
    AppError::conflict_code(
        PRODUCT_BUNDLE_NESTED,
        "the product is a component of another bundle and cannot become a bundle",
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundleItem {
    pub(crate) product_id: Uuid,
    pub(crate) quantity: Decimal,
    pub(crate) position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductBundle {
    pub(crate) product_id: Uuid,
    pub(crate) pricing: BundlePricing,
    pub(crate) components: Vec<BundleItem>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogProduct {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    pub(crate) price: Decimal,
    pub(crate) currency: String,
    pub(crate) is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BundleComponentInput {
    pub(crate) product_id: String,
    pub(crate) quantity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveBundleCommand {
    pub(crate) pricing: BundlePricing,
    pub(crate) components: Vec<BundleComponentInput>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundleDetails {
    pub(crate) expansion: BundleExpansion,
    pub(crate) updated_at: DateTime<Utc>,
}
