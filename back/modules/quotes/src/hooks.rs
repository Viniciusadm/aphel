use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    DealItemsOutcome, Quote, QuoteAcceptance, QuoteDiscount, QuoteItemData, TotalsBreakdown,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::Hook;
use products::domain::DealItemData;

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptingQuote {
    pub(crate) quote: Quote,
    pub(crate) acceptance: QuoteAcceptance,
}

pub struct QuoteAccepting;

impl Hook for QuoteAccepting {
    type Args = AcceptingQuote;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PricedQuote {
    pub(crate) quote_id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) currency: String,
    pub(crate) discount: QuoteDiscount,
    pub(crate) items: Vec<QuoteItemData>,
    pub(crate) totals: TotalsBreakdown,
}

pub struct QuoteTotals;

impl Hook for QuoteTotals {
    type Args = PricedQuote;
}

#[async_trait]
pub trait DealItemsWriter: Send + Sync {
    async fn replace(
        &self,
        ctx: &Ctx<'_>,
        scope: &AccessScope,
        deal_id: Uuid,
        items: Vec<DealItemData>,
    ) -> Result<DealItemsOutcome, AppError>;
}
