use crate::AppError;
pub use aphel_contract_deals::*;
use aphel_kernel::database::transaction::MySqlTx;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedAmount {
    pub amount: Option<Decimal>,
    pub currency: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevenueState {
    pub revenue: DealRevenue,
    pub won_on: Option<NaiveDate>,
}
#[async_trait]
pub trait DealFinancials: Send + Sync {
    async fn lock_amount(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
    ) -> Result<Option<LockedAmount>, AppError>;
    async fn set_amount(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        amount: Decimal,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn lock_revenue(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
    ) -> Result<Option<RevenueState>, AppError>;
    async fn save_revenue(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        revenue: &DealRevenue,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn open_ids(&self, tx: &mut MySqlTx, currency: &str) -> Result<Vec<Uuid>, AppError>;
}
pub trait RevenueCalculator: Send + Sync {
    fn project(
        &self,
        revenue: &mut DealRevenue,
        items: &[crate::line_items::LineItem],
        won_on: Option<NaiveDate>,
    );
}

#[async_trait]
pub trait DealLifecycle: Send + Sync {
    async fn parties(
        &self,
        pool: &sqlx::MySqlPool,
        id: Uuid,
    ) -> Result<Option<DealParties>, AppError>;
    async fn outcomes(
        &self,
        pool: &sqlx::MySqlPool,
        entity: &crate::shared::RecordType,
        id: Uuid,
    ) -> Result<DealOutcomes, AppError>;
}
