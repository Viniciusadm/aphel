use aphel_contract_crm::CrmRecordTypes;
use std::collections::HashMap;

use serde_json::Value;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    BundleItem, BundlePricing, CatalogProduct, NewPriceListItem, PriceList, PriceListAppliesTo,
    PriceListCandidate, PriceListFilter, PriceListItem, ProductBundle, RoleDiscountLimit,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    Filter, FilterOperator, ID_FIELD, RecordQuery, RecordQueryEngine, in_saved_filter,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::access::{PermissionSet, RoleKey};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::MySqlRecordQueryPorts;
use aphel_kernel::infrastructure::repositories::mapper;

const LIST_COLUMNS: &str = "l.id, l.`key`, l.name, l.description, l.currency, l.applies_to, \
     l.segment_id, l.valid_from, l.valid_to, l.is_active, l.is_default, l.created_by, \
     l.created_at, l.updated_at, \
     (SELECT COUNT(*) FROM price_list_items i WHERE i.price_list_id = l.id) AS item_count";

const ITEM_SELECT: &str = "SELECT i.id, i.product_id, i.min_quantity, i.unit_price, i.position, \
     p.name AS product_name, p.code AS product_code, p.price AS product_price, \
     p.currency AS product_currency \
     FROM price_list_items i JOIN products p ON p.id = i.product_id \
     AND p.deleted_at IS NULL \
     WHERE TRUE";

pub(crate) fn push_uuid_list(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|text| Uuid::parse_str(&text).map_err(|_| mapper::invalid_column(column, &text)))
        .transpose()
}

