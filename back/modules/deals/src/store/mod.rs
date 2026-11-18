pub(crate) mod deal_rows;
pub(crate) mod financials;
pub(crate) mod lifecycle;
pub(crate) mod merge_references;
pub(crate) mod probability;

use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{Execute, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    Deal, DealRecord, DealRevenue, DealStatus, NamedRef, OwnerRef, StageSummary, StageTotal,
    StageTransitionEntry,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::domain::users::UserStatus;
use aphel_sdk::platform::infrastructure::query::compiler::{
    Projection, build_query, push_record_scope, quoted,
};
use aphel_sdk::platform::infrastructure::repositories::mapper::{invalid_column, naive, utc};
use aphel_sdk::platform::records::constrain_query;
use aphel_sdk::workflow::StageKind;

const DEAL_SELECT: &str = "SELECT r.id, r.owner_id, r.pipeline_id, r.stage_id, r.title, r.amount, \
     r.currency, r.amount_base, r.exchange_rate, r.mrr, r.arr, r.tcv, r.contract_start, \
     r.contract_end, r.renewal_of_deal_id, r.probability, r.expected_close_date, r.status, r.closed_at, r.lost_reason_id, \
     r.lost_reason_note, r.source, r.contact_id, r.company_id, r.stage_entered_at, \
     r.last_activity_at, r.created_by, r.archived_at, r.created_at, r.updated_at, \
     '' AS stage_name, r.status AS stage_kind, 0 AS stage_probability, NULL AS stage_rotting_days, \
     NULL AS contact_name, NULL AS company_name, u.display_name AS owner_name, \
     NULL AS lost_reason_name FROM deals r \
     LEFT JOIN users u ON u.id = r.owner_id \
     WHERE r.deleted_at IS NULL";

pub fn parse_uuid(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|value| parse_uuid(&value, column))
        .transpose()
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|value| UserId::new(value).map_err(AppError::from))
        .transpose()
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row.try_get::<Option<NaiveDateTime>, _>(column)?.map(utc))
}

fn named(id: Option<Uuid>, name: Option<String>) -> Option<NamedRef> {
    id.zip(name).map(|(id, name)| NamedRef { id, name })
}

fn deal_from_row(row: &MySqlRow) -> Result<DealRecord, AppError> {
    let status: String = row.try_get("status")?;
    let stage_kind: String = row.try_get("stage_kind")?;
    let deal = Deal {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "deals.id")?,
        owner_id: optional_user(row, "owner_id")?,
        pipeline_id: parse_uuid(
            &row.try_get::<String, _>("pipeline_id")?,
            "deals.pipeline_id",
        )?,
        stage_id: parse_uuid(&row.try_get::<String, _>("stage_id")?, "deals.stage_id")?,
        title: row.try_get("title")?,
        amount: row.try_get::<Option<Decimal>, _>("amount")?,
        currency: row.try_get("currency")?,
        revenue: DealRevenue {
            amount_base: row.try_get::<Option<Decimal>, _>("amount_base")?,
            exchange_rate: row.try_get::<Option<Decimal>, _>("exchange_rate")?,
            mrr: row.try_get::<Option<Decimal>, _>("mrr")?,
            arr: row.try_get::<Option<Decimal>, _>("arr")?,
            tcv: row.try_get::<Option<Decimal>, _>("tcv")?,
            contract_start: row.try_get::<Option<NaiveDate>, _>("contract_start")?,
            contract_end: row.try_get::<Option<NaiveDate>, _>("contract_end")?,
            renewal_of_deal_id: optional_uuid(row, "renewal_of_deal_id")?,
        },
        probability: row.try_get::<Option<u8>, _>("probability")?,
        expected_close_date: row.try_get::<Option<NaiveDate>, _>("expected_close_date")?,
        status: DealStatus::parse(&status)
            .ok_or_else(|| invalid_column("deals.status", &status))?,
        closed_at: optional_instant(row, "closed_at")?,
        lost_reason_id: optional_uuid(row, "lost_reason_id")?,
        lost_reason_note: row.try_get("lost_reason_note")?,
        source: row.try_get("source")?,
        contact_id: optional_uuid(row, "contact_id")?,
        company_id: optional_uuid(row, "company_id")?,
        stage_entered_at: utc(row.try_get::<NaiveDateTime, _>("stage_entered_at")?),
        last_activity_at: optional_instant(row, "last_activity_at")?,
        created_by: optional_user(row, "created_by")?,
        archived_at: optional_instant(row, "archived_at")?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    };
    let stage = StageSummary {
        id: deal.stage_id,
        name: row.try_get("stage_name")?,
        kind: StageKind::parse(&stage_kind)
            .ok_or_else(|| invalid_column("workflow_stages.kind", &stage_kind))?,
        probability: u8::try_from(row.try_get::<i64, _>("stage_probability")?)
            .map_err(|_| invalid_column("stage_probability", "out of range"))?,
        rotting_days: row.try_get("stage_rotting_days")?,
    };
    let owner = deal
        .owner_id
        .clone()
        .zip(row.try_get::<Option<String>, _>("owner_name")?)
        .map(|(id, display_name)| OwnerRef { id, display_name });
    Ok(DealRecord {
        contact: named(deal.contact_id, row.try_get("contact_name")?),
        company: named(deal.company_id, row.try_get("company_name")?),
        lost_reason: named(deal.lost_reason_id, row.try_get("lost_reason_name")?),
        owner,
        stage,
        deal,
    })
}

