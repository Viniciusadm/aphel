use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::http::money;
use crate::reports::domain::{
    ConversionQuery, ConversionReport, ConversionSnapshot, ConversionSummary, LostReasonTotal,
    PerformanceDimension, PerformanceMetrics, PerformanceQuery, PerformanceReport, PerformanceRow,
    ReportPipeline, ReportStage, StageFunnel,
};
use aphel_sdk::dashboard::PeriodQuery;
use aphel_sdk::dashboard::dto::{DashboardPeriodResponse, PeriodPresetSchema};
use aphel_sdk::workflow::StageKind;

fn optional_money(value: Option<Decimal>) -> Option<String> {
    value.map(money)
}

fn seconds(value: Option<Decimal>) -> Option<i64> {
    value.and_then(|seconds| seconds.round().to_i64())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = PerformanceDimension)]
pub(crate) enum PerformanceDimensionSchema {
    Owner,
    Team,
    Source,
    Pipeline,
}

impl From<PerformanceDimension> for PerformanceDimensionSchema {
    fn from(dimension: PerformanceDimension) -> Self {
        match dimension {
            PerformanceDimension::Owner => Self::Owner,
            PerformanceDimension::Team => Self::Team,
            PerformanceDimension::Source => Self::Source,
            PerformanceDimension::Pipeline => Self::Pipeline,
        }
    }
}

