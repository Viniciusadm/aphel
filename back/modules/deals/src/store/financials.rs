use crate::domain::DealStatus;
use aphel_kernel::errors::AppError;
use aphel_sdk::deals::{DealFinancials, DealRevenue, LockedAmount, RevenueState};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::repositories::mapper::{
    invalid_column, naive, optional_uuid, utc,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;
pub(crate) struct Financials;
#[async_trait::async_trait]
impl DealFinancials for Financials {
    async fn lock_amount(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
    ) -> Result<Option<LockedAmount>, AppError> {
        let row = sqlx::query(
            "SELECT amount, currency FROM deals \
             WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(deal_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        row.map(|row| {
            Ok(LockedAmount {
                amount: row.try_get::<Option<Decimal>, _>("amount")?,
                currency: row.try_get("currency")?,
            })
        })
        .transpose()
    }

    async fn set_amount(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
        amount: Decimal,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deals SET amount = ?, amount_base = ROUND(? * exchange_rate, 2), updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(amount)
        .bind(amount)
        .bind(naive(at))
        .bind(deal_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn lock_revenue(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
    ) -> Result<Option<RevenueState>, AppError> {
        let row = sqlx::query(
            "SELECT d.amount_base, d.exchange_rate, d.mrr, d.arr, d.tcv, d.contract_start, \
             d.contract_end, d.renewal_of_deal_id, d.status, d.closed_at, o.timezone \
             FROM deals d JOIN organization_settings o ON o.id = 1 \
             WHERE d.id = ? AND d.deleted_at IS NULL FOR UPDATE",
        )
        .bind(deal_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let status: String = row.try_get("status")?;
        let status =
            DealStatus::parse(&status).ok_or_else(|| invalid_column("deals.status", &status))?;
        let timezone: String = row.try_get("timezone")?;
        let timezone = timezone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
        let closed_at: Option<DateTime<Utc>> = row
            .try_get::<Option<NaiveDateTime>, _>("closed_at")?
            .map(utc);
        Ok(Some(RevenueState {
            revenue: DealRevenue {
                amount_base: row.try_get::<Option<Decimal>, _>("amount_base")?,
                exchange_rate: row.try_get::<Option<Decimal>, _>("exchange_rate")?,
                mrr: row.try_get::<Option<Decimal>, _>("mrr")?,
                arr: row.try_get::<Option<Decimal>, _>("arr")?,
                tcv: row.try_get::<Option<Decimal>, _>("tcv")?,
                contract_start: row.try_get::<Option<NaiveDate>, _>("contract_start")?,
                contract_end: row.try_get::<Option<NaiveDate>, _>("contract_end")?,
                renewal_of_deal_id: optional_uuid(&row, "renewal_of_deal_id")?,
            },
            won_on: closed_at
                .filter(|_| status == DealStatus::Won)
                .map(|at| at.with_timezone(&timezone).date_naive()),
        }))
    }

    async fn save_revenue(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
        revenue: &DealRevenue,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deals SET mrr = ?, arr = ?, tcv = ?, contract_start = ?, contract_end = ?, \
             updated_at = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(revenue.mrr)
        .bind(revenue.arr)
        .bind(revenue.tcv)
        .bind(revenue.contract_start)
        .bind(revenue.contract_end)
        .bind(naive(at))
        .bind(deal_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
    async fn open_ids(&self, tx: &mut Tx, currency: &str) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM deals WHERE currency = ? AND status = 'open' AND deleted_at IS NULL FOR UPDATE")
            .bind(currency).fetch_all(&mut **tx).await?;
        ids.into_iter()
            .map(|id| Uuid::parse_str(&id).map_err(|_| invalid_column("deals.id", &id)))
            .collect()
    }
}
