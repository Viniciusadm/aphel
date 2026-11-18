pub(crate) mod reference_cleanup;

use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::platform::application::shared::RecordType;
use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::QUOTE;
use crate::domain::{
    LineBreakdown, Quote, QuoteAcceptance, QuoteDiscount, QuoteItem, QuoteItemComponent,
    QuoteItemData, QuoteOrganization, QuotePeople, QuoteSettings, QuoteStatus, QuoteVersionSummary,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::line_items::{DiscountKind, Recurrence};
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::query::compiler::push_record_scope_as;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const QUOTE_SELECT: &str = "SELECT r.id, r.deal_id, r.number, r.version, r.is_current, \
     r.title, r.status, r.owner_id, r.contact_id, r.company_id, r.currency, r.price_list_id, \
     r.valid_until, r.terms, r.notes, r.discount_type, r.discount_value, r.subtotal, \
     r.discount_total, r.tax_total, r.total, r.exceeds_discount_limit, r.public_key, r.sent_at, \
     r.sent_by, r.viewed_at, r.accepted_at, r.accepted_name, r.accepted_email, r.accepted_ip, \
     r.accepted_user_agent, r.accepted_by, r.rejected_at, r.rejection_reason, r.expired_at, r.deal_synced_at, \
     r.deal_sync_error, r.created_by, r.created_at, r.updated_at \
     FROM quotes r WHERE r.deleted_at IS NULL";

const ITEM_SELECT: &str = "SELECT id, quote_id, position, product_id, name, description, unit, quantity, \
     unit_price, discount_type, discount_value, tax_percent, recurrence, term_months, components, \
     gross, discount_amount, total_discount_share, tax_amount, total \
     FROM quote_items WHERE quote_id = ? ORDER BY position, created_at";

fn uuid(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value)))
        .transpose()
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|id| mapper::typed_id(id, UserId::new))
        .transpose()
}

fn instant(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    Ok(mapper::utc(row.try_get::<NaiveDateTime, _>(column)?))
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row
        .try_get::<Option<NaiveDateTime>, _>(column)?
        .map(mapper::utc))
}

fn optional_naive(value: Option<DateTime<Utc>>) -> Option<NaiveDateTime> {
    value.map(mapper::naive)
}

fn discount_kind(row: &MySqlRow, column: &str) -> Result<DiscountKind, AppError> {
    let value: String = row.try_get(column)?;
    DiscountKind::parse(&value).ok_or_else(|| mapper::invalid_column(column, &value))
}

