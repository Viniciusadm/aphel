use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::reports::domain::{
    CycleTime, FunnelBucket, InstantRange, PerformanceDimension, ReportPipeline, ReportStage,
    StageDuration, StageLoss,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::query::compiler::{push_where, quoted};
use aphel_sdk::platform::infrastructure::repositories::mapper::{invalid_column, naive};
use aphel_sdk::platform::records::constrain_query;
use aphel_sdk::workflow::{PipelineStage, WorkflowReader};

type StageDurationRow = (String, i64, Option<Decimal>, i64, Option<Decimal>);

#[derive(Clone)]
pub(crate) struct BuiltinReportStore {
    pool: MySqlPool,
    workflow: WorkflowReader,
}

impl BuiltinReportStore {
    pub(crate) fn of(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        Ok(Self {
            pool: ctx.pool().clone(),
            workflow: WorkflowReader::of(ctx)?,
        })
    }
}

fn uuid_column(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

fn count_of(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn records(plan: &QueryPlan) -> String {
    quoted(&plan.record_type.table)
}

fn push_stage_catalog(builder: &mut QueryBuilder<MySql>, stages: &[PipelineStage]) {
    builder.push("(");
    if stages.is_empty() {
        builder.push(
            "SELECT NULL AS id, NULL AS pipeline_id, NULL AS position, NULL AS kind WHERE FALSE",
        );
    }
    for (index, stage) in stages.iter().enumerate() {
        if index > 0 {
            builder.push(" UNION ALL ");
        }
        builder
            .push("SELECT ")
            .push_bind(stage.id.to_string())
            .push(" AS id, ")
            .push_bind(stage.pipeline_id.to_string())
            .push(" AS pipeline_id, ")
            .push_bind(stage.position)
            .push(" AS position, ")
            .push_bind(stage.kind.as_str().to_owned())
            .push(" AS kind");
    }
    builder.push(")");
}

fn push_workflow_stages(builder: &mut QueryBuilder<MySql>, alias: &str, pipeline_id: Uuid) {
    builder
        .push(format!(" AND {alias}.pipeline_id = "))
        .push_bind(pipeline_id.to_string())
        .push(format!(" AND {alias}.kind = 'open'"));
}

impl BuiltinReportStore {
    pub(crate) async fn currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| "BRL".to_owned()))
    }

    pub(crate) async fn pipeline(
        &self,
        pipeline_id: Option<Uuid>,
    ) -> Result<Option<ReportPipeline>, AppError> {
        let pipeline = match pipeline_id {
            Some(id) => self.workflow.find_pipeline(id).await?,
            None => self.workflow.default_pipeline().await?,
        };
        let Some(pipeline) = pipeline else {
            return Ok(None);
        };
        let stages = self
            .workflow
            .list_stages(pipeline.id)
            .await?
            .into_iter()
            .filter(|stage| stage.is_active)
            .map(|stage| ReportStage {
                id: stage.id,
                name: stage.name,
                kind: stage.kind,
                position: stage.position,
                probability: stage.probability,
            })
            .collect();
        Ok(Some(ReportPipeline {
            id: pipeline.id,
            name: pipeline.name,
            stages,
        }))
    }

    pub(crate) async fn funnel(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        pipeline_id: Uuid,
    ) -> Result<Vec<FunnelBucket>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT CAST(GREATEST(COALESCE(f.furthest, cs.position), COALESCE(cs.position, f.furthest)) AS SIGNED) AS furthest_position, \
             r.status, COUNT(*), SUM(r.amount_base) FROM {} r \
             LEFT JOIN (SELECT h.deal_id, MAX(ps.position) AS furthest FROM deal_stage_history h \
             JOIN ",
            records(plan)
        ));
        let stages = self.workflow.list_stages(pipeline_id).await?;
        push_stage_catalog(&mut builder, &stages);
        builder.push(" ps ON ps.id = h.stage_id WHERE 1 = 1");
        push_workflow_stages(&mut builder, "ps", pipeline_id);
        builder.push(
            " GROUP BY h.deal_id) f ON f.deal_id = r.id \
             LEFT JOIN ",
        );
        push_stage_catalog(&mut builder, &stages);
        builder.push(" cs ON cs.id = r.stage_id");
        push_workflow_stages(&mut builder, "cs", pipeline_id);
        push_where(&mut builder, scope, plan)?;
        builder.push(" GROUP BY furthest_position, r.status");
        let rows: Vec<(Option<i64>, String, i64, Option<Decimal>)> =
            builder.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(furthest, status, count, amount)| FunnelBucket {
                furthest_position: furthest,
                status,
                count: count_of(count),
                amount: amount.unwrap_or_default(),
            })
            .collect())
    }

    pub(crate) async fn losses_by_stage(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        pipeline_id: Uuid,
    ) -> Result<Vec<StageLoss>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT l.stage_id, COUNT(*), SUM(r.amount_base) FROM {} r \
             LEFT JOIN (SELECT ranked.deal_id, ranked.stage_id FROM (\
             SELECT h.deal_id, h.stage_id, ROW_NUMBER() OVER (PARTITION BY h.deal_id ORDER BY h.entered_at DESC, h.id DESC) AS entry_rank \
             FROM deal_stage_history h JOIN ",
            records(plan)
        ));
        let stages = self.workflow.list_stages(pipeline_id).await?;
        push_stage_catalog(&mut builder, &stages);
        builder.push(" ps ON ps.id = h.stage_id WHERE 1 = 1");
        push_workflow_stages(&mut builder, "ps", pipeline_id);
        builder.push(") ranked WHERE ranked.entry_rank = 1) l ON l.deal_id = r.id");
        push_where(&mut builder, scope, plan)?;
        builder.push(" GROUP BY l.stage_id");
        let rows: Vec<(Option<String>, i64, Option<Decimal>)> =
            builder.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(stage_id, count, amount)| {
                Ok(StageLoss {
                    stage_id: stage_id
                        .map(|id| uuid_column(&id, "deal_stage_history.stage_id"))
                        .transpose()?,
                    count: count_of(count),
                    amount: amount.unwrap_or_default(),
                })
            })
            .collect()
    }

    pub(crate) async fn stage_durations(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        pipeline_id: Uuid,
        entered: InstantRange,
        now: DateTime<Utc>,
    ) -> Result<Vec<StageDuration>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT t.stage_id, \
             CAST(SUM(t.exited_at IS NOT NULL) AS SIGNED), \
             AVG(CASE WHEN t.exited_at IS NOT NULL THEN TIMESTAMPDIFF(SECOND, t.entered_at, t.exited_at) END), \
             CAST(SUM(t.exited_at IS NULL) AS SIGNED), \
             AVG(CASE WHEN t.exited_at IS NULL THEN TIMESTAMPDIFF(SECOND, t.entered_at, ",
        );
        builder.push_bind(naive(now)).push(format!(
            ") END) FROM {} r \
             JOIN (SELECT h.deal_id, h.stage_id, h.entered_at, \
             LEAD(h.entered_at) OVER (PARTITION BY h.deal_id ORDER BY h.entered_at, h.id) AS exited_at \
             FROM deal_stage_history h WHERE h.deal_id IN (SELECT d.id FROM deals d WHERE d.pipeline_id = ",
            records(plan)
        ));
        builder
            .push_bind(pipeline_id.to_string())
            .push(")) t ON t.deal_id = r.id JOIN ");
        let stages = self.workflow.list_stages(pipeline_id).await?;
        push_stage_catalog(&mut builder, &stages);
        builder.push(" ps ON ps.id = t.stage_id");
        push_workflow_stages(&mut builder, "ps", pipeline_id);
        push_where(&mut builder, scope, plan)?;
        builder
            .push(" AND t.entered_at >= ")
            .push_bind(naive(entered.starts_at))
            .push(" AND t.entered_at < ")
            .push_bind(naive(entered.ends_at))
            .push(" GROUP BY t.stage_id");
        let rows: Vec<StageDurationRow> = builder.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(stage_id, completed, average, open, open_average)| {
                Ok(StageDuration {
                    stage_id: uuid_column(&stage_id, "deal_stage_history.stage_id")?,
                    completed: count_of(completed),
                    average_seconds: average,
                    open: count_of(open),
                    open_average_seconds: open_average,
                })
            })
            .collect()
    }

    pub(crate) async fn cycle_times(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        dimension: Option<PerformanceDimension>,
    ) -> Result<Vec<CycleTime>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let key = match dimension {
            None => "NULL",
            Some(PerformanceDimension::Owner) => "r.owner_id",
            Some(PerformanceDimension::Source) => "NULLIF(r.source, '')",
            Some(PerformanceDimension::Pipeline) => "r.pipeline_id",
            Some(PerformanceDimension::Team) => "ot.team_id",
        };
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT {key} AS group_key, COUNT(DISTINCT r.id), \
             AVG(TIMESTAMPDIFF(SECOND, r.created_at, r.closed_at)) FROM {} r",
            records(plan)
        ));
        if dimension == Some(PerformanceDimension::Team) {
            builder.push(
                " LEFT JOIN (SELECT tm.user_id, tm.team_id FROM team_memberships tm) ot \
                     ON ot.user_id = r.owner_id",
            );
        }
        push_where(&mut builder, scope, plan)?;
        builder.push(" AND r.closed_at IS NOT NULL");
        if dimension.is_some() {
            builder.push(" GROUP BY group_key");
        }
        let rows: Vec<(Option<String>, i64, Option<Decimal>)> =
            builder.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(key, count, average)| CycleTime {
                key,
                count: count_of(count),
                average_seconds: average,
            })
            .collect())
    }
}
