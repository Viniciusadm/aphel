use crate::team::store::{push_credit_share, push_credited_filter};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::deals::{
    CreditedUsers, DealProbabilityReader, ProbabilityDeal, ProbabilityDealsRequest,
    ProbabilityGroup, ProbabilitySource,
};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::fields::permissions::{
    NOT_ACCESSIBLE_MESSAGE, not_accessible,
};
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::organization_admin::Listing;
use aphel_sdk::platform::application::query::{QueryPlan, RecordQuery};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::query::compiler::push_where;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::workflow::{PipelineStage, WorkflowCatalog};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

fn probability(source: ProbabilitySource) -> &'static str {
    match source {
        ProbabilitySource::Stage => "CAST(LEAST(fs.probability, 100) AS UNSIGNED)",
        ProbabilitySource::DealOrStage => {
            "CAST(LEAST(COALESCE(r.probability, fs.probability), 100) AS UNSIGNED)"
        }
    }
}

fn push_range(builder: &mut QueryBuilder<MySql>, request: &ProbabilityDealsRequest) {
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

fn deal_from_row(row: &MySqlRow) -> Result<ProbabilityDeal, AppError> {
    let id: String = row.try_get("id")?;
    let probability: u64 = row.try_get("probability")?;
    let owner_id: Option<String> = row.try_get("owner_id")?;
    let owner_name: Option<String> = row.try_get("owner_name")?;
    Ok(ProbabilityDeal {
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

struct ProbabilityStore {
    pool: Db,
    stages: Vec<PipelineStage>,
}
impl ProbabilityStore {
    async fn of(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        let catalog = ctx.services().get::<dyn WorkflowCatalog>().ok_or_else(|| {
            AppError::unavailable_code(
                "workflow_catalog_unavailable",
                "workflow metadata is not enabled",
            )
        })?;
        let pipelines = catalog.pipelines(ctx.pool()).await?;
        let mut stages = Vec::new();
        for ids in pipelines
            .into_iter()
            .map(|pipeline| pipeline.id)
            .collect::<Vec<_>>()
            .chunks(1000)
        {
            stages.extend(catalog.stages_for_pipelines(ctx.pool(), ids).await?);
        }
        Ok(Self {
            pool: ctx.pool().clone(),
            stages,
        })
    }
    fn push_from(&self, builder: &mut QueryBuilder<MySql>) {
        builder.push(" FROM deals r JOIN (");
        if self.stages.is_empty() {
            builder.push(
                "SELECT NULL AS id, NULL AS name, CAST(0 AS UNSIGNED) AS probability WHERE FALSE",
            );
        }
        for (index, stage) in self.stages.iter().enumerate() {
            if index > 0 {
                builder.push(" UNION ALL ");
            }
            builder
                .push("SELECT ")
                .push_bind(stage.id.to_string())
                .push(" AS id, ")
                .push_bind(stage.name.clone())
                .push(" AS name, ")
                .push_bind(stage.probability)
                .push(" AS probability");
        }
        builder.push(") fs ON fs.id = r.stage_id ");
    }
    pub(crate) async fn groups(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        source: ProbabilitySource,
        credited: Option<&CreditedUsers>,
    ) -> Result<Vec<ProbabilityGroup>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT DATE_FORMAT(r.expected_close_date, '%Y-%m-01') AS month, {} AS probability, \
             COUNT(*) AS deals, SUM(r.amount_base",
            probability(source)
        ));
        if let Some(credited) = credited {
            builder.push(" * ");
            push_credit_share(&mut builder, credited, "r");
        }
        builder.push(") AS amount");
        self.push_from(&mut builder);
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
                Ok(ProbabilityGroup {
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
        request: &ProbabilityDealsRequest,
    ) -> Result<Listing<ProbabilityDeal>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        self.push_from(&mut count);
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
             r.expected_close_date",
            probability(request.source)
        ));
        self.push_from(&mut builder);
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

pub(crate) struct ProbabilityReader;

async fn plan(ctx: &Ctx<'_>, query: RecordQuery) -> Result<(FieldSet, QueryPlan), AppError> {
    let fields = ctx
        .field_set(ctx.scope, aphel_sdk::deals::RECORD_TYPE)
        .await?;
    let inaccessible: Vec<_> = [
        "amount",
        aphel_sdk::money::BASE_AMOUNT_FIELD,
        "expected_close_date",
    ]
    .into_iter()
    .filter(|key| fields.is_hidden(key))
    .map(|key| FieldViolation::new(key, NOT_ACCESSIBLE_MESSAGE))
    .collect();
    if !inaccessible.is_empty() {
        return Err(not_accessible(inaccessible));
    }
    Ok((fields, crate::service::query_plan(ctx, query).await?))
}

#[async_trait::async_trait]
impl DealProbabilityReader for ProbabilityReader {
    async fn groups(
        &self,
        ctx: &Ctx<'_>,
        query: RecordQuery,
        source: ProbabilitySource,
        credited: Option<&CreditedUsers>,
    ) -> Result<Vec<ProbabilityGroup>, AppError> {
        let (fields, plan) = plan(ctx, query).await?;
        let source = if fields.is_hidden("probability") {
            ProbabilitySource::Stage
        } else {
            source
        };
        ProbabilityStore::of(ctx)
            .await?
            .groups(ctx.scope, &plan, source, credited)
            .await
    }
    async fn deals(
        &self,
        ctx: &Ctx<'_>,
        query: RecordQuery,
        request: &ProbabilityDealsRequest,
    ) -> Result<Listing<ProbabilityDeal>, AppError> {
        if !(1..=1000).contains(&request.window.limit) {
            return Err(AppError::Validation(
                "deal probability page limit must be between 1 and 1000".into(),
            ));
        }
        let (fields, plan) = plan(ctx, query).await?;
        let mut request = request.clone();
        if fields.is_hidden("probability") {
            request.source = ProbabilitySource::Stage;
        }
        let mut listing = ProbabilityStore::of(ctx)
            .await?
            .deals(ctx.scope, &plan, &request)
            .await?;
        for deal in &mut listing.items {
            if fields.is_hidden("title") {
                deal.title.clear();
            }
            if fields.is_hidden("currency") {
                deal.currency.clear();
            }
            if fields.is_hidden("owner_id") {
                deal.owner = None;
            }
            if fields.is_hidden("stage_id") {
                deal.stage_name.clear();
            }
        }
        Ok(listing)
    }
}
