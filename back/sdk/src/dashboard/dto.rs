use super::{PeriodPreset, PeriodSelection, ResolvedPeriod};
use aphel_kernel::api::dto::common::timestamp as instant;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DashboardPeriodPreset)]
pub enum PeriodPresetSchema {
    Today,
    Yesterday,
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[serde(rename = "last_30_days")]
    Last30Days,
    Week,
    LastWeek,
    Month,
    LastMonth,
    Quarter,
    LastQuarter,
    Year,
    LastYear,
    Custom,
}

impl From<PeriodPreset> for PeriodPresetSchema {
    fn from(preset: PeriodPreset) -> Self {
        match preset {
            PeriodPreset::Today => Self::Today,
            PeriodPreset::Yesterday => Self::Yesterday,
            PeriodPreset::Last7Days => Self::Last7Days,
            PeriodPreset::Last30Days => Self::Last30Days,
            PeriodPreset::Week => Self::Week,
            PeriodPreset::LastWeek => Self::LastWeek,
            PeriodPreset::Month => Self::Month,
            PeriodPreset::LastMonth => Self::LastMonth,
            PeriodPreset::Quarter => Self::Quarter,
            PeriodPreset::LastQuarter => Self::LastQuarter,
            PeriodPreset::Year => Self::Year,
            PeriodPreset::LastYear => Self::LastYear,
            PeriodPreset::Custom => Self::Custom,
        }
    }
}

impl From<PeriodPresetSchema> for PeriodPreset {
    fn from(preset: PeriodPresetSchema) -> Self {
        match preset {
            PeriodPresetSchema::Today => Self::Today,
            PeriodPresetSchema::Yesterday => Self::Yesterday,
            PeriodPresetSchema::Last7Days => Self::Last7Days,
            PeriodPresetSchema::Last30Days => Self::Last30Days,
            PeriodPresetSchema::Week => Self::Week,
            PeriodPresetSchema::LastWeek => Self::LastWeek,
            PeriodPresetSchema::Month => Self::Month,
            PeriodPresetSchema::LastMonth => Self::LastMonth,
            PeriodPresetSchema::Quarter => Self::Quarter,
            PeriodPresetSchema::LastQuarter => Self::LastQuarter,
            PeriodPresetSchema::Year => Self::Year,
            PeriodPresetSchema::LastYear => Self::LastYear,
            PeriodPresetSchema::Custom => Self::Custom,
        }
    }
}

impl PeriodPresetSchema {
    pub fn as_str(self) -> &'static str {
        PeriodPreset::from(self).as_str()
    }

    pub fn stored(value: &str) -> Self {
        PeriodPreset::parse(value)
            .unwrap_or(PeriodSelection::default().preset)
            .into()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardPeriodResponse {
    pub preset: PeriodPresetSchema,
    #[schema(example = "2026-04-01")]
    pub from: String,
    #[schema(example = "2026-04-30")]
    pub to: String,
    pub starts_at: String,
    pub ends_at: String,
}

impl From<ResolvedPeriod> for DashboardPeriodResponse {
    fn from(period: ResolvedPeriod) -> Self {
        Self {
            preset: period.preset.into(),
            from: period.from.format("%Y-%m-%d").to_string(),
            to: period.to.format("%Y-%m-%d").to_string(),
            starts_at: instant(period.starts_at),
            ends_at: instant(period.ends_at),
        }
    }
}
