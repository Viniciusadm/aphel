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
    pub code: Option<String>,
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
    async fn items(&self, ctx: &Ctx<'_>, ids: &[Uuid]) -> Result<Vec<CatalogItem>, AppError> {
        let mut items = Vec::new();
        for id in ids {
            if let Some(item) = self.item(ctx, *id).await? {
                items.push(item);
            }
        }
        Ok(items)
    }
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

pub async fn catalog_items(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, CatalogItem>, AppError> {
    if ids.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 catalog items may be requested".into(),
        ));
    }
    let requested: std::collections::BTreeSet<_> = ids.iter().copied().collect();
    if requested.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let ids: Vec<_> = requested.iter().copied().collect();
    let mut found = std::collections::HashMap::new();
    for source in ctx.app().contributions::<dyn ItemCatalog>() {
        for item in source.items(ctx, &ids).await? {
            if !requested.contains(&item.id) {
                return Err(AppError::Configuration(
                    "catalog returned an unrequested identity".into(),
                ));
            }
            if found.insert(item.id, item).is_some() {
                return Err(AppError::Configuration(
                    "catalog item identity is ambiguous".into(),
                ));
            }
        }
    }
    Ok(found)
}
pub async fn catalog_item(ctx: &Ctx<'_>, id: Uuid) -> Result<Option<CatalogItem>, AppError> {
    Ok(catalog_items(ctx, &[id]).await?.remove(&id))
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
