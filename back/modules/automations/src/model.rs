use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use aphel_contract_automation::application::context::TriggerContext;
use aphel_contract_automation::application::definition::{AutomationDefinition, TriggerSpec};
use aphel_contract_automation::domain::{
    AutomationStatus, RunStatus, SkipReason, StepKind, StepStatus,
};
use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::shared::{RecordRef, RecordType};

pub(crate) const AUTOMATION_RECORD_TYPE: RecordType =
    RecordType::from_static("automations.automation");

pub(crate) const AUTOMATION_CREATED: &str = "automation.created";
pub(crate) const AUTOMATION_UPDATED: &str = "automation.updated";
pub(crate) const AUTOMATION_PUBLISHED: &str = "automation.published";
pub(crate) const AUTOMATION_PAUSED: &str = "automation.paused";
pub(crate) const AUTOMATION_ACTIVATED: &str = "automation.activated";
pub(crate) const AUTOMATION_DELETED: &str = "automation.deleted";
pub(crate) const AUTOMATION_RUN_SKIPPED: &str = "automation.run_skipped";

pub(crate) const AUTOMATION_NOT_FOUND: &str = "automation_not_found";
pub(crate) const AUTOMATION_RUN_NOT_FOUND: &str = "automation_run_not_found";
pub(crate) const AUTOMATION_KEY_TAKEN: &str = "automation_key_taken";
pub(crate) const AUTOMATION_NOT_PUBLISHED: &str = "automation_not_published";
pub(crate) const AUTOMATION_NOT_MANUAL: &str = "automation_not_manual";
pub(crate) const AUTOMATION_NOT_ACTIVE: &str = "automation_not_active";
pub(crate) const AUTOMATION_LIMIT_REACHED: &str = "automation_limit_reached";
pub(crate) const INVALID_AUTOMATION: &str = "invalid_automation";

pub(crate) const MAX_AUTOMATIONS: u64 = 200;
pub(crate) const MAX_NAME_LENGTH: usize = 140;
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 1000;
pub(crate) const MAX_KEY_LENGTH: usize = 64;
pub(crate) const MAX_CHAIN_DEPTH: u32 = 5;
pub(crate) const RUNS_PER_RECORD_LIMIT: u64 = 20;
pub(crate) const RUNS_PER_RECORD_WINDOW_MINUTES: i64 = 60;
pub(crate) const SKIP_NOTICE_WINDOW_MINUTES: i64 = 60;
pub(crate) const LOCK_WAIT_SECONDS: u32 = 10;
pub(crate) const MAX_ERROR_LENGTH: usize = 1000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Automation {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) trigger: TriggerSpec,
    pub(crate) definition: AutomationDefinition,
    pub(crate) status: AutomationStatus,
    pub(crate) version: u32,
    pub(crate) created_by: Option<UserId>,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AutomationVersion {
    pub(crate) id: Uuid,
    pub(crate) automation_id: Uuid,
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) record_type: String,
    pub(crate) trigger: TriggerSpec,
    pub(crate) definition: AutomationDefinition,
    pub(crate) published_by: Option<UserId>,
    pub(crate) published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveAutomation {
    pub(crate) automation: Automation,
    pub(crate) version: AutomationVersion,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AutomationRun {
    pub(crate) id: Uuid,
    pub(crate) automation_id: Uuid,
    pub(crate) version: u32,
    pub(crate) trigger_key: String,
    pub(crate) trigger_event_id: Option<Uuid>,
    pub(crate) trigger_type: String,
    pub(crate) record: RecordRef,
    pub(crate) status: RunStatus,
    pub(crate) depth: u32,
    pub(crate) chain: Vec<Uuid>,
    pub(crate) reason: Option<SkipReason>,
    pub(crate) error: Option<String>,
    pub(crate) attempts: u32,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
    pub(crate) resume_at: Option<DateTime<Utc>>,
    pub(crate) waiting_step_id: Option<String>,
    pub(crate) context: Option<TriggerContext>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunStep {
    pub(crate) id: Uuid,
    pub(crate) run_id: Uuid,
    pub(crate) position: u32,
    pub(crate) step_id: String,
    pub(crate) kind: StepKind,
    pub(crate) action: Option<String>,
    pub(crate) input: Option<Value>,
    pub(crate) condition_result: Option<bool>,
    pub(crate) next_step_id: Option<String>,
    pub(crate) output: Option<Value>,
    pub(crate) status: StepStatus,
    pub(crate) error: Option<String>,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) duration_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunDetails {
    pub(crate) run: AutomationRun,
    pub(crate) steps: Vec<RunStep>,
}

pub(crate) const STATS_WINDOW_DAYS: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AutomationStats {
    pub(crate) last_run_at: Option<DateTime<Utc>>,
    pub(crate) runs_30d: u64,
    pub(crate) failed_30d: u64,
    pub(crate) waiting: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AutomationSummary {
    pub(crate) automation: Automation,
    pub(crate) stats: AutomationStats,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AutomationListFilter {
    pub(crate) status: Option<AutomationStatus>,
    pub(crate) record_type: Option<String>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RunListFilter {
    pub(crate) status: Option<RunStatus>,
    pub(crate) record_id: Option<Uuid>,
}

pub(crate) fn truncate_error(message: &str) -> String {
    message.chars().take(MAX_ERROR_LENGTH).collect()
}
