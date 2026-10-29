use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use utoipa::{IntoParams, ToSchema};

use crate::engine::EngineOutcome;
use crate::{
    Automation, AutomationRun, AutomationStats, AutomationSummary, AutomationTrigger,
    AutomationVersion, CreateAutomationCommand, DryRunCommand, DryRunResult, MAX_CHAIN_DEPTH,
    MAX_STEPS, RUNS_PER_RECORD_LIMIT, RUNS_PER_RECORD_WINDOW_MINUTES, RunDetails, RunStep,
    UpdateAutomationCommand,
};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::query::FilterNodeSchema;
use aphel_kernel::application::automations::actions::AutomationAction;
use aphel_kernel::application::automations::definition::{
    AutomationDefinition, AutomationOptions, BranchCase, StepBody, StepSpec,
};
use aphel_kernel::application::automations::templates::{TEMPLATE_ROOTS, TRIGGER_PATHS};
use aphel_kernel::application::automations::time::triggers::MIN_SCHEDULE_INTERVAL_MINUTES;
use aphel_kernel::application::automations::time::waits::WAIT_UNTIL_POLL_MINUTES;
use aphel_kernel::application::query::Filter;
use aphel_kernel::domain::automations::{
    AutomationStatus, RunStatus, SkipReason, StepKind, StepStatus,
};

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn empty_definition() -> Value {
    json!({ "steps": [] })
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationStatus)]
pub(crate) enum AutomationStatusSchema {
    Draft,
    Active,
    Paused,
}

impl From<AutomationStatus> for AutomationStatusSchema {
    fn from(value: AutomationStatus) -> Self {
        match value {
            AutomationStatus::Draft => Self::Draft,
            AutomationStatus::Active => Self::Active,
            AutomationStatus::Paused => Self::Paused,
        }
    }
}