#[derive(Clone)]
pub(crate) struct DealStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl DealStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    pub(crate) async fn find(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        deal_id: Uuid,
    ) -> Result<Option<DealRecord>, AppError> {
        let definition = self.registry.require(RecordType::DEAL.as_str())?;
        let mut builder = QueryBuilder::<MySql>::new(DEAL_SELECT);
        builder.push(" AND r.id = ").push_bind(deal_id.to_string());
        push_record_scope(&mut builder, scope, definition, visibility);
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref().map(deal_from_row).transpose()
    }

    pub(crate) async fn load(&self, ids: &[Uuid]) -> Result<Vec<DealRecord>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(DEAL_SELECT);
        builder.push(" AND r.id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(deal_from_row).collect()
    }

    pub(crate) async fn insert(&self, tx: &mut Tx, deal: &Deal) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deals (id, owner_id, pipeline_id, stage_id, title, amount, currency, \
             amount_base, exchange_rate, mrr, arr, tcv, contract_start, contract_end, renewal_of_deal_id, \
             probability, expected_close_date, status, closed_at, lost_reason_id, lost_reason_note, source, \
             contact_id, company_id, stage_entered_at, last_activity_at, created_by, archived_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(deal.id.to_string())
        .bind(deal.owner_id.as_ref().map(UserId::as_str))
        .bind(deal.pipeline_id.to_string())
        .bind(deal.stage_id.to_string())
        .bind(&deal.title)
        .bind(deal.amount)
        .bind(&deal.currency)
        .bind(deal.revenue.amount_base)
        .bind(deal.revenue.exchange_rate)
        .bind(deal.revenue.mrr)
        .bind(deal.revenue.arr)
        .bind(deal.revenue.tcv)
        .bind(deal.revenue.contract_start)
        .bind(deal.revenue.contract_end)
        .bind(deal.revenue.renewal_of_deal_id.map(|id| id.to_string()))
        .bind(deal.probability)
        .bind(deal.expected_close_date)
        .bind(deal.status.as_str())
        .bind(deal.closed_at.map(naive))
        .bind(deal.lost_reason_id.map(|id| id.to_string()))
        .bind(&deal.lost_reason_note)
        .bind(&deal.source)
        .bind(deal.contact_id.map(|id| id.to_string()))
        .bind(deal.company_id.map(|id| id.to_string()))
        .bind(naive(deal.stage_entered_at))
        .bind(deal.last_activity_at.map(naive))
        .bind(deal.created_by.as_ref().map(UserId::as_str))
        .bind(deal.archived_at.map(naive))
        .bind(naive(deal.created_at))
        .bind(naive(deal.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut Tx, deal: &Deal) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deals SET owner_id = ?, pipeline_id = ?, stage_id = ?, title = ?, amount = ?, currency = ?, \
             amount_base = ?, exchange_rate = ?, mrr = ?, arr = ?, tcv = ?, contract_start = ?, \
             contract_end = ?, renewal_of_deal_id = ?, probability = ?, expected_close_date = ?, status = ?, closed_at = ?, lost_reason_id = ?, \
             lost_reason_note = ?, source = ?, contact_id = ?, company_id = ?, stage_entered_at = ?, \
             archived_at = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(deal.owner_id.as_ref().map(UserId::as_str))
        .bind(deal.pipeline_id.to_string())
        .bind(deal.stage_id.to_string())
        .bind(&deal.title)
        .bind(deal.amount)
        .bind(&deal.currency)
        .bind(deal.revenue.amount_base)
        .bind(deal.revenue.exchange_rate)
        .bind(deal.revenue.mrr)
        .bind(deal.revenue.arr)
        .bind(deal.revenue.tcv)
        .bind(deal.revenue.contract_start)
        .bind(deal.revenue.contract_end)
        .bind(deal.revenue.renewal_of_deal_id.map(|id| id.to_string()))
        .bind(deal.probability)
        .bind(deal.expected_close_date)
        .bind(deal.status.as_str())
        .bind(deal.closed_at.map(naive))
        .bind(deal.lost_reason_id.map(|id| id.to_string()))
        .bind(&deal.lost_reason_note)
        .bind(&deal.source)
        .bind(deal.contact_id.map(|id| id.to_string()))
        .bind(deal.company_id.map(|id| id.to_string()))
        .bind(naive(deal.stage_entered_at))
        .bind(deal.archived_at.map(naive))
        .bind(naive(deal.updated_at))
        .bind(deal.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn soft_delete(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deals SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(naive(at))
        .bind(naive(at))
        .bind(deal_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
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

    pub(crate) async fn member_name(&self, user_id: &UserId) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = ? AND status = ?")
                .bind(user_id.as_str())
                .bind(UserStatus::Active.as_str())
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn stage_totals(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
    ) -> Result<Vec<StageTotal>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut count = build_query(scope, plan, Projection::Count)?;
        let prefix = format!("SELECT COUNT(*) FROM {} r", quoted(&plan.record_type.table));
        let sql = count.sql().as_str().to_owned();
        let Some(conditions) = sql.strip_prefix(&prefix) else {
            return Err(AppError::Internal(
                "unexpected shape of the record count query".to_owned(),
            ));
        };
        let grouped = format!(
            "SELECT r.stage_id, COUNT(*), SUM(r.amount_base) FROM {} r{conditions} GROUP BY r.stage_id",
            quoted(&plan.record_type.table)
        );
        let arguments = count
            .build()
            .take_arguments()
            .map_err(|error| AppError::Internal(error.to_string()))?
            .unwrap_or_default();
        let rows: Vec<(String, i64, Option<Decimal>)> =
            sqlx::query_as_with(sqlx::AssertSqlSafe(grouped), arguments)
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|(stage_id, count, total)| {
                Ok(StageTotal {
                    stage_id: parse_uuid(&stage_id, "deals.stage_id")?,
                    count: u64::try_from(count).unwrap_or_default(),
                    total_amount: total.unwrap_or_default(),
                })
            })
            .collect()
    }

    pub(crate) async fn touch_last_activity(
        &self,
        deal_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE deals SET last_activity_at = ?, updated_at = updated_at \
             WHERE id = ? AND (last_activity_at IS NULL OR last_activity_at < ?)",
        )
        .bind(naive(at))
        .bind(deal_id.to_string())
        .bind(naive(at))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn open_currencies(&self) -> Result<Vec<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT DISTINCT currency FROM deals \
             WHERE status = ? AND deleted_at IS NULL",
        )
        .bind(DealStatus::Open.as_str())
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn apply_open_rate(
        &self,
        tx: &mut Tx,
        currency: &str,
        rate: Option<Decimal>,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE deals SET exchange_rate = ?, amount_base = ROUND(amount * ?, 2), \
             updated_at = updated_at \
             WHERE currency = ? AND status = ? AND deleted_at IS NULL \
             AND NOT (exchange_rate <=> ? AND amount_base <=> ROUND(amount * ?, 2))",
        )
        .bind(rate)
        .bind(rate)
        .bind(currency)
        .bind(DealStatus::Open.as_str())
        .bind(rate)
        .bind(rate)
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn completed_ids(&self) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM deals WHERE status <> ? AND amount_base IS NOT NULL",
        )
        .bind(DealStatus::Open.as_str())
        .fetch_all(&self.pool)
        .await?;
        ids.iter()
            .map(|id| Uuid::parse_str(id).map_err(|error| AppError::Internal(error.to_string())))
            .collect()
    }

    pub(crate) async fn rebase_completed(&self, factor: Decimal) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE deals SET amount_base = ROUND(amount_base * ?, 2), \
             exchange_rate = ROUND(exchange_rate * ?, 10), updated_at = updated_at \
             WHERE status <> ? AND amount_base IS NOT NULL",
        )
        .bind(factor)
        .bind(factor)
        .bind(DealStatus::Open.as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn record_initial_stage(
        &self,
        deal_id: Uuid,
        stage_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deal_stage_history (id, deal_id, stage_id, from_stage_id, entered_at, event_id) \
             SELECT d.id, d.id, ?, NULL, d.created_at, NULL FROM deals d \
             WHERE d.id = ? \
             ON DUPLICATE KEY UPDATE id = deal_stage_history.id",
        )
        .bind(stage_id.to_string())
        .bind(deal_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn record_stage_transition(
        &self,
        entry: &StageTransitionEntry,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deal_stage_history (id, deal_id, stage_id, from_stage_id, entered_at, event_id) \
             SELECT ?, d.id, ?, ?, ?, ? FROM deals d \
             WHERE d.id = ? \
             ON DUPLICATE KEY UPDATE id = deal_stage_history.id",
        )
        .bind(entry.event_id.to_string())
        .bind(entry.stage_id.to_string())
        .bind(entry.from_stage_id.map(|id| id.to_string()))
        .bind(naive(entry.entered_at))
        .bind(entry.event_id.to_string())
        .bind(entry.deal_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
