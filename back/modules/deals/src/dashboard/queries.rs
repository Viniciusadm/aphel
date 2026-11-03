use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::QueryPlan;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::{Projection, build_query, quoted};
use aphel_kernel::infrastructure::repositories::mapper::invalid_column;
use aphel_kernel::infrastructure::repositories::mapper::utc;
use aphel_kernel::records::constrain_query;
use aphel_sdk::workflow::StageKind;
use aphel_sdk::workflow::WorkflowReader;
use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlArguments;
use sqlx::mysql::MySqlRow;
use sqlx::{Execute, MySql, MySqlPool, QueryBuilder, Row};
use std::collections::HashMap;
use uuid::Uuid;

fn uuid_column(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}
pub fn aggregate_query(
    scope: &AccessScope,
    plan: &QueryPlan,
    select: &str,
    suffix: &str,
) -> Result<(String, MySqlArguments), AppError> {
    let mut count = build_query(scope, plan, Projection::Count)?;
    let table = quoted(&plan.record_type.table);
    let prefix = format!("SELECT COUNT(*) FROM {table} r");
    let sql = count.sql().as_str().to_owned();
    let Some(conditions) = sql.strip_prefix(&prefix) else {
        return Err(AppError::Internal(
            "unexpected shape of the record count query".to_owned(),
        ));
    };
    let statement = format!("SELECT {select} FROM {table} r{conditions}{suffix}");
    let arguments = count
        .build()
        .take_arguments()
        .map_err(|error| AppError::Internal(error.to_string()))?
        .unwrap_or_default();
    Ok((statement, arguments))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DealTotals {
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
    pub(crate) forecast: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageAggregate {
    pub(crate) stage_id: Uuid,
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardStage {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) kind: StageKind,
    pub(crate) position: i32,
    pub(crate) probability: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardPipeline {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) stages: Vec<DashboardStage>,
}

type ForecastRow = (String, String, Option<u8>, i64, Option<Decimal>);

pub(crate) async fn deal_totals(
    pool: &MySqlPool,
    services: &aphel_kernel::modules::Services,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<DealTotals, AppError> {
    let constrained = constrain_query(pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let (statement, arguments) = aggregate_query(
        scope,
        plan,
        "r.pipeline_id, r.stage_id, r.probability, COUNT(*), SUM(r.amount_base)",
        " GROUP BY r.pipeline_id, r.stage_id, r.probability",
    )?;
    let rows: Vec<ForecastRow> = sqlx::query_as_with(sqlx::AssertSqlSafe(statement), arguments)
        .fetch_all(pool)
        .await?;
    let pipelines: Vec<_> = rows
        .iter()
        .map(|(id, ..)| uuid_column(id, "deals.pipeline_id"))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?
        .into_iter()
        .collect();
    let workflow = WorkflowReader::new(pool.clone(), services)?;
    let stages: HashMap<_, _> = workflow
        .stages_for_pipelines(&pipelines)
        .await?
        .into_iter()
        .map(|stage| (stage.id, stage.probability))
        .collect();
    let mut totals = DealTotals::default();
    for (_, stage, probability, count, amount) in rows {
        let amount = amount.unwrap_or_default();
        let stage = uuid_column(&stage, "deals.stage_id")?;
        let probability = probability.or_else(|| stages.get(&stage).copied());
        totals.count += u64::try_from(count).unwrap_or_default();
        totals.amount += amount;
        if let Some(probability) = probability {
            totals.forecast += amount * Decimal::from(probability) / Decimal::from(100);
        }
    }
    Ok(totals)
}

pub(crate) async fn stage_aggregates(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<StageAggregate>, AppError> {
    let constrained = constrain_query(pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let (statement, arguments) = aggregate_query(
        scope,
        plan,
        "r.stage_id, COUNT(*), SUM(r.amount_base)",
        " GROUP BY r.stage_id",
    )?;
    let rows: Vec<(String, i64, Option<Decimal>)> =
        sqlx::query_as_with(sqlx::AssertSqlSafe(statement), arguments)
            .fetch_all(pool)
            .await?;
    rows.into_iter()
        .map(|(stage_id, count, amount)| {
            Ok(StageAggregate {
                stage_id: uuid_column(&stage_id, "deals.stage_id")?,
                count: u64::try_from(count).unwrap_or_default(),
                amount: amount.unwrap_or_default(),
            })
        })
        .collect()
}

pub(crate) async fn pipeline(
    pool: &MySqlPool,
    services: &aphel_kernel::modules::Services,
    scope: &AccessScope,
    pipeline_id: Option<Uuid>,
) -> Result<Option<DashboardPipeline>, AppError> {
    let store = WorkflowReader::new(pool.clone(), services)?;
    let pipeline = match pipeline_id {
        Some(id) => store
            .find_pipeline(id)
            .await?
            .filter(|pipeline| aphel_sdk::workflow::pipeline_visible(scope, pipeline)),
        None => store
            .list_pipelines()
            .await?
            .into_iter()
            .filter(|pipeline| {
                pipeline.is_active && aphel_sdk::workflow::pipeline_visible(scope, pipeline)
            })
            .min_by_key(|pipeline| (!pipeline.is_default, pipeline.position, pipeline.id)),
    };
    let Some(pipeline) = pipeline else {
        return Ok(None);
    };
    let stages = store
        .list_stages(pipeline.id)
        .await?
        .into_iter()
        .filter(|stage| stage.is_active)
        .map(|stage| DashboardStage {
            id: stage.id,
            name: stage.name,
            kind: stage.kind,
            position: stage.position,
            probability: stage.probability,
        })
        .collect();
    Ok(Some(DashboardPipeline {
        id: pipeline.id,
        name: pipeline.name,
        stages,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaleDeal {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) amount: Option<Decimal>,
    pub(crate) currency: String,
    pub(crate) stage_name: String,
    pub(crate) owner: Option<UserReference>,
    pub(crate) last_touched_at: DateTime<Utc>,
}

pub(crate) async fn stale_deals(
    pool: &MySqlPool,
    services: &aphel_kernel::modules::Services,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<StaleDeal>, AppError> {
    let ids = record_ids(pool, scope, plan).await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT d.id, d.title, d.amount, d.currency, d.stage_id, d.pipeline_id, d.owner_id, \
         u.display_name AS owner_name, \
         GREATEST(d.updated_at, COALESCE(d.last_activity_at, d.updated_at)) AS last_touched_at \
         FROM deals d \
         LEFT JOIN users u ON u.id = d.owner_id WHERE d.id",
    );
    push_ids(&mut builder, &ids);
    let rows = builder.build().fetch_all(pool).await?;
    let pipelines: Vec<_> = rows
        .iter()
        .map(|row| {
            uuid_column(
                &row.try_get::<String, _>("pipeline_id")?,
                "deals.pipeline_id",
            )
        })
        .collect::<Result<std::collections::BTreeSet<_>, AppError>>()?
        .into_iter()
        .collect();
    let workflow = WorkflowReader::new(pool.clone(), services)?;
    let stages: HashMap<_, _> = workflow
        .stages_for_pipelines(&pipelines)
        .await?
        .into_iter()
        .map(|stage| (stage.id, stage.name))
        .collect();
    let mut items = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let id = uuid_column(&id, "deals.id")?;
        let touched: NaiveDateTime = row.try_get("last_touched_at")?;
        items.insert(
            id,
            StaleDeal {
                id,
                title: row.try_get("title")?,
                amount: row.try_get("amount")?,
                currency: row.try_get("currency")?,
                stage_name: {
                    let stage =
                        uuid_column(&row.try_get::<String, _>("stage_id")?, "deals.stage_id")?;
                    stages
                        .get(&stage)
                        .cloned()
                        .unwrap_or_else(|| stage.to_string())
                },
                owner: user_column(row, "owner_id", "owner_name")?,
                last_touched_at: utc(touched),
            },
        );
    }
    Ok(in_order(&ids, items))
}

pub fn user_column(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    id.map(|id| {
        Ok(UserReference {
            id: UserId::new(id.clone()).map_err(|_| invalid_column(id_column, &id))?,
            display_name: name.unwrap_or_default(),
        })
    })
    .transpose()
}

pub fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

pub fn in_order<T>(ids: &[Uuid], mut items: HashMap<Uuid, T>) -> Vec<T> {
    ids.iter().filter_map(|id| items.remove(id)).collect()
}

pub async fn record_ids(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<Uuid>, AppError> {
    let constrained = constrain_query(pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let mut builder = build_query(scope, plan, Projection::Ids)?;
    let rows: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    rows.iter().map(|id| uuid_column(id, "id")).collect()
}