impl From<AutomationStatusSchema> for AutomationStatus {
    fn from(value: AutomationStatusSchema) -> Self {
        match value {
            AutomationStatusSchema::Draft => Self::Draft,
            AutomationStatusSchema::Active => Self::Active,
            AutomationStatusSchema::Paused => Self::Paused,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationRunStatus)]
pub(crate) enum RunStatusSchema {
    Running,
    Waiting,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl From<RunStatus> for RunStatusSchema {
    fn from(value: RunStatus) -> Self {
        match value {
            RunStatus::Running => Self::Running,
            RunStatus::Waiting => Self::Waiting,
            RunStatus::Succeeded => Self::Succeeded,
            RunStatus::Failed => Self::Failed,
            RunStatus::Skipped => Self::Skipped,
            RunStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<RunStatusSchema> for RunStatus {
    fn from(value: RunStatusSchema) -> Self {
        match value {
            RunStatusSchema::Running => Self::Running,
            RunStatusSchema::Waiting => Self::Waiting,
            RunStatusSchema::Succeeded => Self::Succeeded,
            RunStatusSchema::Failed => Self::Failed,
            RunStatusSchema::Skipped => Self::Skipped,
            RunStatusSchema::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationStepStatus)]
pub(crate) enum StepStatusSchema {
    Succeeded,
    Failed,
    Skipped,
    Planned,
    Waiting,
}

impl From<StepStatus> for StepStatusSchema {
    fn from(value: StepStatus) -> Self {
        match value {
            StepStatus::Succeeded => Self::Succeeded,
            StepStatus::Failed => Self::Failed,
            StepStatus::Skipped => Self::Skipped,
            StepStatus::Planned => Self::Planned,
            StepStatus::Waiting => Self::Waiting,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationStepType)]
pub(crate) enum StepTypeSchema {
    Condition,
    Branch,
    Action,
    Wait,
    WaitUntil,
}

impl From<StepKind> for StepTypeSchema {
    fn from(value: StepKind) -> Self {
        match value {
            StepKind::Condition => Self::Condition,
            StepKind::Branch => Self::Branch,
            StepKind::Action => Self::Action,
            StepKind::Wait => Self::Wait,
            StepKind::WaitUntil => Self::WaitUntil,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationSkipReason)]
pub(crate) enum SkipReasonSchema {
    MaxDepth,
    CausationCycle,
    RateLimited,
    RecordNotFound,
    RecordNotAccessible,
    PublisherUnavailable,
    AutomationInactive,
    RecordDeleted,
    AutomationPaused,
    VersionPublished,
}

impl From<SkipReason> for SkipReasonSchema {
    fn from(value: SkipReason) -> Self {
        match value {
            SkipReason::MaxDepth => Self::MaxDepth,
            SkipReason::CausationCycle => Self::CausationCycle,
            SkipReason::RateLimited => Self::RateLimited,
            SkipReason::RecordNotFound => Self::RecordNotFound,
            SkipReason::RecordNotAccessible => Self::RecordNotAccessible,
            SkipReason::PublisherUnavailable => Self::PublisherUnavailable,
            SkipReason::AutomationInactive => Self::AutomationInactive,
            SkipReason::RecordDeleted => Self::RecordDeleted,
            SkipReason::AutomationPaused => Self::AutomationPaused,
            SkipReason::VersionPublished => Self::VersionPublished,
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, ToSchema)]
#[schema(as = AutomationTrigger)]
pub(crate) struct TriggerSchema {
    #[serde(rename = "type")]
    #[schema(example = "deal.stage_changed")]
    pub(crate) kind: String,
    #[schema(value_type = Option<String>, example = "0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f")]
    pub(crate) to_stage_id: Option<String>,
    #[schema(value_type = Option<String>)]
    pub(crate) from_stage_id: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    pub(crate) fields: Option<Vec<String>>,
    #[schema(value_type = Option<String>)]
    pub(crate) incoming_webhook_id: Option<String>,
    #[schema(value_type = Option<String>, example = "expected_close_date")]
    pub(crate) field: Option<String>,
    #[schema(value_type = Option<String>, example = "-3d")]
    pub(crate) offset: Option<String>,
    #[schema(value_type = Option<String>, example = "09:00")]
    pub(crate) at: Option<String>,
    #[schema(value_type = Option<StaleModeSchema>)]
    pub(crate) mode: Option<String>,
    #[schema(value_type = Option<u32>, example = 7)]
    pub(crate) days: Option<u32>,
    #[schema(value_type = Option<String>, example = "0 0 9 * * MON")]
    pub(crate) cron: Option<String>,
    #[schema(no_recursion)]
    pub(crate) filter: Option<FilterNodeSchema>,
    #[schema(value_type = Option<String>)]
    pub(crate) segment_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = AutomationStaleMode)]
pub(crate) enum StaleModeSchema {
    NoUpdate,
    NoActivity,
    SameStage,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = AutomationOptions)]
pub(crate) struct OptionsSchema {
    #[schema(default = true)]
    pub(crate) cancel_waiting_on_pause: Option<bool>,
    #[schema(default = false)]
    pub(crate) cancel_waiting_on_publish: Option<bool>,
}

impl From<AutomationOptions> for OptionsSchema {
    fn from(options: AutomationOptions) -> Self {
        Self {
            cancel_waiting_on_pause: Some(options.cancel_waiting_on_pause),
            cancel_waiting_on_publish: Some(options.cancel_waiting_on_publish),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = AutomationBranchCase)]
pub(crate) struct BranchCaseSchema {
    pub(crate) key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[schema(value_type = FilterNodeSchema)]
    pub(crate) filter: Filter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next: Option<String>,
}

impl From<BranchCase> for BranchCaseSchema {
    fn from(case: BranchCase) -> Self {
        Self {
            key: case.key,
            label: case.label,
            filter: case.filter,
            next: case.next,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = AutomationStep)]
pub(crate) struct StepSchema {
    #[schema(example = "check_open")]
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
    #[serde(rename = "type")]
    pub(crate) kind: StepTypeSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<FilterNodeSchema>, no_recursion)]
    pub(crate) filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cases: Option<Vec<BranchCaseSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "create_task")]
    pub(crate) action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub(crate) config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2d")]
    pub(crate) duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "expected_close_date")]
    pub(crate) until_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "-1d")]
    pub(crate) offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<FilterNodeSchema>, no_recursion)]
    pub(crate) recheck: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "7d")]
    pub(crate) timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeout_next: Option<String>,
}

