use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceListSelection {
    pub currency: String,
    pub company_id: Option<Uuid>,
    pub on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceRequest {
    pub price_list_id: Uuid,
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPrice {
    pub price_list_id: Uuid,
    pub product_id: Uuid,
    pub unit_price: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceListSummary {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundlePricing {
    Fixed,
    Sum,
}

impl BundlePricing {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Sum => "sum",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fixed" => Some(Self::Fixed),
            "sum" => Some(Self::Sum),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleComponent {
    pub product_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleExpansion {
    pub product_id: Uuid,
    pub pricing: BundlePricing,
    pub unit_price: Decimal,
    pub currency: String,
    pub components: Vec<BundleComponent>,
}

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("pricing", "1.0.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "pricing.resolve",
            aphel_contract::KeyKind::ExtensionPoint,
            "Scoped price list and tier resolution",
        ),
        aphel_contract::KeyEntry::new(
            "pricing.bundle",
            aphel_contract::KeyKind::ExtensionPoint,
            "Catalog bundle expansion",
        ),
        aphel_contract::KeyEntry::new(
            "pricing.discounts",
            aphel_contract::KeyKind::ExtensionPoint,
            "Scoped role discount limits",
        ),
    ],
};
