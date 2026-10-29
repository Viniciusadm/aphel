use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::{DealItemSettings, LockedDeal};
use crate::recurring::won_date;
use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper::{
    instant, invalid_column, naive, optional_user, optional_uuid, utc,
};
use deals::domain::{DealRevenue, DealStatus};
use products::domain::{DealAmountMode, DealItem, DealItemData, DiscountKind, ProductRecurrence};

const ITEM_SELECT: &str = "SELECT id, deal_id, product_id, name, unit, quantity, unit_price, \
     discount_type, discount_value, recurrence, term_months, total, amount_base, exchange_rate, \
     position, created_by, created_at, updated_at \
     FROM deal_items WHERE deal_id = ? ORDER BY position, id";

fn item_from_row(row: &MySqlRow) -> Result<DealItem, AppError> {
    let discount: String = row.try_get("discount_type")?;
    let recurrence: String = row.try_get("recurrence")?;
    Ok(DealItem {
        id: uuid_column(row, "id")?,
        deal_id: uuid_column(row, "deal_id")?,
        data: DealItemData {
            product_id: optional_uuid(row, "product_id")?,
            name: row.try_get("name")?,
            unit: row.try_get("unit")?,
            quantity: row.try_get::<Decimal, _>("quantity")?.normalize(),
            unit_price: row.try_get("unit_price")?,
            discount_kind: DiscountKind::parse(&discount)
                .ok_or_else(|| invalid_column("deal_items.discount_type", &discount))?,
            discount_value: row.try_get("discount_value")?,
            recurrence: ProductRecurrence::parse(&recurrence)
                .ok_or_else(|| invalid_column("deal_items.recurrence", &recurrence))?,
            term_months: row.try_get::<Option<u16>, _>("term_months")?,
        },
        total: row.try_get("total")?,
        amount_base: row.try_get::<Option<Decimal>, _>("amount_base")?,
        exchange_rate: row.try_get::<Option<Decimal>, _>("exchange_rate")?,
        position: row.try_get("position")?,
        created_by: optional_user(row, "created_by")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevenueState {
    pub(crate) revenue: DealRevenue,
    pub(crate) won_on: Option<NaiveDate>,
}

#[derive(Clone)]
pub(crate) struct DealItemStore {
    pool: MySqlPool,
}

impl DealItemStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list(&self, deal_id: Uuid) -> Result<Vec<DealItem>, AppError> {
        let rows = sqlx::query(ITEM_SELECT)
            .bind(deal_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn lock_deal(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
    ) -> Result<Option<LockedDeal>, AppError> {
        let row = sqlx::query(
            "SELECT amount, currency FROM deals \
             WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(deal_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        row.map(|row| {
            Ok(LockedDeal {
                amount: row.try_get::<Option<Decimal>, _>("amount")?,
                currency: row.try_get("currency")?,
            })
        })
        .transpose()
    }

    pub(crate) async fn list_locked(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
    ) -> Result<Vec<DealItem>, AppError> {
        let rows = sqlx::query(ITEM_SELECT)
            .bind(deal_id.to_string())
            .fetch_all(&mut **tx)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, item: &DealItem) -> Result<(), AppError> {
        let data = &item.data;
        sqlx::query(
            "INSERT INTO deal_items (id, deal_id, product_id, name, unit, quantity, \
             unit_price, discount_type, discount_value, recurrence, term_months, total, amount_base, \
             exchange_rate, position, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(item.deal_id.to_string())
        .bind(data.product_id.map(|id| id.to_string()))
        .bind(&data.name)
        .bind(&data.unit)
        .bind(data.quantity)
        .bind(data.unit_price)
        .bind(data.discount_kind.as_str())
        .bind(data.discount_value)
        .bind(data.recurrence.as_str())
        .bind(data.term_months)
        .bind(item.total)
        .bind(item.amount_base)
        .bind(item.exchange_rate)
        .bind(item.position)
        .bind(item.created_by.as_ref().map(UserId::as_str))
        .bind(naive(item.created_at))
        .bind(naive(item.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut MySqlTx, item: &DealItem) -> Result<(), AppError> {
        let data = &item.data;
        sqlx::query(
            "UPDATE deal_items SET product_id = ?, name = ?, unit = ?, quantity = ?, unit_price = ?, \
             discount_type = ?, discount_value = ?, recurrence = ?, term_months = ?, total = ?, \
             amount_base = ?, exchange_rate = ?, updated_at = ? \
             WHERE deal_id = ? AND id = ?",
        )
        .bind(data.product_id.map(|id| id.to_string()))
        .bind(&data.name)
        .bind(&data.unit)
        .bind(data.quantity)
        .bind(data.unit_price)
        .bind(data.discount_kind.as_str())
        .bind(data.discount_value)
        .bind(data.recurrence.as_str())
        .bind(data.term_months)
        .bind(item.total)
        .bind(item.amount_base)
        .bind(item.exchange_rate)
        .bind(naive(item.updated_at))
        .bind(item.deal_id.to_string())
        .bind(item.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
        item_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM deal_items WHERE deal_id = ? AND id = ?")
            .bind(deal_id.to_string())
            .bind(item_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn set_positions(
        &self,
        tx: &mut MySqlTx,
        deal_id: Uuid,
        positions: &[(Uuid, i32)],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (item_id, position) in positions {
            sqlx::query(
                "UPDATE deal_items SET position = ?, updated_at = ? \
                 WHERE deal_id = ? AND id = ?",
            )
            .bind(position)
            .bind(naive(at))
            .bind(deal_id.to_string())
            .bind(item_id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn set_deal_amount(
        &self,
        tx: &mut MySqlTx,
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

    pub(crate) async fn settings(&self) -> Result<Option<DealItemSettings>, AppError> {
        let row =
            sqlx::query("SELECT amount_mode, updated_at FROM deal_item_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        row.map(|row| {
            let mode: String = row.try_get("amount_mode")?;
            Ok(DealItemSettings {
                amount_mode: DealAmountMode::parse(&mode)
                    .ok_or_else(|| invalid_column("deal_item_settings.amount_mode", &mode))?,
                updated_at: Some(instant(&row, "updated_at")?),
            })
        })
        .transpose()
    }

    pub(crate) async fn save_settings(
        &self,
        tx: &mut MySqlTx,
        updated_by: &UserId,
        settings: &DealItemSettings,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deal_item_settings (id, amount_mode, updated_by, created_at, \
             updated_at) VALUES (1, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE amount_mode = VALUES(amount_mode), \
             updated_by = VALUES(updated_by), updated_at = VALUES(updated_at)",
        )
        .bind(settings.amount_mode.as_str())
        .bind(updated_by.as_str())
        .bind(naive(at))
        .bind(naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn amount_managed(&self, deal_id: Uuid) -> Result<bool, AppError> {
        let managed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM deal_item_settings WHERE amount_mode = ?) \
             AND EXISTS(SELECT 1 FROM deal_items WHERE deal_id = ?)",
        )
        .bind(DealAmountMode::Items.as_str())
        .bind(deal_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(managed)
    }

    pub(crate) async fn lock_revenue(
        &self,
        tx: &mut MySqlTx,
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
            won_on: won_date(status, closed_at, timezone),
        }))
    }

    pub(crate) async fn save_revenue(
        &self,
        tx: &mut MySqlTx,
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
}

pub(crate) async fn sync_deal_rate(
    tx: &mut MySqlTx,
    deal_id: Uuid,
    rate: Option<Decimal>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE deal_items SET exchange_rate = ?, amount_base = ROUND(total * ?, 2) \
         WHERE deal_id = ? \
         AND NOT (exchange_rate <=> ? AND amount_base <=> ROUND(total * ?, 2))",
    )
    .bind(rate)
    .bind(rate)
    .bind(deal_id.to_string())
    .bind(rate)
    .bind(rate)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn sync_open_currency_rate(
    tx: &mut MySqlTx,
    currency: &str,
    rate: Option<Decimal>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE deal_items i JOIN deals d ON d.id = i.deal_id \
         SET i.exchange_rate = ?, i.amount_base = ROUND(i.total * ?, 2) \
         WHERE d.currency = ? AND d.status = ? AND d.deleted_at IS NULL \
         AND NOT (i.exchange_rate <=> ? AND i.amount_base <=> ROUND(i.total * ?, 2))",
    )
    .bind(rate)
    .bind(rate)
    .bind(currency)
    .bind(DealStatus::Open.as_str())
    .bind(rate)
    .bind(rate)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