impl From<StepSpec> for StepSchema {
    fn from(step: StepSpec) -> Self {
        let kind = step.body.kind().into();
        let empty = Self {
            id: step.id,
            label: step.label,
            kind,
            filter: None,
            next: None,
            cases: None,
            default: None,
            action: None,
            config: None,
            duration: None,
            until_field: None,
            offset: None,
            recheck: None,
            timeout: None,
            timeout_next: None,
        };
        match step.body {
            StepBody::Condition { filter, next } => Self {
                filter: Some(filter),
                next,
                ..empty
            },
            StepBody::Branch { cases, default } => Self {
                cases: Some(cases.into_iter().map(Into::into).collect()),
                default,
                ..empty
            },
            StepBody::Action {
                action,
                config,
                next,
            } => Self {
                action: Some(action),
                config: Some(config),
                next,
                ..empty
            },
            StepBody::Wait {
                duration,
                until_field,
                offset,
                recheck,
                next,
            } => Self {
                duration,
                until_field,
                offset,
                recheck,
                next,
                ..empty
            },
            StepBody::WaitUntil {
                filter,
                timeout,
                next,
                timeout_next,
            } => Self {
                filter: Some(filter),
                timeout: Some(timeout),
                next,
                timeout_next,
                ..empty
            },
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = AutomationDefinition)]
pub(crate) struct DefinitionSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "check_open")]
    pub(crate) start: Option<String>,
    pub(crate) steps: Vec<StepSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<OptionsSchema>,
}

