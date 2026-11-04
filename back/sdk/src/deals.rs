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
    async fn lifecycle_people(&self, ctx: &crate::Ctx<'_>, id: Uuid)
    -> Result<Vec<Uuid>, AppError>;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreditedUsers {
    User(aphel_kernel::domain::UserId),
    Team(aphel_kernel::domain::TeamId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreditTotals {
    pub count: u64,
    pub amount: Decimal,
}

#[async_trait]
pub trait DealCreditReader: Send + Sync {
    async fn won_total(
        &self,
        ctx: &crate::Ctx<'_>,
        credited: &CreditedUsers,
        amount_key: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<CreditTotals, AppError>;
    async fn won_months(
        &self,
        ctx: &crate::Ctx<'_>,
        credited: &CreditedUsers,
        amount_key: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<std::collections::BTreeMap<NaiveDate, CreditTotals>, AppError>;
}

pub async fn credited_won_total(
    ctx: &crate::Ctx<'_>,
    credited: &CreditedUsers,
    amount_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Option<CreditTotals>, AppError> {
    let Some(reader) = ctx.services().get::<dyn DealCreditReader>() else {
        return Ok(None);
    };
    reader
        .won_total(ctx, credited, amount_key, from, to)
        .await
        .map(Some)
}

pub const RESOURCE: aphel_kernel::application::public_events::PublicResource =
    aphel_kernel::application::public_events::PublicResource::new(RECORD_TYPE, "DealResponse");

use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::RecordQuery;
use aphel_kernel::domain::{TeamId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbabilityDealsRequest {
    pub source: ProbabilitySource,
    pub probability_range: Option<(u8, u8)>,
    pub window: ListWindow,
    pub credited: Option<CreditedUsers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbabilityDeal {
    pub id: Uuid,
    pub title: String,
    pub amount: Option<Decimal>,
    pub currency: String,
    pub probability: u8,
    pub stage_name: String,
    pub owner: Option<UserReference>,
    pub expected_close_date: Option<NaiveDate>,
}

#[async_trait]
pub trait DealProbabilityReader: Send + Sync {
    async fn groups(
        &self,
        ctx: &crate::Ctx<'_>,
        query: RecordQuery,
        source: ProbabilitySource,
        credited: Option<&CreditedUsers>,
    ) -> Result<Vec<ProbabilityGroup>, AppError>;
    async fn deals(
        &self,
        ctx: &crate::Ctx<'_>,
        query: RecordQuery,
        request: &ProbabilityDealsRequest,
    ) -> Result<Listing<ProbabilityDeal>, AppError>;
}

pub fn probability_reader(
    ctx: &crate::Ctx<'_>,
) -> Result<std::sync::Arc<dyn DealProbabilityReader>, AppError> {
    ctx.services()
        .get::<dyn DealProbabilityReader>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "deal_probability_reader_unavailable",
                "deal probability reading is not enabled",
            )
        })
}

pub async fn credited_won_months(
    ctx: &crate::Ctx<'_>,
    credited: &CreditedUsers,
    amount_key: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Option<std::collections::BTreeMap<NaiveDate, CreditTotals>>, AppError> {
    let Some(reader) = ctx.services().get::<dyn DealCreditReader>() else {
        return Ok(None);
    };
    reader
        .won_months(ctx, credited, amount_key, from, to)
        .await
        .map(Some)
}

pub fn credited_from_owner_filter(
    user_id: Option<&str>,
    team_id: Option<&str>,
) -> Option<CreditedUsers> {
    let present = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|value| Uuid::parse_str(value).ok())
    };
    match (present(user_id), present(team_id)) {
        (Some(user_id), None) => UserId::new(user_id.to_string())
            .ok()
            .map(CreditedUsers::User),
        (None, Some(team_id)) => TeamId::new(team_id.to_string())
            .ok()
            .map(CreditedUsers::Team),
        _ => None,
    }
}
