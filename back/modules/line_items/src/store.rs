use aphel_sdk::platform::application::shared::{RecordRef, RecordType};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::LineItemSettings;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::line_items::{AmountMode, DiscountKind, LineData, LineItem, Recurrence};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper::{
    instant, invalid_column, naive, optional_user, optional_uuid,
};

const ITEM_SELECT: &str = "SELECT id, record_type, record_id, product_id, name, unit, quantity, unit_price, \
     discount_type, discount_value, tax_percent, recurrence, term_months, total, amount_base, exchange_rate, \
     position, created_by, created_at, updated_at \
     FROM line_items WHERE record_type = ? AND record_id = ? ORDER BY position, id";

fn item_from_row(row: &MySqlRow) -> Result<LineItem, AppError> {
    let discount: String = row.try_get("discount_type")?;
    let recurrence: String = row.try_get("recurrence")?;
    Ok(LineItem {
        id: uuid_column(row, "id")?,
        record: aphel_sdk::platform::application::shared::RecordRef::new(
            RecordType::parse(row.try_get::<String, _>("record_type")?.as_str())?,
            uuid_column(row, "record_id")?,
        ),
        data: LineData {
            product_id: optional_uuid(row, "product_id")?,
            name: row.try_get("name")?,
            unit: row.try_get("unit")?,
            quantity: row.try_get::<Decimal, _>("quantity")?.normalize(),
            unit_price: row.try_get("unit_price")?,
            discount_kind: DiscountKind::parse(&discount)
                .ok_or_else(|| invalid_column("line_items.discount_type", &discount))?,
            discount_value: row.try_get("discount_value")?,
            tax_percent: row.try_get("tax_percent")?,
            recurrence: Recurrence::parse(&recurrence)
                .ok_or_else(|| invalid_column("line_items.recurrence", &recurrence))?,
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

#[derive(Clone)]
pub(crate) struct LineItemStore {
    pool: MySqlPool,
}

impl LineItemStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list(&self, record: &RecordRef) -> Result<Vec<LineItem>, AppError> {
        let rows = sqlx::query(ITEM_SELECT)
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn list_locked(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<Vec<LineItem>, AppError> {
        let rows = sqlx::query(ITEM_SELECT)
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .fetch_all(&mut **tx)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, item: &LineItem) -> Result<(), AppError> {
        let data = &item.data;
        sqlx::query(
            "INSERT INTO line_items (id, record_type, record_id, product_id, name, unit, quantity, \
             unit_price, discount_type, discount_value, tax_percent, recurrence, term_months, total, amount_base, \
             exchange_rate, position, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(item.record.record_type.as_str())
        .bind(item.record.record_id.to_string())
        .bind(data.product_id.map(|id| id.to_string()))
        .bind(&data.name)
        .bind(&data.unit)
        .bind(data.quantity)
        .bind(data.unit_price)
        .bind(data.discount_kind.as_str())
        .bind(data.discount_value)
        .bind(data.tax_percent)
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

    pub(crate) async fn update(&self, tx: &mut MySqlTx, item: &LineItem) -> Result<(), AppError> {
        let data = &item.data;
        sqlx::query(
            "UPDATE line_items SET product_id = ?, name = ?, unit = ?, quantity = ?, unit_price = ?, \
             discount_type = ?, discount_value = ?, tax_percent = ?, recurrence = ?, term_months = ?, total = ?, \
             amount_base = ?, exchange_rate = ?, updated_at = ? \
             WHERE record_type = ? AND record_id = ? AND id = ?",
        )
        .bind(data.product_id.map(|id| id.to_string()))
        .bind(&data.name)
        .bind(&data.unit)
        .bind(data.quantity)
        .bind(data.unit_price)
        .bind(data.discount_kind.as_str())
        .bind(data.discount_value)
        .bind(data.tax_percent)
        .bind(data.recurrence.as_str())
        .bind(data.term_months)
        .bind(item.total)
        .bind(item.amount_base)
        .bind(item.exchange_rate)
        .bind(naive(item.updated_at))
        .bind(item.record.record_type.as_str())
        .bind(item.record.record_id.to_string())
        .bind(item.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        item_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM line_items WHERE record_type = ? AND record_id = ? AND id = ?")
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .bind(item_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn set_positions(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        positions: &[(Uuid, i32)],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (item_id, position) in positions {
            sqlx::query(
                "UPDATE line_items SET position = ?, updated_at = ? \
                 WHERE record_type = ? AND record_id = ? AND id = ?",
            )
            .bind(position)
            .bind(naive(at))
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .bind(item_id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn settings(&self) -> Result<LineItemSettings, AppError> {
        let settings = aphel_sdk::platform::modules::module_settings(&self.pool, crate::KEY).await?;
        let mode = settings
            .get(crate::SETTING_AMOUNT_MODE)
            .and_then(|value| value.as_str())
            .and_then(AmountMode::parse)
            .unwrap_or_default();
        Ok(LineItemSettings { amount_mode: mode })
    }

    pub(crate) async fn save_amount_mode(
        &self,
        tx: &mut MySqlTx,
        mode: AmountMode,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE installed_modules \
             SET settings = JSON_SET(COALESCE(settings, JSON_OBJECT()), '$.amount_mode', ?) \
             WHERE module_key = ?",
        )
        .bind(mode.as_str())
        .bind(crate::KEY)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn amount_managed(&self, record: &RecordRef) -> Result<bool, AppError> {
        if self.settings().await?.amount_mode != AmountMode::Items {
            return Ok(false);
        }
        let has_items: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM line_items WHERE record_type = ? AND record_id = ?)",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(has_items)
    }
}

pub(crate) async fn rebase_record_amounts(
    tx: &mut MySqlTx,
    record: &RecordRef,
    factor: Decimal,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE line_items SET amount_base = ROUND(amount_base * ?, 2), \
         exchange_rate = ROUND(exchange_rate * ?, 10) \
         WHERE record_type = ? AND record_id = ? AND amount_base IS NOT NULL",
    )
    .bind(factor)
    .bind(factor)
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn sync_record_rate(
    tx: &mut MySqlTx,
    record: &RecordRef,
    rate: Option<Decimal>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE line_items SET exchange_rate = ?, amount_base = ROUND(total * ?, 2) \
         WHERE record_type = ? AND record_id = ? \
         AND NOT (exchange_rate <=> ? AND amount_base <=> ROUND(total * ?, 2))",
    )
    .bind(rate)
    .bind(rate)
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .bind(rate)
    .bind(rate)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