impl From<AutomationDefinition> for DefinitionSchema {
    fn from(definition: AutomationDefinition) -> Self {
        Self {
            start: definition.start,
            steps: definition.steps.into_iter().map(Into::into).collect(),
            options: definition.options.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateAutomationRequest {
    #[schema(example = "follow_up_proposal")]
    pub(crate) key: Option<String>,
    #[schema(example = "Follow up on sent proposal")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(example = "deal")]
    pub(crate) record_type: String,
    #[schema(value_type = TriggerSchema)]
    pub(crate) trigger: Value,
    #[serde(default = "empty_definition")]
    #[schema(value_type = Option<DefinitionSchema>)]
    pub(crate) definition: Value,
}

impl CreateAutomationRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateAutomationCommand {
        CreateAutomationCommand {
            key: self.key,
            name: self.name,
            description: self.description,
            record_type: self.record_type,
            trigger: self.trigger,
            definition: self.definition,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateAutomationRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(value_type = Option<TriggerSchema>)]
    pub(crate) trigger: Option<Value>,
    #[schema(value_type = Option<DefinitionSchema>)]
    pub(crate) definition: Option<Value>,
}

impl UpdateAutomationRequest {
    pub(crate) fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateAutomationCommand {
        UpdateAutomationCommand {
            name: self.name,
            description: self.description,
            trigger: self.trigger,
            definition: self.definition,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct DryRunRequest {
    pub(crate) record_id: String,
    #[serde(default)]
    pub(crate) use_draft: bool,
    #[serde(default)]
    pub(crate) changed_fields: Vec<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) previous: Map<String, Value>,
}

impl DryRunRequest {
    pub(crate) fn into_command(self) -> DryRunCommand {
        DryRunCommand {
            record_id: self.record_id,
            use_draft: self.use_draft,
            changed_fields: self.changed_fields,
            previous: self.previous,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RunAutomationRequest {
    pub(crate) record_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListAutomationsParams {
    pub(crate) status: Option<AutomationStatusSchema>,
    pub(crate) record_type: Option<String>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListRunsParams {
    pub(crate) status: Option<RunStatusSchema>,
    pub(crate) record_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    #[schema(value_type = TriggerSchema)]
    pub(crate) trigger: Value,
    pub(crate) definition: DefinitionSchema,
    pub(crate) status: AutomationStatusSchema,
    pub(crate) version: u32,
    pub(crate) created_by: Option<String>,
    pub(crate) updated_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationStatsResponse {
    pub(crate) last_run_at: Option<String>,
    pub(crate) runs_30d: u64,
    pub(crate) failed_30d: u64,
    pub(crate) waiting: u64,
}

impl From<AutomationStats> for AutomationStatsResponse {
    fn from(stats: AutomationStats) -> Self {
        Self {
            last_run_at: stats.last_run_at.map(timestamp),
            runs_30d: stats.runs_30d,
            failed_30d: stats.failed_30d,
            waiting: stats.waiting,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationListItemResponse {
    #[serde(flatten)]
    pub(crate) automation: AutomationResponse,
    pub(crate) stats: AutomationStatsResponse,
}

impl From<AutomationSummary> for AutomationListItemResponse {
    fn from(summary: AutomationSummary) -> Self {
        Self {
            automation: summary.automation.into(),
            stats: summary.stats.into(),
        }
    }
}

impl From<Automation> for AutomationResponse {
    fn from(automation: Automation) -> Self {
        Self {
            id: automation.id.to_string(),
            key: automation.key,
            name: automation.name,
            description: automation.description,
            record_type: automation.record_type,
            trigger: to_value(&automation.trigger),
            definition: automation.definition.into(),
            status: automation.status.into(),
            version: automation.version,
            created_by: automation.created_by.map(|user| user.as_str().to_owned()),
            updated_by: automation.updated_by.map(|user| user.as_str().to_owned()),
            created_at: timestamp(automation.created_at),
            updated_at: timestamp(automation.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationVersionResponse {
    pub(crate) id: String,
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) record_type: String,
    #[schema(value_type = TriggerSchema)]
    pub(crate) trigger: Value,
    pub(crate) definition: DefinitionSchema,
    pub(crate) published_by: Option<String>,
    pub(crate) published_at: String,
}

impl From<AutomationVersion> for AutomationVersionResponse {
    fn from(version: AutomationVersion) -> Self {
        Self {
            id: version.id.to_string(),
            version: version.version,
            name: version.name,
            record_type: version.record_type,
            trigger: to_value(&version.trigger),
            definition: version.definition.into(),
            published_by: version.published_by.map(|user| user.as_str().to_owned()),
            published_at: timestamp(version.published_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationRunResponse {
    pub(crate) id: String,
    pub(crate) automation_id: String,
    pub(crate) version: u32,
    pub(crate) trigger_type: String,
    pub(crate) trigger_event_id: Option<String>,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) status: RunStatusSchema,
    pub(crate) reason: Option<SkipReasonSchema>,
    pub(crate) error: Option<String>,
    pub(crate) depth: u32,
    pub(crate) chain: Vec<String>,
    pub(crate) attempts: u32,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) resume_at: Option<String>,
    pub(crate) waiting_step_id: Option<String>,
}

impl From<AutomationRun> for AutomationRunResponse {
    fn from(run: AutomationRun) -> Self {
        Self {
            id: run.id.to_string(),
            automation_id: run.automation_id.to_string(),
            version: run.version,
            trigger_type: run.trigger_type,
            trigger_event_id: run.trigger_event_id.map(|id| id.to_string()),
            record_type: run.record.record_type.as_str().to_owned(),
            record_id: run.record.record_id.to_string(),
            status: run.status.into(),
            reason: run.reason.map(Into::into),
            error: run.error,
            depth: run.depth,
            chain: run.chain.iter().map(ToString::to_string).collect(),
            attempts: run.attempts,
            started_at: timestamp(run.started_at),
            finished_at: run.finished_at.map(timestamp),
            resume_at: run.resume_at.map(timestamp),
            waiting_step_id: run.waiting_step_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RunStepResponse {
    pub(crate) step_id: String,
    pub(crate) position: u32,
    pub(crate) kind: StepTypeSchema,
    pub(crate) action: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub(crate) input: Option<Value>,
    pub(crate) condition_result: Option<bool>,
    pub(crate) next_step_id: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub(crate) output: Option<Value>,
    pub(crate) status: StepStatusSchema,
    pub(crate) error: Option<String>,
    pub(crate) started_at: String,
    pub(crate) duration_ms: u32,
}

impl From<RunStep> for RunStepResponse {
    fn from(step: RunStep) -> Self {
        Self {
            step_id: step.step_id,
            position: step.position,
            kind: step.kind.into(),
            action: step.action,
            input: step.input,
            condition_result: step.condition_result,
            next_step_id: step.next_step_id,
            output: step.output,
            status: step.status.into(),
            error: step.error,
            started_at: timestamp(step.started_at),
            duration_ms: step.duration_ms,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationRunDetailsResponse {
    #[serde(flatten)]
    pub(crate) run: AutomationRunResponse,
    pub(crate) steps: Vec<RunStepResponse>,
}

impl From<RunDetails> for AutomationRunDetailsResponse {
    fn from(details: RunDetails) -> Self {
        Self {
            run: details.run.into(),
            steps: details.steps.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DryRunResponse {
    pub(crate) version: Option<u32>,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) record_available: bool,
    #[schema(example = "completed")]
    pub(crate) outcome: &'static str,
    pub(crate) error: Option<String>,
    pub(crate) steps: Vec<RunStepResponse>,
}

impl From<DryRunResult> for DryRunResponse {
    fn from(result: DryRunResult) -> Self {
        let (outcome, error) = match result.outcome {
            EngineOutcome::Completed => ("completed", None),
            EngineOutcome::Failed { error, .. } => ("failed", Some(error)),
            EngineOutcome::Waiting { .. } => ("waiting", None),
        };
        Self {
            version: result.version,
            record_type: result.record.record_type.as_str().to_owned(),
            record_id: result.record.record_id.to_string(),
            record_available: result.record_available,
            outcome,
            error,
            steps: result.steps.into_iter().map(Into::into).collect(),
        }
    }
}

const CATALOG_RECORD_TYPES: [&str; 4] = ["contact", "company", "deal", "activity"];
const CUSTOM_SAMPLE: &str = "co_sample";

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TriggerCatalogEntry {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
    pub(crate) record_types: Vec<&'static str>,
    pub(crate) custom_objects: bool,
    pub(crate) manual: bool,
    pub(crate) time_based: bool,
    #[schema(value_type = Object)]
    pub(crate) options_schema: Value,
}

impl TriggerCatalogEntry {
    pub(crate) fn from_trigger(trigger: &dyn AutomationTrigger) -> Self {
        Self {
            key: trigger.key(),
            label: trigger.label(),
            description: trigger.description(),
            record_types: CATALOG_RECORD_TYPES
                .into_iter()
                .filter(|record_type| trigger.supports(record_type))
                .collect(),
            custom_objects: trigger.supports(CUSTOM_SAMPLE),
            manual: trigger.is_manual(),
            time_based: trigger.is_time_based(),
            options_schema: trigger.options_schema(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActionCatalogEntry {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
    pub(crate) record_types: Vec<&'static str>,
    pub(crate) custom_objects: bool,
    pub(crate) secret_fields: Vec<&'static str>,
    #[schema(value_type = Object)]
    pub(crate) config_schema: Value,
}

impl ActionCatalogEntry {
    pub(crate) fn from_action(action: &dyn AutomationAction) -> Self {
        Self {
            key: action.key(),
            label: action.label(),
            description: action.description(),
            record_types: CATALOG_RECORD_TYPES
                .into_iter()
                .filter(|record_type| action.supports(record_type))
                .collect(),
            custom_objects: action.supports(CUSTOM_SAMPLE),
            secret_fields: action.secret_fields().to_vec(),
            config_schema: action.config_schema(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationLimitsResponse {
    pub(crate) max_steps: usize,
    pub(crate) max_chain_depth: u32,
    pub(crate) runs_per_record_limit: u64,
    pub(crate) runs_per_record_window_minutes: i64,
    pub(crate) wait_until_poll_minutes: i64,
    pub(crate) min_schedule_interval_minutes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AutomationCatalogResponse {
    pub(crate) triggers: Vec<TriggerCatalogEntry>,
    pub(crate) actions: Vec<ActionCatalogEntry>,
    pub(crate) template_roots: Vec<&'static str>,
    pub(crate) trigger_paths: Vec<&'static str>,
    pub(crate) limits: AutomationLimitsResponse,
}

impl AutomationCatalogResponse {
    pub(crate) fn new(
        triggers: Vec<TriggerCatalogEntry>,
        actions: Vec<ActionCatalogEntry>,
    ) -> Self {
        Self {
            triggers,
            actions,
            template_roots: TEMPLATE_ROOTS.to_vec(),
            trigger_paths: TRIGGER_PATHS.to_vec(),
            limits: AutomationLimitsResponse {
                max_steps: MAX_STEPS,
                max_chain_depth: MAX_CHAIN_DEPTH,
                runs_per_record_limit: RUNS_PER_RECORD_LIMIT,
                runs_per_record_window_minutes: RUNS_PER_RECORD_WINDOW_MINUTES,
                wait_until_poll_minutes: WAIT_UNTIL_POLL_MINUTES,
                min_schedule_interval_minutes: MIN_SCHEDULE_INTERVAL_MINUTES,
            },
        }
    }
}
