use crate::{AppError, Ctx, fields::EntityTraits, shared::RecordRef};
pub use aphel_contract_line_items::*;
use aphel_kernel::{database::transaction::MySqlTx, domain::UserId, modules::Hook};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;
pub const KEY: &str = "line_items";
pub trait LineItemTraits {
    fn line_items(self, mapping: LineItemMapping) -> Self;
    fn line_item_mapping(&self) -> Result<Option<LineItemMapping>, AppError>;
}
impl LineItemTraits for EntityTraits {
    fn line_items(self, mapping: LineItemMapping) -> Self {
        self.with_trait_data(
            ATTACHABLE.key(),
            serde_json::to_value(mapping).expect("line item mapping"),
        )
    }
    fn line_item_mapping(&self) -> Result<Option<LineItemMapping>, AppError> {
        self.trait_data(ATTACHABLE.key())
            .map(|value| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    AppError::Configuration(format!("invalid line-item host mapping: {error}"))
                })
            })
            .transpose()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItem {
    pub id: Uuid,
    pub record: RecordRef,
    pub data: LineData,
    pub total: Decimal,
    pub amount_base: Option<Decimal>,
    pub exchange_rate: Option<Decimal>,
    pub position: i32,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
pub fn items_total(items: &[LineItem]) -> Decimal {
    items.iter().map(|item| item.total).sum()
}
#[derive(Debug, Clone)]
pub struct CatalogItem {
    pub id: Uuid,
    pub name: String,
    pub unit: Option<String>,
    pub currency: String,
    pub price: Decimal,
    pub recurrence: Recurrence,
    pub term_months: Option<u16>,
    pub active: bool,
}
#[async_trait]
pub trait ItemCatalog: Send + Sync {
    fn record_type(&self) -> crate::shared::RecordType;
    async fn item(&self, ctx: &Ctx<'_>, id: Uuid) -> Result<Option<CatalogItem>, AppError>;
}
#[derive(Debug, Clone)]
pub struct TotalsChanged {
    pub record: RecordRef,
    pub items: Vec<LineItem>,
    pub mode: AmountMode,
    pub at: DateTime<Utc>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, Copy)]
pub struct LineItemsChanged;
impl Hook for LineItemsChanged {
    type Args = TotalsChanged;
    fn key() -> &'static str {
        "line_items.totals_changed"
    }
}
#[derive(Debug, Clone)]
pub struct LineItemSummary {
    pub currency: String,
    pub items: Vec<LineItem>,
    pub subtotal: Decimal,
    pub discount_total: Decimal,
    pub total: Decimal,
}
#[async_trait]
pub trait LineItemsPort: Send + Sync {
    async fn summary(&self, ctx: &Ctx<'_>, record: &RecordRef)
    -> Result<LineItemSummary, AppError>;
    async fn rows(&self, ctx: &Ctx<'_>, record: &RecordRef) -> Result<Vec<LineItem>, AppError>;
    async fn managed(&self, ctx: &Ctx<'_>, record: &RecordRef) -> Result<bool, AppError>;
    async fn sync_rate(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        rate: Option<Decimal>,
    ) -> Result<(), AppError>;
    async fn replace(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        items: Vec<LineData>,
    ) -> Result<bool, AppError>;
}

pub async fn catalog_item(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<CatalogItem>, AppError> {
    let mut found = None;
    for source in aphel_kernel::modules::app().contributions::<dyn ItemCatalog>() {
        if let Some(item) = source.item(ctx, id).await? {
            if found.is_some() {
                return Err(AppError::Configuration(
                    "catalog item identity is ambiguous".to_owned(),
                ));
            }
            found = Some(item);
        }
    }
    Ok(found)
}

pub fn monthly_amount(item: &LineItem) -> Option<Decimal> {
    let period = match item.data.recurrence {
        Recurrence::None => return None,
        Recurrence::Monthly => 1,
        Recurrence::Quarterly => 3,
        Recurrence::Yearly => 12,
    };
    Some(item.total / Decimal::from(period))
}

#[async_trait]
pub trait LineItemProjection: Send + Sync {
    fn record_type(&self) -> crate::shared::RecordType;
    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        totals: &TotalsChanged,
    ) -> Result<(), AppError>;
}
