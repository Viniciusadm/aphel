use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    ForecastDeal, ForecastDealsRequest, ForecastGroupRow, ForecastMethod, ForecastSettings,
    ProbabilitySource,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::QueryPlan;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::push_where;
use aphel_kernel::infrastructure::repositories::mapper;
use deals::team::domain::CreditedUsers;
use deals::team::store::{push_credit_share, push_credited_filter};

const DEALS_FROM: &str = " FROM deals r JOIN workflow_stages fs ON fs.id = r.stage_id ";

fn probability(source: ProbabilitySource) -> &'static str {
    match source {
        ProbabilitySource::Stage => "CAST(LEAST(fs.probability, 100) AS UNSIGNED)",
        ProbabilitySource::DealOrStage => {
            "CAST(LEAST(COALESCE(r.probability, fs.probability), 100) AS UNSIGNED)"
        }
    }
}

fn push_range(builder: &mut QueryBuilder<MySql>, request: &ForecastDealsRequest) {
    if let Some((min, max)) = request.probability_range {
        let expression = probability(request.source);
        builder
            .push(format!(" AND {expression} >= "))
            .push_bind(min)
            .push(format!(" AND {expression} <= "))
            .push_bind(max);
    }
}

fn percent(value: u64) -> u8 {
    u8::try_from(value.min(100)).unwrap_or(100)
}

fn settings_from_row(row: &MySqlRow) -> Result<ForecastSettings, AppError> {
    let method: String = row.try_get("method")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(ForecastSettings {
        method: ForecastMethod::parse(&method)
            .ok_or_else(|| mapper::invalid_column("method", &method))?,
        categories_enabled: row.try_get("categories_enabled")?,
        best_case_min_probability: row.try_get("best_case_min_probability")?,
        commit_min_probability: row.try_get("commit_min_probability")?,
        horizon_months: row.try_get("horizon_months")?,
        updated_at: Some(mapper::utc(updated_at)),
    })
}

fn deal_from_row(row: &MySqlRow) -> Result<ForecastDeal, AppError> {
    let id: String = row.try_get("id")?;
    let probability: u64 = row.try_get("probability")?;
    let owner_id: Option<String> = row.try_get("owner_id")?;
    let owner_name: Option<String> = row.try_get("owner_name")?;
    Ok(ForecastDeal {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        title: row.try_get("title")?,
        amount: row.try_get("amount")?,
        currency: row.try_get("currency")?,
        probability: percent(probability),
        stage_name: row.try_get("stage_name")?,
        owner: owner_id
            .map(|id| {
                Ok::<_, AppError>(UserReference {
                    id: mapper::typed_id(id, UserId::new)?,
                    display_name: owner_name.unwrap_or_default(),
                })
            })
            .transpose()?,
        expected_close_date: row.try_get("expected_close_date")?,
    })
}

#[derive(Clone)]
pub(crate) struct ForecastStore {
    pool: MySqlPool,
}

impl ForecastStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| "BRL".to_owned()))
    }

    pub(crate) async fn settings(&self) -> Result<Option<ForecastSettings>, AppError> {
        sqlx::query(
            "SELECT method, categories_enabled, best_case_min_probability, commit_min_probability, \
             horizon_months, updated_at FROM forecast_settings",
        )
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(settings_from_row)
        .transpose()
    }

    pub(crate) async fn save_settings(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        settings: &ForecastSettings,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO forecast_settings (id, method, categories_enabled, \
             best_case_min_probability, commit_min_probability, horizon_months, updated_by, \
             created_at, updated_at) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE method = VALUES(method), \
             categories_enabled = VALUES(categories_enabled), \
             best_case_min_probability = VALUES(best_case_min_probability), \
             commit_min_probability = VALUES(commit_min_probability), \
             horizon_months = VALUES(horizon_months), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(settings.method.as_str())
        .bind(settings.categories_enabled)
        .bind(settings.best_case_min_probability)
        .bind(settings.commit_min_probability)
        .bind(settings.horizon_months)
        .bind(scope.user_id().as_str())
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn groups(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        source: ProbabilitySource,
        credited: Option<&CreditedUsers>,
    ) -> Result<Vec<ForecastGroupRow>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT DATE_FORMAT(r.expected_close_date, '%Y-%m-01') AS month, {} AS probability, \
             COUNT(*) AS deals, SUM(r.amount_base",
            probability(source)
        ));
        if let Some(credited) = credited {
            builder.push(" * ");
            push_credit_share(&mut builder, credited, "r");
        }
        builder.push(format!(") AS amount{DEALS_FROM}"));
        push_where(&mut builder, scope, plan)?;
        if let Some(credited) = credited {
            push_credited_filter(&mut builder, credited, "r");
        }
        builder.push(" GROUP BY month, probability ORDER BY month, probability");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let month: String = row.try_get("month")?;
                let probability: u64 = row.try_get("probability")?;
                let deals: i64 = row.try_get("deals")?;
                let amount: Option<Decimal> = row.try_get("amount")?;
                Ok(ForecastGroupRow {
                    month: NaiveDate::parse_from_str(&month, "%Y-%m-%d")
                        .map_err(|_| mapper::invalid_column("expected_close_date", &month))?,
                    probability: percent(probability),
                    count: u64::try_from(deals).unwrap_or_default(),
                    amount: amount.unwrap_or_default(),
                })
            })
            .collect()
    }

    pub(crate) async fn deals(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        request: &ForecastDealsRequest,
    ) -> Result<Listing<ForecastDeal>, AppError> {
        let mut count = QueryBuilder::<MySql>::new(format!("SELECT COUNT(*){DEALS_FROM}"));
        push_where(&mut count, scope, plan)?;
        push_range(&mut count, request);
        if let Some(credited) = &request.credited {
            push_credited_filter(&mut count, credited, "r");
        }
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id, r.title, r.amount, r.currency, {} AS probability, fs.name AS stage_name, \
             r.owner_id, (SELECT fu.display_name FROM users fu WHERE fu.id = r.owner_id) AS owner_name, \
             r.expected_close_date{DEALS_FROM}",
            probability(request.source)
        ));
        push_where(&mut builder, scope, plan)?;
        push_range(&mut builder, request);
        if let Some(credited) = &request.credited {
            push_credited_filter(&mut builder, credited, "r");
        }
        builder
            .push(" ORDER BY r.amount_base IS NULL, r.amount_base DESC, r.expected_close_date, r.id LIMIT ")
            .push_bind(request.window.limit)
            .push(" OFFSET ")
            .push_bind(request.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(deal_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }
}