impl From<PerformanceDimensionSchema> for PerformanceDimension {
    fn from(dimension: PerformanceDimensionSchema) -> Self {
        match dimension {
            PerformanceDimensionSchema::Owner => Self::Owner,
            PerformanceDimensionSchema::Team => Self::Team,
            PerformanceDimensionSchema::Source => Self::Source,
            PerformanceDimensionSchema::Pipeline => Self::Pipeline,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ReportStageKind)]
pub(crate) enum StageKindSchema {
    Open,
    Won,
    Lost,
}

impl From<StageKind> for StageKindSchema {
    fn from(kind: StageKind) -> Self {
        match kind {
            StageKind::Open => Self::Open,
            StageKind::Won => Self::Won,
            StageKind::Lost => Self::Lost,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ConversionParams {
    #[param(example = "0190a1b2-0000-7000-8000-000000000010")]
    pub(crate) pipeline_id: Option<String>,
    #[param(example = "quarter")]
    pub(crate) period: Option<PeriodPresetSchema>,
    #[param(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-03-31")]
    pub(crate) to: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl From<ConversionParams> for ConversionQuery {
    fn from(params: ConversionParams) -> Self {
        Self {
            pipeline_id: params.pipeline_id,
            period: PeriodQuery {
                period: params.period.map(|preset| preset.as_str().to_owned()),
                from: params.from,
                to: params.to,
            },
            compare_to_previous: params.compare_to_previous.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PerformanceParams {
    #[param(example = "owner")]
    pub(crate) dimension: Option<PerformanceDimensionSchema>,
    #[param(example = "0190a1b2-0000-7000-8000-000000000010")]
    pub(crate) pipeline_id: Option<String>,
    #[param(example = "month")]
    pub(crate) period: Option<PeriodPresetSchema>,
    #[param(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-01-31")]
    pub(crate) to: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl From<PerformanceParams> for PerformanceQuery {
    fn from(params: PerformanceParams) -> Self {
        Self {
            dimension: params.dimension.map(Into::into),
            pipeline_id: params.pipeline_id,
            period: PeriodQuery {
                period: params.period.map(|preset| preset.as_str().to_owned()),
                from: params.from,
                to: params.to,
            },
            compare_to_previous: params.compare_to_previous.unwrap_or(false),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReportStageResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: StageKindSchema,
    pub(crate) position: i32,
    pub(crate) probability: u8,
}

impl From<ReportStage> for ReportStageResponse {
    fn from(stage: ReportStage) -> Self {
        Self {
            id: stage.id.to_string(),
            name: stage.name,
            kind: stage.kind.into(),
            position: stage.position,
            probability: stage.probability,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReportPipelineResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) stages: Vec<ReportStageResponse>,
}

impl From<ReportPipeline> for ReportPipelineResponse {
    fn from(pipeline: ReportPipeline) -> Self {
        Self {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            stages: pipeline.stages.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ConversionSummaryResponse {
    pub(crate) created: u64,
    #[schema(example = "150000.00")]
    pub(crate) created_amount: Option<String>,
    pub(crate) open: u64,
    pub(crate) won: u64,
    #[schema(example = "42000.00")]
    pub(crate) won_amount: Option<String>,
    pub(crate) lost: u64,
    pub(crate) lost_amount: Option<String>,
    #[schema(example = 0.25)]
    pub(crate) conversion_rate: Option<f64>,
    #[schema(example = 0.4)]
    pub(crate) win_rate: Option<f64>,
    #[schema(example = 1_296_000)]
    pub(crate) average_cycle_seconds: Option<i64>,
}

impl From<ConversionSummary> for ConversionSummaryResponse {
    fn from(summary: ConversionSummary) -> Self {
        Self {
            created: summary.created,
            created_amount: optional_money(summary.created_amount),
            open: summary.open,
            won: summary.won,
            won_amount: optional_money(summary.won_amount),
            lost: summary.lost,
            lost_amount: optional_money(summary.lost_amount),
            conversion_rate: summary.conversion_rate,
            win_rate: summary.win_rate,
            average_cycle_seconds: seconds(summary.average_cycle_seconds),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct StageFunnelResponse {
    pub(crate) stage_id: String,
    pub(crate) reached: u64,
    pub(crate) reached_amount: Option<String>,
    pub(crate) advanced: u64,
    #[schema(example = 0.6)]
    pub(crate) passage_rate: Option<f64>,
    pub(crate) lost: u64,
    pub(crate) lost_amount: Option<String>,
    pub(crate) completed_visits: u64,
    #[schema(example = 259_200)]
    pub(crate) average_seconds: Option<i64>,
    pub(crate) open_visits: u64,
    pub(crate) open_average_seconds: Option<i64>,
}

impl From<StageFunnel> for StageFunnelResponse {
    fn from(stage: StageFunnel) -> Self {
        Self {
            stage_id: stage.stage_id.to_string(),
            reached: stage.reached,
            reached_amount: optional_money(stage.reached_amount),
            advanced: stage.advanced,
            passage_rate: stage.passage_rate,
            lost: stage.lost,
            lost_amount: optional_money(stage.lost_amount),
            completed_visits: stage.completed_visits,
            average_seconds: seconds(stage.average_seconds),
            open_visits: stage.open_visits,
            open_average_seconds: seconds(stage.open_average_seconds),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LostReasonTotalResponse {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) count: u64,
    pub(crate) amount: Option<String>,
}

impl From<LostReasonTotal> for LostReasonTotalResponse {
    fn from(reason: LostReasonTotal) -> Self {
        Self {
            id: reason.id,
            name: reason.name,
            count: reason.count,
            amount: optional_money(reason.amount),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ConversionSnapshotResponse {
    pub(crate) period: DashboardPeriodResponse,
    pub(crate) summary: ConversionSummaryResponse,
    pub(crate) stages: Vec<StageFunnelResponse>,
    pub(crate) unattributed_lost: u64,
    pub(crate) unattributed_lost_amount: Option<String>,
    pub(crate) lost_reasons: Vec<LostReasonTotalResponse>,
}

impl From<ConversionSnapshot> for ConversionSnapshotResponse {
    fn from(snapshot: ConversionSnapshot) -> Self {
        Self {
            period: snapshot.period.into(),
            summary: snapshot.summary.into(),
            stages: snapshot.stages.into_iter().map(Into::into).collect(),
            unattributed_lost: snapshot.unattributed_lost,
            unattributed_lost_amount: optional_money(snapshot.unattributed_lost_amount),
            lost_reasons: snapshot.lost_reasons.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ConversionReportResponse {
    pub(crate) pipeline: Option<ReportPipelineResponse>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) amounts_visible: bool,
    pub(crate) current: Option<ConversionSnapshotResponse>,
    pub(crate) previous: Option<ConversionSnapshotResponse>,
}

impl From<ConversionReport> for ConversionReportResponse {
    fn from(report: ConversionReport) -> Self {
        Self {
            pipeline: report.pipeline.map(Into::into),
            currency: report.currency,
            timezone: report.timezone,
            amounts_visible: report.amounts_visible,
            current: report.current.map(Into::into),
            previous: report.previous.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PerformanceMetricsResponse {
    pub(crate) created: u64,
    pub(crate) won: u64,
    #[schema(example = "42000.00")]
    pub(crate) won_amount: Option<String>,
    pub(crate) lost: u64,
    #[schema(example = 0.4)]
    pub(crate) conversion_rate: Option<f64>,
    #[schema(example = 1_296_000)]
    pub(crate) average_cycle_seconds: Option<i64>,
    pub(crate) activities_done: Option<u64>,
}

impl From<PerformanceMetrics> for PerformanceMetricsResponse {
    fn from(metrics: PerformanceMetrics) -> Self {
        Self {
            created: metrics.created,
            won: metrics.won,
            won_amount: optional_money(metrics.won_amount),
            lost: metrics.lost,
            conversion_rate: metrics.conversion_rate,
            average_cycle_seconds: seconds(metrics.average_cycle_seconds),
            activities_done: metrics.activities_done,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PerformanceRowResponse {
    pub(crate) key: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) current: PerformanceMetricsResponse,
    pub(crate) previous: Option<PerformanceMetricsResponse>,
}

impl From<PerformanceRow> for PerformanceRowResponse {
    fn from(row: PerformanceRow) -> Self {
        Self {
            key: row.key,
            label: row.label,
            current: row.current.into(),
            previous: row.previous.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PerformanceReportResponse {
    pub(crate) dimension: PerformanceDimensionSchema,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) period: DashboardPeriodResponse,
    pub(crate) previous_period: Option<DashboardPeriodResponse>,
    pub(crate) amounts_visible: bool,
    pub(crate) activities_visible: bool,
    pub(crate) rows: Vec<PerformanceRowResponse>,
    pub(crate) totals: PerformanceMetricsResponse,
    pub(crate) previous_totals: Option<PerformanceMetricsResponse>,
    pub(crate) truncated: bool,
}

impl From<PerformanceReport> for PerformanceReportResponse {
    fn from(report: PerformanceReport) -> Self {
        Self {
            dimension: report.dimension.into(),
            currency: report.currency,
            timezone: report.timezone,
            period: report.period.into(),
            previous_period: report.previous_period.map(Into::into),
            amounts_visible: report.amounts_visible,
            activities_visible: report.activities_visible,
            rows: report.rows.into_iter().map(Into::into).collect(),
            totals: report.totals.into(),
            previous_totals: report.previous_totals.map(Into::into),
            truncated: report.truncated,
        }
    }
}
