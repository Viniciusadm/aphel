use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use aphel_sdk::dashboard::PeriodQuery;
use aphel_sdk::dashboard::ResolvedPeriod;
use workflow::domain::StageKind;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PerformanceDimension {
    Owner,
    Team,
    Source,
    Pipeline,
}

impl PerformanceDimension {
    pub(crate) fn field(&self) -> &'static str {
        match self {
            Self::Owner => "owner_id",
            Self::Team => "owner_team",
            Self::Source => "source",
            Self::Pipeline => "pipeline_id",
        }
    }

    pub(crate) fn activity_field(&self) -> Option<&'static str> {
        match self {
            Self::Owner => Some("owner_id"),
            Self::Team => Some("owner_team"),
            Self::Source | Self::Pipeline => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportStage {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) kind: StageKind,
    pub(crate) position: i32,
    pub(crate) probability: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportPipeline {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) stages: Vec<ReportStage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunnelBucket {
    pub(crate) furthest_position: Option<i64>,
    pub(crate) status: String,
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageLoss {
    pub(crate) stage_id: Option<Uuid>,
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageDuration {
    pub(crate) stage_id: Uuid,
    pub(crate) completed: u64,
    pub(crate) average_seconds: Option<Decimal>,
    pub(crate) open: u64,
    pub(crate) open_average_seconds: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CycleTime {
    pub(crate) key: Option<String>,
    pub(crate) count: u64,
    pub(crate) average_seconds: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstantRange {
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ConversionQuery {
    pub(crate) pipeline_id: Option<String>,
    pub(crate) period: PeriodQuery,
    pub(crate) compare_to_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PerformanceQuery {
    pub(crate) dimension: Option<PerformanceDimension>,
    pub(crate) pipeline_id: Option<String>,
    pub(crate) period: PeriodQuery,
    pub(crate) compare_to_previous: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConversionSummary {
    pub(crate) created: u64,
    pub(crate) created_amount: Option<Decimal>,
    pub(crate) open: u64,
    pub(crate) won: u64,
    pub(crate) won_amount: Option<Decimal>,
    pub(crate) lost: u64,
    pub(crate) lost_amount: Option<Decimal>,
    pub(crate) conversion_rate: Option<f64>,
    pub(crate) win_rate: Option<f64>,
    pub(crate) average_cycle_seconds: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageFunnel {
    pub(crate) stage_id: Uuid,
    pub(crate) reached: u64,
    pub(crate) reached_amount: Option<Decimal>,
    pub(crate) advanced: u64,
    pub(crate) passage_rate: Option<f64>,
    pub(crate) lost: u64,
    pub(crate) lost_amount: Option<Decimal>,
    pub(crate) completed_visits: u64,
    pub(crate) average_seconds: Option<Decimal>,
    pub(crate) open_visits: u64,
    pub(crate) open_average_seconds: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LostReasonTotal {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) count: u64,
    pub(crate) amount: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConversionSnapshot {
    pub(crate) period: ResolvedPeriod,
    pub(crate) summary: ConversionSummary,
    pub(crate) stages: Vec<StageFunnel>,
    pub(crate) unattributed_lost: u64,
    pub(crate) unattributed_lost_amount: Option<Decimal>,
    pub(crate) lost_reasons: Vec<LostReasonTotal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConversionReport {
    pub(crate) pipeline: Option<ReportPipeline>,
    pub(crate) currency: String,
    pub(crate) timezone: String,
    pub(crate) amounts_visible: bool,
    pub(crate) current: Option<ConversionSnapshot>,
    pub(crate) previous: Option<ConversionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PerformanceMetrics {
    pub(crate) created: u64,
    pub(crate) won: u64,
    pub(crate) won_amount: Option<Decimal>,
    pub(crate) lost: u64,
    pub(crate) conversion_rate: Option<f64>,
    pub(crate) average_cycle_seconds: Option<Decimal>,
    pub(crate) activities_done: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PerformanceRow {
    pub(crate) key: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) current: PerformanceMetrics,
    pub(crate) previous: Option<PerformanceMetrics>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PerformanceReport {
    pub(crate) dimension: PerformanceDimension,
    pub(crate) currency: String,
    pub(crate) timezone: String,
    pub(crate) period: ResolvedPeriod,
    pub(crate) previous_period: Option<ResolvedPeriod>,
    pub(crate) amounts_visible: bool,
    pub(crate) activities_visible: bool,
    pub(crate) rows: Vec<PerformanceRow>,
    pub(crate) totals: PerformanceMetrics,
    pub(crate) previous_totals: Option<PerformanceMetrics>,
    pub(crate) truncated: bool,
}