fn list_from_row(row: &MySqlRow) -> Result<PriceList, AppError> {
    let applies_to: String = row.try_get("applies_to")?;
    let created_by: Option<String> = row.try_get("created_by")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let item_count: i64 = row.try_get("item_count")?;
    Ok(PriceList {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        currency: row.try_get("currency")?,
        applies_to: PriceListAppliesTo::parse(&applies_to)
            .ok_or_else(|| mapper::invalid_column("price_lists.applies_to", &applies_to))?,
        segment_id: optional_uuid(row, "segment_id")?,
        company_ids: Vec::new(),
        valid_from: row.try_get::<Option<NaiveDate>, _>("valid_from")?,
        valid_to: row.try_get::<Option<NaiveDate>, _>("valid_to")?,
        is_active: row.try_get("is_active")?,
        is_default: row.try_get("is_default")?,
        item_count: item_count.max(0) as u64,
        created_by: created_by.and_then(|id| UserId::new(id).ok()),
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn item_from_row(row: &MySqlRow) -> Result<PriceListItem, AppError> {
    Ok(PriceListItem {
        id: uuid_column(row, "id")?,
        product_id: uuid_column(row, "product_id")?,
        product_name: row.try_get("product_name")?,
        product_code: row.try_get("product_code")?,
        product_price: row.try_get::<Decimal, _>("product_price")?,
        product_currency: row.try_get("product_currency")?,
        min_quantity: row.try_get::<Decimal, _>("min_quantity")?.normalize(),
        unit_price: row.try_get::<Decimal, _>("unit_price")?,
        position: row.try_get("position")?,
    })
}

#[derive(Clone)]
pub(crate) struct PriceListStore {
    pool: MySqlPool,
}

impl PriceListStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(format!(
            "SELECT {LIST_COLUMNS} FROM price_lists l WHERE TRUE"
        ))
    }

    async fn with_companies(&self, mut lists: Vec<PriceList>) -> Result<Vec<PriceList>, AppError> {
        let ids: Vec<Uuid> = lists
            .iter()
            .filter(|list| list.applies_to == PriceListAppliesTo::Organizations)
            .map(|list| list.id)
            .collect();
        if ids.is_empty() {
            return Ok(lists);
        }
        let mut builder = QueryBuilder::new(
            "SELECT price_list_id, company_id FROM price_list_companies WHERE TRUE",
        );
        builder.push(" AND price_list_id");
        push_uuid_list(&mut builder, &ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut organizations: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for row in &rows {
            organizations
                .entry(uuid_column(row, "price_list_id")?)
                .or_default()
                .push(uuid_column(row, "company_id")?);
        }
        for list in &mut lists {
            if let Some(ids) = organizations.remove(&list.id) {
                list.company_ids = ids;
            }
        }
        Ok(lists)
    }

    fn push_filter(builder: &mut QueryBuilder<MySql>, filter: &PriceListFilter) {
        if let Some(currency) = &filter.currency {
            builder
                .push(" AND l.currency = ")
                .push_bind(currency.clone());
        }
        if let Some(active) = filter.active {
            builder.push(" AND l.is_active = ").push_bind(active);
        }
        if let Some(search) = &filter.search {
            let pattern = format!(
                "%{}%",
                search
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            builder
                .push(" AND (l.name LIKE ")
                .push_bind(pattern.clone())
                .push(" OR l.`key` LIKE ")
                .push_bind(pattern)
                .push(")");
        }
    }

    async fn existing_records(
        &self,
        ctx: &Ctx<'_>,
        record_type: RecordType,
        ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        let mut existing = Vec::new();
        for batch in ids.chunks(1000) {
            existing.extend(
                aphel_sdk::records::visible_record_ids(
                    &self.pool,
                    ctx.registry(),
                    ctx.scope,
                    &record_type,
                    batch,
                )
                .await?,
            );
        }
        Ok(existing)
    }
}

impl PriceListStore {
    pub(crate) async fn list(
        &self,
        filter: &PriceListFilter,
        window: ListWindow,
    ) -> Result<Listing<PriceList>, AppError> {
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM price_lists l WHERE TRUE");
        Self::push_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = Self::select();
        Self::push_filter(&mut builder, filter);
        builder
            .push(" ORDER BY l.is_default DESC, l.name, l.id LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let lists = rows
            .iter()
            .map(list_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items: self.with_companies(lists).await?,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn all(&self) -> Result<Vec<PriceList>, AppError> {
        let mut builder = Self::select();
        builder.push(" ORDER BY l.name, l.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let lists = rows
            .iter()
            .map(list_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.with_companies(lists).await
    }

    pub(crate) async fn count(&self) -> Result<u64, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM price_lists")
            .fetch_one(&self.pool)
            .await?;
        Ok(total.max(0) as u64)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<PriceList>, AppError> {
        let mut builder = Self::select();
        builder.push(" AND l.id = ").push_bind(id.to_string());
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let list = list_from_row(&row)?;
        Ok(self.with_companies(vec![list]).await?.pop())
    }

    pub(crate) async fn find_by_key(&self, key: &str) -> Result<Option<PriceList>, AppError> {
        let mut builder = Self::select();
        builder.push(" AND l.`key` = ").push_bind(key.to_owned());
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let list = list_from_row(&row)?;
        Ok(self.with_companies(vec![list]).await?.pop())
    }

    pub(crate) async fn candidates(
        &self,
        currency: &str,
        on: NaiveDate,
        company_id: Option<Uuid>,
    ) -> Result<Vec<PriceListCandidate>, AppError> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT {LIST_COLUMNS}, EXISTS(SELECT 1 FROM price_list_companies c \
             WHERE c.price_list_id = l.id AND c.company_id = "
        ));
        builder.push_bind(company_id.map(|id| id.to_string()));
        builder.push(") AS matches_company FROM price_lists l WHERE TRUE");
        builder
            .push(" AND l.is_active = TRUE AND l.currency = ")
            .push_bind(currency.to_owned())
            .push(" AND (l.valid_from IS NULL OR l.valid_from <= ")
            .push_bind(on)
            .push(") AND (l.valid_to IS NULL OR l.valid_to >= ")
            .push_bind(on)
            .push(") ORDER BY l.created_at, l.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(PriceListCandidate {
                    list: list_from_row(row)?,
                    matches_company: row.try_get::<bool, _>("matches_company")?,
                })
            })
            .collect()
    }

    pub(crate) async fn items(&self, price_list_id: Uuid) -> Result<Vec<PriceListItem>, AppError> {
        let mut builder = QueryBuilder::new(ITEM_SELECT);
        builder
            .push(" AND i.price_list_id = ")
            .push_bind(price_list_id.to_string())
            .push(" ORDER BY i.position, p.name, i.min_quantity");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn product_items(
        &self,
        price_list_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<PriceListItem>, AppError> {
        let mut builder = QueryBuilder::new(ITEM_SELECT);
        builder
            .push(" AND i.price_list_id = ")
            .push_bind(price_list_id.to_string())
            .push(" AND i.product_id = ")
            .push_bind(product_id.to_string())
            .push(" ORDER BY i.min_quantity");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn organization_currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| {
            aphel_kernel::domain::organization_profile::DEFAULT_CURRENCY.to_owned()
        }))
    }

    pub(crate) async fn currency_enabled(&self, currency: &str) -> Result<bool, AppError> {
        let base: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM organization_settings WHERE id = 1 AND currency = ?)",
        )
        .bind(currency)
        .fetch_one(&self.pool)
        .await?;
        if base {
            return Ok(true);
        }
        let Some(rates) = aphel_kernel::modules::app()
            .services()
            .get::<dyn aphel_sdk::money::MoneyRates>()
        else {
            return Ok(false);
        };
        rates.currency_enabled(&self.pool, currency).await
    }

    pub(crate) async fn existing_companies(
        &self,
        ctx: &Ctx<'_>,
        company_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        self.existing_records(
            ctx,
            aphel_kernel::application::shared::RecordType::COMPANY,
            company_ids,
        )
        .await
    }

    pub(crate) async fn existing_products(
        &self,
        ctx: &Ctx<'_>,
        product_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        self.existing_records(
            ctx,
            aphel_kernel::application::shared::RecordType::PRODUCT,
            product_ids,
        )
        .await
    }

    pub(crate) async fn products_by_code(
        &self,
        codes: &[String],
    ) -> Result<Vec<(String, Uuid)>, AppError> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(
            "SELECT active_code, id FROM products WHERE deleted_at IS NULL AND active_code IN (",
        );
        let mut separated = builder.separated(", ");
        for code in codes {
            separated.push_bind(code.clone());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| Ok((row.try_get("active_code")?, uuid_column(row, "id")?)))
            .collect()
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, list: &PriceList) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO price_lists (id, `key`, name, description, currency, \
             applies_to, segment_id, valid_from, valid_to, is_active, is_default, created_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(list.id.to_string())
        .bind(&list.key)
        .bind(&list.name)
        .bind(&list.description)
        .bind(&list.currency)
        .bind(list.applies_to.as_str())
        .bind(list.segment_id.map(|id| id.to_string()))
        .bind(list.valid_from)
        .bind(list.valid_to)
        .bind(list.is_active)
        .bind(list.is_default)
        .bind(list.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(list.created_at))
        .bind(mapper::naive(list.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut MySqlTx, list: &PriceList) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE price_lists SET `key` = ?, name = ?, description = ?, currency = ?, \
             applies_to = ?, segment_id = ?, valid_from = ?, valid_to = ?, is_active = ?, \
             is_default = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&list.key)
        .bind(&list.name)
        .bind(&list.description)
        .bind(&list.currency)
        .bind(list.applies_to.as_str())
        .bind(list.segment_id.map(|id| id.to_string()))
        .bind(list.valid_from)
        .bind(list.valid_to)
        .bind(list.is_active)
        .bind(list.is_default)
        .bind(mapper::naive(list.updated_at))
        .bind(list.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn clear_default(
        &self,
        tx: &mut MySqlTx,
        currency: &str,
        except: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE price_lists SET is_default = FALSE \
             WHERE currency = ? AND is_default = TRUE AND id <> ?",
        )
        .bind(currency)
        .bind(except.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_companies(
        &self,
        tx: &mut MySqlTx,
        price_list_id: Uuid,
        company_ids: &[Uuid],
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM price_list_companies WHERE price_list_id = ?")
            .bind(price_list_id.to_string())
            .execute(&mut **tx)
            .await?;
        if company_ids.is_empty() {
            return Ok(());
        }
        let mut builder =
            QueryBuilder::new("INSERT INTO price_list_companies (price_list_id, company_id) ");
        builder.push_values(company_ids, |mut values, company_id| {
            values
                .push_bind(price_list_id.to_string())
                .push_bind(company_id.to_string());
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn replace_items(
        &self,
        tx: &mut MySqlTx,
        price_list_id: Uuid,
        items: &[NewPriceListItem],
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM price_list_items WHERE price_list_id = ?")
            .bind(price_list_id.to_string())
            .execute(&mut **tx)
            .await?;
        for chunk in items.chunks(500) {
            let mut builder = QueryBuilder::new(
                "INSERT INTO price_list_items (id, price_list_id, product_id, \
                 min_quantity, unit_price, position) ",
            );
            builder.push_values(chunk, |mut values, item| {
                values
                    .push_bind(item.id.to_string())
                    .push_bind(price_list_id.to_string())
                    .push_bind(item.product_id.to_string())
                    .push_bind(item.min_quantity)
                    .push_bind(item.unit_price)
                    .push_bind(item.position);
            });
            builder.build().execute(&mut **tx).await?;
        }
        Ok(())
    }

    pub(crate) async fn touch(
        &self,
        tx: &mut MySqlTx,
        price_list_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE price_lists SET updated_at = ? WHERE id = ?")
            .bind(mapper::naive(at))
            .bind(price_list_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut MySqlTx,
        price_list_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM price_lists WHERE id = ?")
            .bind(price_list_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct DiscountLimitStore {
    pool: MySqlPool,
}

impl DiscountLimitStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl DiscountLimitStore {
    pub(crate) async fn list(&self) -> Result<Vec<RoleDiscountLimit>, AppError> {
        let rows = sqlx::query(
            "SELECT r.id, r.`key`, r.name, l.max_discount_percent FROM roles r \
             LEFT JOIN role_discount_limits l ON l.role_id = r.id \
             ORDER BY r.is_system DESC, r.name, r.id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(RoleDiscountLimit {
                    role_id: uuid_column(row, "id")?,
                    role_key: row.try_get("key")?,
                    role_name: row.try_get("name")?,
                    max_discount_percent: row
                        .try_get::<Option<Decimal>, _>("max_discount_percent")?
                        .map(|value| value.normalize()),
                })
            })
            .collect()
    }

    pub(crate) async fn for_user(&self, user_id: &UserId) -> Result<Option<Decimal>, AppError> {
        let limit: Option<Decimal> = sqlx::query_scalar(
            "SELECT l.max_discount_percent FROM users u \
             JOIN role_discount_limits l ON l.role_id = u.role_id \
             WHERE u.id = ?",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(limit.map(|value| value.normalize()))
    }

    pub(crate) async fn set(
        &self,
        tx: &mut MySqlTx,
        role_id: Uuid,
        max_discount_percent: Option<Decimal>,
        by: Option<&UserId>,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        match max_discount_percent {
            None => {
                sqlx::query("DELETE FROM role_discount_limits WHERE role_id = ?")
                    .bind(role_id.to_string())
                    .execute(&mut **tx)
                    .await?;
            }
            Some(percent) => {
                sqlx::query(
                    "INSERT INTO role_discount_limits (role_id, max_discount_percent, \
                     updated_by, updated_at) VALUES (?, ?, ?, ?) \
                     ON DUPLICATE KEY UPDATE max_discount_percent = VALUES(max_discount_percent), \
                     updated_by = VALUES(updated_by), updated_at = VALUES(updated_at)",
                )
                .bind(role_id.to_string())
                .bind(percent)
                .bind(by.map(UserId::as_str))
                .bind(mapper::naive(at))
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }
}

pub(crate) struct SegmentMembership {
    query: MySqlRecordQueryPorts,
}

impl SegmentMembership {
    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self { query: ctx.query() }
    }
}

fn evaluation_scope(scope: &AccessScope) -> AccessScope {
    AccessScope::new(
        scope.user_id().clone(),
        RoleKey::system(),
        scope.team_ids().to_vec(),
        PermissionSet::everything(),
    )
    .with_enabled_modules(scope.enabled_modules().to_vec())
}

impl SegmentMembership {
    pub(crate) async fn contains(
        &self,
        scope: &AccessScope,
        segment_id: Uuid,
        company_id: Uuid,
    ) -> Result<bool, AppError> {
        let evaluation = evaluation_scope(scope);
        let result = RecordQueryEngine::new(&self.query)
            .query(
                &evaluation,
                RecordType::COMPANY.as_str(),
                RecordQuery {
                    filter: Some(Filter::and(vec![
                        in_saved_filter(segment_id),
                        Filter::condition(
                            ID_FIELD,
                            FilterOperator::Eq,
                            Value::String(company_id.to_string()),
                        ),
                    ])),
                    window: ListWindow {
                        offset: 0,
                        limit: 1,
                    },
                    ..RecordQuery::default()
                },
            )
            .await;
        match result {
            Ok(listing) => Ok(listing.total > 0),
            Err(AppError::InvalidFields { .. } | AppError::Validation(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[derive(Clone)]
pub(crate) struct BundleStore {
    pool: MySqlPool,
}

impl BundleStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl BundleStore {
    pub(crate) async fn find(&self, product_id: Uuid) -> Result<Option<ProductBundle>, AppError> {
        let row =
            sqlx::query("SELECT pricing, updated_at FROM product_bundles WHERE product_id = ?")
                .bind(product_id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let pricing: String = row.try_get("pricing")?;
        let updated_at: NaiveDateTime = row.try_get("updated_at")?;
        let components = sqlx::query(
            "SELECT component_product_id, quantity, position FROM product_bundle_components \
             WHERE bundle_product_id = ? ORDER BY position, component_product_id",
        )
        .bind(product_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| {
            Ok(BundleItem {
                product_id: uuid_column(row, "component_product_id")?,
                quantity: row.try_get::<Decimal, _>("quantity")?.normalize(),
                position: row.try_get("position")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Some(ProductBundle {
            product_id,
            pricing: BundlePricing::parse(&pricing)
                .ok_or_else(|| mapper::invalid_column("product_bundles.pricing", &pricing))?,
            components,
            updated_at: mapper::utc(updated_at),
        }))
    }

    pub(crate) async fn products(
        &self,
        ctx: &Ctx<'_>,
        product_ids: &[Uuid],
    ) -> Result<Vec<CatalogProduct>, AppError> {
        Ok(aphel_sdk::line_items::catalog_items(ctx, product_ids)
            .await?
            .into_values()
            .map(|item| CatalogProduct {
                id: item.id,
                name: item.name,
                code: item.code,
                price: item.price,
                currency: item.currency,
                is_active: item.active,
            })
            .collect())
    }

    pub(crate) async fn bundled(&self, product_ids: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder =
            QueryBuilder::new("SELECT product_id FROM product_bundles WHERE product_id");
        push_uuid_list(&mut builder, product_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| uuid_column(row, "product_id"))
            .collect()
    }

    pub(crate) async fn is_component(&self, product_id: Uuid) -> Result<bool, AppError> {
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM product_bundle_components c \
             JOIN products p ON p.id = c.bundle_product_id AND p.deleted_at IS NULL \
             WHERE c.component_product_id = ?)",
        )
        .bind(product_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(used)
    }

    pub(crate) async fn timezone(&self) -> Result<String, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(timezone.unwrap_or_default())
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        bundle: &ProductBundle,
        by: Option<&UserId>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO product_bundles (product_id, pricing, updated_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE pricing = VALUES(pricing), \
             updated_by = VALUES(updated_by), updated_at = VALUES(updated_at)",
        )
        .bind(bundle.product_id.to_string())
        .bind(bundle.pricing.as_str())
        .bind(by.map(UserId::as_str))
        .bind(mapper::naive(bundle.updated_at))
        .bind(mapper::naive(bundle.updated_at))
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM product_bundle_components WHERE bundle_product_id = ?")
            .bind(bundle.product_id.to_string())
            .execute(&mut **tx)
            .await?;
        if bundle.components.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::new(
            "INSERT INTO product_bundle_components (bundle_product_id, component_product_id, \
             quantity, position) ",
        );
        builder.push_values(&bundle.components, |mut values, item| {
            values
                .push_bind(bundle.product_id.to_string())
                .push_bind(item.product_id.to_string())
                .push_bind(item.quantity)
                .push_bind(item.position);
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn remove(&self, tx: &mut MySqlTx, product_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM product_bundles WHERE product_id = ?")
            .bind(product_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
