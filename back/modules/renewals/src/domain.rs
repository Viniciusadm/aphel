use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

pub(crate) const DEFAULT_RENEWAL_DAYS_BEFORE: u16 = 30;
pub(crate) const MAX_RENEWAL_DAYS_BEFORE: u16 = 365;
pub(crate) const DEFAULT_RENEWAL_TITLE_PREFIX: &str = "Renovação: ";
pub(crate) const MAX_RENEWAL_TITLE_PREFIX_LENGTH: usize = 60;

pub(crate) const PIPELINE_RENEWAL_UPDATED: &str = "pipeline.renewal_updated";
pub(crate) const DEAL_RENEWAL_CREATED: &str = "deal.renewal_created";

pub(crate) const RENEWALS_SCAN_JOB: &str = "renewals.scan";
pub(crate) const RENEWALS_SCAN_CRON: &str = "0 15 * * * *";

pub(crate) const RENEWAL_OF_DEAL_FIELD: &str = "renewal_of_deal_id";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum RenewalOwnerMode {
    #[default]
    Original,
    Unassigned,
}

impl RenewalOwnerMode {
    pub(crate) const ALL: [Self; 2] = [Self::Original, Self::Unassigned];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Unassigned => "none",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenewalSettings {
    pub(crate) pipeline_id: Uuid,
    pub(crate) is_enabled: bool,
    pub(crate) days_before: u16,
    pub(crate) target_pipeline_id: Uuid,
    pub(crate) target_stage_id: Uuid,
    pub(crate) title_prefix: String,
    pub(crate) owner_mode: RenewalOwnerMode,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl RenewalSettings {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "is_enabled": self.is_enabled,
            "days_before": self.days_before,
            "target_pipeline_id": self.target_pipeline_id.to_string(),
            "target_stage_id": self.target_stage_id.to_string(),
            "title_prefix": self.title_prefix,
            "owner_mode": self.owner_mode.as_str(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenewalSettingsView {
    pub(crate) settings: RenewalSettings,
    pub(crate) configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveRenewalSettingsCommand {
    pub(crate) is_enabled: Option<bool>,
    pub(crate) days_before: Option<i64>,
    pub(crate) target_pipeline_id: Option<String>,
    pub(crate) target_stage_id: Option<String>,
    pub(crate) title_prefix: Option<String>,
    pub(crate) owner_mode: Option<RenewalOwnerMode>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExistingRenewal {
    pub(crate) deal_id: Uuid,
    pub(crate) deleted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RenewalScanOutcome {
    pub(crate) created: u32,
    pub(crate) failed: u32,
}

pub(crate) fn renewal_title(prefix: &str, title: &str, max_length: usize) -> String {
    let full = format!("{prefix}{title}");
    if full.chars().count() <= max_length {
        return full;
    }
    full.chars()
        .take(max_length)
        .collect::<String>()
        .trim_end()
        .to_owned()
}