fn quote_from_row(row: &MySqlRow) -> Result<Quote, AppError> {
    let status: String = row.try_get("status")?;
    let accepted_name: Option<String> = row.try_get("accepted_name")?;
    let acceptance = match accepted_name {
        Some(name) => Some(QuoteAcceptance {
            name,
            email: row
                .try_get::<Option<String>, _>("accepted_email")?
                .unwrap_or_default(),
            ip: row.try_get("accepted_ip")?,
            user_agent: row.try_get("accepted_user_agent")?,
            recorded_by: optional_user(row, "accepted_by")?,
        }),
        None => None,
    };
    Ok(Quote {
        id: uuid(row, "id")?,
        deal_id: uuid(row, "deal_id")?,
        number: row.try_get("number")?,
        version: row.try_get("version")?,
        is_current: row.try_get("is_current")?,
        title: row.try_get("title")?,
        status: QuoteStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("quotes.status", &status))?,
        owner_id: optional_user(row, "owner_id")?,
        contact_id: optional_uuid(row, "contact_id")?,
        company_id: optional_uuid(row, "company_id")?,
        currency: row.try_get("currency")?,
        price_list_id: optional_uuid(row, "price_list_id")?,
        valid_until: row.try_get::<Option<NaiveDate>, _>("valid_until")?,
        terms: row.try_get("terms")?,
        notes: row.try_get("notes")?,
        discount: QuoteDiscount {
            kind: discount_kind(row, "discount_type")?,
            value: row.try_get("discount_value")?,
        },
        subtotal: row.try_get("subtotal")?,
        discount_total: row.try_get("discount_total")?,
        tax_total: row.try_get("tax_total")?,
        total: row.try_get("total")?,
        exceeds_discount_limit: row.try_get("exceeds_discount_limit")?,
        public_key: row.try_get("public_key")?,
        sent_at: optional_instant(row, "sent_at")?,
        sent_by: optional_user(row, "sent_by")?,
        viewed_at: optional_instant(row, "viewed_at")?,
        accepted_at: optional_instant(row, "accepted_at")?,
        rejected_at: optional_instant(row, "rejected_at")?,
        expired_at: optional_instant(row, "expired_at")?,
        acceptance,
        rejection_reason: row.try_get("rejection_reason")?,
        deal_synced_at: optional_instant(row, "deal_synced_at")?,
        deal_sync_error: row.try_get("deal_sync_error")?,
        created_by: optional_user(row, "created_by")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

fn decimal_value(value: &Value) -> Decimal {
    match value {
        Value::String(text) => text.parse().unwrap_or_default(),
        Value::Number(number) => number.to_string().parse().unwrap_or_default(),
        _ => Decimal::ZERO,
    }
}

fn components_from_json(value: &Value) -> Vec<QuoteItemComponent> {
    value
        .as_array()
        .map(|components| {
            components
                .iter()
                .map(|component| QuoteItemComponent {
                    product_id: component
                        .get("product_id")
                        .and_then(Value::as_str)
                        .and_then(|id| Uuid::parse_str(id).ok()),
                    name: component
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    code: component
                        .get("code")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    quantity: component
                        .get("quantity")
                        .map(decimal_value)
                        .unwrap_or_default(),
                    unit_price: component
                        .get("unit_price")
                        .map(decimal_value)
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn components_json(components: &[QuoteItemComponent]) -> Value {
    Value::Array(
        components
            .iter()
            .map(|component| {
                json!({
                    "product_id": component.product_id.map(|id| id.to_string()),
                    "name": component.name,
                    "code": component.code,
                    "quantity": component.quantity.normalize().to_string(),
                    "unit_price": format!("{:.2}", component.unit_price),
                })
            })
            .collect(),
    )
}

fn item_from_row(row: &MySqlRow) -> Result<QuoteItem, AppError> {
    let recurrence: String = row.try_get("recurrence")?;
    let Json(components): Json<Value> = row.try_get("components")?;
    Ok(QuoteItem {
        id: uuid(row, "id")?,
        quote_id: uuid(row, "quote_id")?,
        position: row.try_get("position")?,
        data: QuoteItemData {
            product_id: optional_uuid(row, "product_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            unit: row.try_get("unit")?,
            quantity: row.try_get("quantity")?,
            unit_price: row.try_get("unit_price")?,
            discount_kind: discount_kind(row, "discount_type")?,
            discount_value: row.try_get("discount_value")?,
            tax_percent: row.try_get("tax_percent")?,
            recurrence: Recurrence::parse(&recurrence)
                .ok_or_else(|| mapper::invalid_column("quote_items.recurrence", &recurrence))?,
            term_months: row.try_get::<Option<u16>, _>("term_months")?,
            components: components_from_json(&components),
        },
        amounts: {
            let gross: Decimal = row.try_get("gross")?;
            let item_discount: Decimal = row.try_get("discount_amount")?;
            let share: Decimal = row.try_get("total_discount_share")?;
            let tax: Decimal = row.try_get("tax_amount")?;
            LineBreakdown {
                gross,
                item_discount,
                net: gross - item_discount,
                total_discount_share: share,
                taxable: gross - item_discount - share,
                tax,
                total: row.try_get("total")?,
            }
        },
    })
}

fn version_from_row(row: &MySqlRow) -> Result<QuoteVersionSummary, AppError> {
    let status: String = row.try_get("status")?;
    Ok(QuoteVersionSummary {
        id: uuid(row, "id")?,
        version: row.try_get("version")?,
        status: QuoteStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("quotes.status", &status))?,
        total: row.try_get("total")?,
        is_current: row.try_get("is_current")?,
        created_at: instant(row, "created_at")?,
    })
}

#[derive(Clone)]
pub(crate) struct QuoteStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl QuoteStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(QUOTE_SELECT)
    }

    fn push_scope(
        &self,
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
    ) -> Result<(), AppError> {
        let definition = self.registry.require(QUOTE.as_str())?;
        push_record_scope_as(builder, scope, definition, visibility, "r");
        Ok(())
    }
}

impl QuoteStore {
    pub(crate) async fn find(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        id: Uuid,
    ) -> Result<Option<Quote>, AppError> {
        let mut builder = Self::select();
        builder.push(" AND r.id = ").push_bind(id.to_string());
        self.push_scope(&mut builder, scope, visibility)?;
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref().map(quote_from_row).transpose()
    }

    pub(crate) async fn find_many(&self, ids: &[Uuid]) -> Result<Vec<Quote>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::select();
        builder.push(" AND r.id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(quote_from_row).collect()
    }

    pub(crate) async fn lock(&self, tx: &mut Tx, id: Uuid) -> Result<Option<Quote>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND r.id = ")
            .push_bind(id.to_string())
            .push(" FOR UPDATE");
        let row = builder.build().fetch_optional(&mut **tx).await?;
        row.as_ref().map(quote_from_row).transpose()
    }

    pub(crate) async fn items(&self, quote_id: Uuid) -> Result<Vec<QuoteItem>, AppError> {
        let rows = sqlx::query(ITEM_SELECT)
            .bind(quote_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn versions(&self, number: u32) -> Result<Vec<QuoteVersionSummary>, AppError> {
        let rows = sqlx::query(
            "SELECT id, version, status, total, is_current, created_at FROM quotes \
             WHERE number = ? AND deleted_at IS NULL ORDER BY version DESC",
        )
        .bind(number)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(version_from_row).collect()
    }

    pub(crate) async fn for_deal(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        deal_id: Uuid,
    ) -> Result<Vec<Quote>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND r.is_current = TRUE AND r.deal_id = ")
            .push_bind(deal_id.to_string());
        self.push_scope(&mut builder, scope, visibility)?;
        builder.push(" ORDER BY r.number DESC LIMIT 200");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(quote_from_row).collect()
    }

    pub(crate) async fn next_number(&self, tx: &mut Tx) -> Result<u32, AppError> {
        sqlx::query(
            "INSERT INTO quote_counters (id, last_number) VALUES (1, LAST_INSERT_ID(1)) \
             ON DUPLICATE KEY UPDATE last_number = LAST_INSERT_ID(last_number + 1), \
             updated_at = CURRENT_TIMESTAMP(6)",
        )
        .execute(&mut **tx)
        .await?;
        let number: u64 = sqlx::query_scalar("SELECT LAST_INSERT_ID()")
            .fetch_one(&mut **tx)
            .await?;
        u32::try_from(number)
            .map_err(|_| AppError::Internal("the quote counter overflowed".to_owned()))
    }

    pub(crate) async fn insert(&self, tx: &mut Tx, quote: &Quote) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO quotes (id, deal_id, number, version, is_current, title, \
             status, owner_id, contact_id, company_id, currency, price_list_id, valid_until, terms, \
             notes, discount_type, discount_value, subtotal, discount_total, tax_total, total, \
             exceeds_discount_limit, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(quote.id.to_string())
        .bind(quote.deal_id.to_string())
        .bind(quote.number)
        .bind(quote.version)
        .bind(quote.is_current)
        .bind(&quote.title)
        .bind(quote.status.as_str())
        .bind(quote.owner_id.as_ref().map(UserId::as_str))
        .bind(quote.contact_id.map(|id| id.to_string()))
        .bind(quote.company_id.map(|id| id.to_string()))
        .bind(&quote.currency)
        .bind(quote.price_list_id.map(|id| id.to_string()))
        .bind(quote.valid_until)
        .bind(&quote.terms)
        .bind(&quote.notes)
        .bind(quote.discount.kind.as_str())
        .bind(quote.discount.value)
        .bind(quote.subtotal)
        .bind(quote.discount_total)
        .bind(quote.tax_total)
        .bind(quote.total)
        .bind(quote.exceeds_discount_limit)
        .bind(quote.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(quote.created_at))
        .bind(mapper::naive(quote.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut Tx, quote: &Quote) -> Result<(), AppError> {
        let acceptance = quote.acceptance.as_ref();
        sqlx::query(
            "UPDATE quotes SET title = ?, status = ?, owner_id = ?, contact_id = ?, company_id = ?, \
             price_list_id = ?, valid_until = ?, terms = ?, notes = ?, discount_type = ?, \
             discount_value = ?, subtotal = ?, discount_total = ?, tax_total = ?, total = ?, \
             exceeds_discount_limit = ?, public_key = ?, sent_at = ?, sent_by = ?, viewed_at = ?, \
             accepted_at = ?, accepted_name = ?, accepted_email = ?, accepted_ip = ?, \
             accepted_user_agent = ?, accepted_by = ?, rejected_at = ?, rejection_reason = ?, expired_at = ?, \
             deal_synced_at = ?, deal_sync_error = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&quote.title)
        .bind(quote.status.as_str())
        .bind(quote.owner_id.as_ref().map(UserId::as_str))
        .bind(quote.contact_id.map(|id| id.to_string()))
        .bind(quote.company_id.map(|id| id.to_string()))
        .bind(quote.price_list_id.map(|id| id.to_string()))
        .bind(quote.valid_until)
        .bind(&quote.terms)
        .bind(&quote.notes)
        .bind(quote.discount.kind.as_str())
        .bind(quote.discount.value)
        .bind(quote.subtotal)
        .bind(quote.discount_total)
        .bind(quote.tax_total)
        .bind(quote.total)
        .bind(quote.exceeds_discount_limit)
        .bind(&quote.public_key)
        .bind(optional_naive(quote.sent_at))
        .bind(quote.sent_by.as_ref().map(UserId::as_str))
        .bind(optional_naive(quote.viewed_at))
        .bind(optional_naive(quote.accepted_at))
        .bind(acceptance.map(|acceptance| acceptance.name.as_str()))
        .bind(acceptance.map(|acceptance| acceptance.email.as_str()))
        .bind(acceptance.and_then(|acceptance| acceptance.ip.as_deref()))
        .bind(acceptance.and_then(|acceptance| acceptance.user_agent.as_deref()))
        .bind(
            acceptance
                .and_then(|acceptance| acceptance.recorded_by.as_ref())
                .map(UserId::as_str),
        )
        .bind(optional_naive(quote.rejected_at))
        .bind(&quote.rejection_reason)
        .bind(optional_naive(quote.expired_at))
        .bind(optional_naive(quote.deal_synced_at))
        .bind(&quote.deal_sync_error)
        .bind(mapper::naive(quote.updated_at))
        .bind(quote.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn replace_items(
        &self,
        tx: &mut Tx,
        quote_id: Uuid,
        items: &[QuoteItem],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM quote_items WHERE quote_id = ?")
            .bind(quote_id.to_string())
            .execute(&mut **tx)
            .await?;
        if items.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO quote_items (id, quote_id, position, product_id, name, \
             description, unit, quantity, unit_price, discount_type, discount_value, tax_percent, \
             recurrence, term_months, components, gross, discount_amount, total_discount_share, \
             tax_amount, total, created_at, updated_at) ",
        );
        let at = mapper::naive(at);
        builder.push_values(items, |mut values, item| {
            let data = &item.data;
            values
                .push_bind(item.id.to_string())
                .push_bind(quote_id.to_string())
                .push_bind(item.position)
                .push_bind(data.product_id.map(|id| id.to_string()))
                .push_bind(data.name.clone())
                .push_bind(data.description.clone())
                .push_bind(data.unit.clone())
                .push_bind(data.quantity)
                .push_bind(data.unit_price)
                .push_bind(data.discount_kind.as_str())
                .push_bind(data.discount_value)
                .push_bind(data.tax_percent)
                .push_bind(data.recurrence.as_str())
                .push_bind(data.term_months)
                .push_bind(Json(components_json(&data.components)))
                .push_bind(item.amounts.gross)
                .push_bind(item.amounts.item_discount)
                .push_bind(item.amounts.total_discount_share)
                .push_bind(item.amounts.tax)
                .push_bind(item.amounts.total)
                .push_bind(at)
                .push_bind(at);
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn retire_versions(
        &self,
        tx: &mut Tx,
        number: u32,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE quotes SET is_current = FALSE, updated_at = ? \
             WHERE number = ? AND is_current = TRUE AND deleted_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(number)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_number(
        &self,
        tx: &mut Tx,
        number: u32,
        at: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM quotes WHERE number = ? \
             AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(number)
        .fetch_all(&mut **tx)
        .await?;
        sqlx::query(
            "UPDATE quotes SET deleted_at = ?, public_key = NULL \
             WHERE number = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(number)
        .execute(&mut **tx)
        .await?;
        ids.iter()
            .map(|id| Uuid::parse_str(id).map_err(|_| mapper::invalid_column("quotes.id", id)))
            .collect()
    }

    pub(crate) async fn by_public_key(&self, public_key: &str) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query("SELECT id FROM quotes WHERE public_key = ? AND deleted_at IS NULL")
            .bind(public_key)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| uuid(&row, "id")).transpose()
    }

    pub(crate) async fn organization(&self) -> Result<QuoteOrganization, AppError> {
        let row = sqlx::query(
            "SELECT name, timezone, currency, locale, logo_file_id, primary_color FROM organization_settings \
             WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(QuoteOrganization {
            name: row.try_get("name")?,
            timezone: row.try_get("timezone")?,
            currency: row.try_get("currency")?,
            locale: row.try_get("locale")?,
            logo_file_id: optional_uuid(&row, "logo_file_id")?,
            primary_color: row.try_get("primary_color")?,
        })
    }

    pub(crate) async fn people(
        &self,
        ctx: &Ctx<'_>,
        quote: &Quote,
    ) -> Result<QuotePeople, AppError> {
        let owner: Option<(String, String)> = match &quote.owner_id {
            Some(id) => {
                sqlx::query_as("SELECT display_name,email FROM users WHERE id = ?")
                    .bind(id.as_str())
                    .fetch_optional(&self.pool)
                    .await?
            }
            None => None,
        };
        let query = ctx.query();
        use aphel_sdk::platform::application::query::RecordQueryPorts;
        let mut names = Vec::new();
        for (entity, id) in [
            (RecordType::CONTACT, quote.contact_id),
            (RecordType::COMPANY, quote.company_id),
        ] {
            let label = match id {
                Some(id) => aphel_sdk::records::visible_record_label(
                    &self.pool,
                    ctx.registry(),
                    query.definitions(),
                    ctx.scope,
                    &aphel_sdk::platform::application::shared::RecordRef::new(entity, id),
                )
                .await?
                .map(|label| label.title.unwrap_or_else(|| id.to_string())),
                None => None,
            };
            names.push(label);
        }
        Ok(QuotePeople {
            owner_name: owner.as_ref().map(|row| row.0.clone()),
            owner_email: owner.map(|row| row.1),
            contact_name: names.remove(0),
            company_name: names.remove(0),
        })
    }

    pub(crate) async fn settings(&self) -> Result<QuoteSettings, AppError> {
        let row = sqlx::query(
            "SELECT default_valid_days, default_terms, accept_stages, updated_by, updated_at \
             FROM quote_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(QuoteSettings::default());
        };
        let Json(stages): Json<Value> = row.try_get("accept_stages")?;
        let accept_stages: BTreeMap<Uuid, Uuid> = stages
            .as_object()
            .map(|map| {
                map.iter()
                    .filter_map(|(pipeline, stage)| {
                        Some((
                            Uuid::parse_str(pipeline).ok()?,
                            Uuid::parse_str(stage.as_str()?).ok()?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(QuoteSettings {
            default_valid_days: row.try_get::<Option<u16>, _>("default_valid_days")?,
            default_terms: row.try_get("default_terms")?,
            accept_stages,
            updated_by: optional_user(&row, "updated_by")?,
            updated_at: Some(instant(&row, "updated_at")?),
        })
    }

    pub(crate) async fn save_settings(
        &self,
        tx: &mut Tx,
        settings: &QuoteSettings,
    ) -> Result<(), AppError> {
        let stages: serde_json::Map<String, Value> = settings
            .accept_stages
            .iter()
            .map(|(pipeline, stage)| (pipeline.to_string(), Value::String(stage.to_string())))
            .collect();
        sqlx::query(
            "INSERT INTO quote_settings (id, default_valid_days, default_terms, \
             accept_stages, updated_by, updated_at) VALUES (1, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE default_valid_days = VALUES(default_valid_days), \
             default_terms = VALUES(default_terms), accept_stages = VALUES(accept_stages), \
             updated_by = VALUES(updated_by), updated_at = VALUES(updated_at)",
        )
        .bind(settings.default_valid_days)
        .bind(&settings.default_terms)
        .bind(Json(Value::Object(stages)))
        .bind(settings.updated_by.as_ref().map(UserId::as_str))
        .bind(optional_naive(settings.updated_at).unwrap_or_else(|| mapper::naive(Utc::now())))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn expiring(&self, today: NaiveDate) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM quotes WHERE status = 'sent' \
             AND deleted_at IS NULL AND valid_until IS NOT NULL AND valid_until < ? LIMIT 500",
        )
        .bind(today)
        .fetch_all(&self.pool)
        .await?;
        ids.iter()
            .map(|id| Uuid::parse_str(id).map_err(|_| mapper::invalid_column("quotes.id", id)))
            .collect()
    }
}

pub(crate) async fn lock_organization_settings(
    tx: &mut aphel_sdk::platform::database::transaction::Tx,
) -> Result<(), aphel_kernel::errors::AppError> {
    sqlx::query("SELECT id FROM organization_settings WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}
