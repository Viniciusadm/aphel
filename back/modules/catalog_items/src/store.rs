use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{Product, ProductData, Recurrence};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mapper::{instant, optional_user, push_ids};

const PRODUCT_SELECT: &str = "SELECT r.id, r.name, r.code, r.description, r.price, r.currency, \
     r.unit, r.recurrence, r.term_months, r.is_active, r.created_by, r.created_at, r.updated_at \
     FROM products r WHERE r.deleted_at IS NULL";

fn product_from_row(row: &MySqlRow) -> Result<Product, AppError> {
    let recurrence: String = row.try_get("recurrence")?;
    Ok(Product {
        id: uuid_column(row, "id")?,
        data: ProductData {
            name: row.try_get("name")?,
            code: row.try_get("code")?,
            description: row.try_get("description")?,
            price: row.try_get::<Decimal, _>("price")?,
            currency: row.try_get("currency")?,
            unit: row.try_get("unit")?,
            recurrence: Recurrence::parse(&recurrence)
                .ok_or_else(|| mapper::invalid_column("products.recurrence", &recurrence))?,
            term_months: row.try_get::<Option<u16>, _>("term_months")?,
            is_active: row.try_get("is_active")?,
        },
        created_by: optional_user(row, "created_by")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

#[derive(Clone)]
pub struct ProductStore {
    pool: MySqlPool,
}

impl ProductStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(PRODUCT_SELECT)
    }

    pub async fn find(&self, product_id: Uuid) -> Result<Option<Product>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND r.id = ")
            .push_bind(product_id.to_string());
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref().map(product_from_row).transpose()
    }

    pub(crate) async fn find_many(&self, product_ids: &[Uuid]) -> Result<Vec<Product>, AppError> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::select();
        builder.push(" AND r.id");
        push_ids(&mut builder, product_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(product_from_row).collect()
    }

    pub(crate) async fn code_taken(
        &self,
        code: &str,
        except: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM products WHERE active_code = ? \
             AND (? IS NULL OR id <> ?))",
        )
        .bind(code)
        .bind(except.map(|id| id.to_string()))
        .bind(except.map(|id| id.to_string()))
        .fetch_one(&self.pool)
        .await?;
        Ok(taken)
    }

    pub(crate) async fn organization_currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| {
            aphel_sdk::platform::domain::organization_profile::DEFAULT_CURRENCY.to_owned()
        }))
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, product: &Product) -> Result<(), AppError> {
        let data = &product.data;
        sqlx::query(
            "INSERT INTO products (id, name, code, description, price, currency, \
             unit, recurrence, term_months, is_active, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(product.id.to_string())
        .bind(&data.name)
        .bind(&data.code)
        .bind(&data.description)
        .bind(data.price)
        .bind(&data.currency)
        .bind(&data.unit)
        .bind(data.recurrence.as_str())
        .bind(data.term_months)
        .bind(data.is_active)
        .bind(product.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(product.created_at))
        .bind(mapper::naive(product.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        product_id: Uuid,
        data: &ProductData,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE products SET name = ?, code = ?, description = ?, price = ?, currency = ?, \
             unit = ?, recurrence = ?, term_months = ?, is_active = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&data.name)
        .bind(&data.code)
        .bind(&data.description)
        .bind(data.price)
        .bind(&data.currency)
        .bind(&data.unit)
        .bind(data.recurrence.as_str())
        .bind(data.term_months)
        .bind(data.is_active)
        .bind(mapper::naive(updated_at))
        .bind(product_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn soft_delete(
        &self,
        tx: &mut MySqlTx,
        product_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE products SET deleted_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(deleted_at))
        .bind(product_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
