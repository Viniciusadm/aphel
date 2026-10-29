use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::RecordType;
use deals::team::domain::CreditedUsers;

pub(crate) use aphel_kernel::application::shared::is_access_denied as is_unavailable;

pub(crate) const DEFAULT_HORIZON_MONTHS: u8 = 6;
pub(crate) const MAX_HORIZON_MONTHS: u8 = 18;
pub(crate) const DEFAULT_BEST_CASE_MIN_PROBABILITY: u8 = 50;
pub(crate) const DEFAULT_COMMIT_MIN_PROBABILITY: u8 = 90;

pub(crate) const FORECAST_SETTINGS_UPDATED: &str = "forecast_settings.updated";
pub(crate) const FORECAST_SETTINGS_RECORD_TYPE: RecordType =
    RecordType::from_static("forecast_settings");
pub(crate) const MONTH_FORMAT: &str = "%Y-%m";
pub(crate) const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ForecastMethod {
    StageProbability,
    DealProbability,
    CommitOnly,
}

impl ForecastMethod {
    pub(crate) const ALL: [Self; 3] = [
        Self::StageProbability,
        Self::DealProbability,
        Self::CommitOnly,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::StageProbability => "stage_probability",
            Self::DealProbability => "deal_probability",
            Self::CommitOnly => "commit_only",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|method| method.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ForecastCategory {
    Pipeline,
    BestCase,
    Commit,
}

impl ForecastCategory {
    pub(crate) const ALL: [Self; 3] = [Self::Pipeline, Self::BestCase, Self::Commit];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForecastSettings {
    pub(crate) method: ForecastMethod,
    pub(crate) categories_enabled: bool,
    pub(crate) best_case_min_probability: u8,
    pub(crate) commit_min_probability: u8,
    pub(crate) horizon_months: u8,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

impl Default for ForecastSettings {
    fn default() -> Self {
        Self {
            method: ForecastMethod::DealProbability,
            categories_enabled: false,
            best_case_min_probability: DEFAULT_BEST_CASE_MIN_PROBABILITY,
            commit_min_probability: DEFAULT_COMMIT_MIN_PROBABILITY,
            horizon_months: DEFAULT_HORIZON_MONTHS,
            updated_at: None,
        }
    }
}

impl ForecastSettings {
    pub(crate) fn uses_categories(&self) -> bool {
        self.categories_enabled || self.method == ForecastMethod::CommitOnly
    }

    pub(crate) fn category(&self, probability: u8) -> ForecastCategory {
        if probability >= self.commit_min_probability {
            ForecastCategory::Commit
        } else if probability >= self.best_case_min_probability {
            ForecastCategory::BestCase
        } else {
            ForecastCategory::Pipeline
        }
    }

    pub(crate) fn probability_range(&self, category: ForecastCategory) -> (u8, u8) {
        match category {
            ForecastCategory::Pipeline => (0, self.best_case_min_probability.saturating_sub(1)),
            ForecastCategory::BestCase => (
                self.best_case_min_probability,
                self.commit_min_probability.saturating_sub(1),
            ),
            ForecastCategory::Commit => (self.commit_min_probability, u8::MAX),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbabilitySource {
    Stage,
    DealOrStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForecastGroupRow {
    pub(crate) month: NaiveDate,
    pub(crate) probability: u8,
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForecastDealsRequest {
    pub(crate) source: ProbabilitySource,
    pub(crate) probability_range: Option<(u8, u8)>,
    pub(crate) window: ListWindow,
    pub(crate) credited: Option<CreditedUsers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForecastDeal {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) amount: Option<Decimal>,
    pub(crate) currency: String,
    pub(crate) probability: u8,
    pub(crate) stage_name: String,
    pub(crate) owner: Option<UserReference>,
    pub(crate) expected_close_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateForecastSettingsCommand {
    pub(crate) method: Option<String>,
    pub(crate) categories_enabled: Option<bool>,
    pub(crate) best_case_min_probability: Option<i64>,
    pub(crate) commit_min_probability: Option<i64>,
    pub(crate) horizon_months: Option<i64>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ForecastOwnerFilter {
    pub(crate) owner_id: Option<String>,
    pub(crate) team_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ForecastReportQuery {
    pub(crate) owners: ForecastOwnerFilter,
    pub(crate) months: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForecastDealsQuery {
    pub(crate) owners: ForecastOwnerFilter,
    pub(crate) month: String,
    pub(crate) category: Option<ForecastCategory>,
    pub(crate) window: ListWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ForecastBucket {
    pub(crate) count: u64,
    pub(crate) amount: Decimal,
}

impl ForecastBucket {
    pub(crate) fn add(&mut self, count: u64, amount: Decimal) {
        self.count += count;
        self.amount += amount;
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ForecastSummary {
    pub(crate) open: ForecastBucket,
    pub(crate) forecast: Decimal,
    pub(crate) categories: Option<BTreeMap<ForecastCategory, ForecastBucket>>,
    pub(crate) won: Option<ForecastBucket>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ForecastMonth {
    pub(crate) month: NaiveDate,
    pub(crate) summary: ForecastSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ForecastReport {
    pub(crate) settings: ForecastSettings,
    pub(crate) currency: String,
    pub(crate) timezone: String,
    pub(crate) from: NaiveDate,
    pub(crate) to: NaiveDate,
    pub(crate) probability_source: ProbabilitySource,
    pub(crate) months: Vec<ForecastMonth>,
    pub(crate) totals: ForecastSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ForecastDealView {
    pub(crate) deal: ForecastDeal,
    pub(crate) category: Option<ForecastCategory>,
    pub(crate) weighted_amount: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ForecastDealsPage {
    pub(crate) currency: String,
    pub(crate) month: NaiveDate,
    pub(crate) listing: Listing<ForecastDealView>,
}

pub(crate) fn weighted(amount: Decimal, probability: u8) -> Decimal {
    amount * Decimal::from(probability.min(100)) / Decimal::from(100)
}
