use crate::{AppError, Ctx};
pub use aphel_contract_pricing::*;
use aphel_kernel::domain::UserId;
use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;
#[async_trait]
pub trait PriceListResolver: Send + Sync {
    async fn find(
        &self,
        ctx: &Ctx<'_>,
        price_list_id: Uuid,
    ) -> Result<Option<PriceListSummary>, AppError>;

    async fn default_for(
        &self,
        ctx: &Ctx<'_>,
        selection: &PriceListSelection,
    ) -> Result<Option<PriceListSummary>, AppError>;

    async fn price(
        &self,
        ctx: &Ctx<'_>,
        request: &PriceRequest,
    ) -> Result<Option<ResolvedPrice>, AppError>;
}

#[async_trait]
pub trait DiscountLimits: Send + Sync {
    async fn max_discount_percent(
        &self,
        ctx: &Ctx<'_>,
        user_id: &UserId,
    ) -> Result<Option<Decimal>, AppError>;
}

#[async_trait]
pub trait BundleExpander: Send + Sync {
    async fn expand(
        &self,
        ctx: &Ctx<'_>,
        product_id: Uuid,
        price_list_id: Option<Uuid>,
    ) -> Result<Option<BundleExpansion>, AppError>;
}

pub fn price_list_resolver(ctx: &Ctx<'_>) -> Option<std::sync::Arc<dyn PriceListResolver>> {
    ctx.services().get::<dyn PriceListResolver>()
}
pub fn discount_limits(ctx: &Ctx<'_>) -> Option<std::sync::Arc<dyn DiscountLimits>> {
    ctx.services().get::<dyn DiscountLimits>()
}
pub fn bundle_expander(ctx: &Ctx<'_>) -> Option<std::sync::Arc<dyn BundleExpander>> {
    ctx.services().get::<dyn BundleExpander>()
}
