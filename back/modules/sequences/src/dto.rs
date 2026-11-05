use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::{EnrollmentSource, EnrollmentStatus, ExitReason, StepKind, format_clock};
use crate::{
    EnrollmentCounts, EnrollmentFilter, EnrollmentView, SaveSequenceCommand, SequenceListQuery,
    SequenceMetrics, SequenceStep, SequenceSummary, StepInput, StepMetrics,
};
use aphel_kernel::domain::UserId;
use aphel_sdk::platform::api::dto::common::double_option;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SequenceStepKind)]
pub(crate) enum SequenceStepKindSchema {
    AutoEmail,
    ManualEmail,
    Call,
    Linkedin,
    Task,
    Wait,
}

impl From<StepKind> for SequenceStepKindSchema {
    fn from(value: StepKind) -> Self {
        match value {
            StepKind::AutoEmail => Self::AutoEmail,
            StepKind::ManualEmail => Self::ManualEmail,
            StepKind::Call => Self::Call,
            StepKind::Linkedin => Self::Linkedin,
            StepKind::Task => Self::Task,
            StepKind::Wait => Self::Wait,
        }
    }
}

impl From<SequenceStepKindSchema> for StepKind {
    fn from(value: SequenceStepKindSchema) -> Self {
        match value {
            SequenceStepKindSchema::AutoEmail => Self::AutoEmail,
            SequenceStepKindSchema::ManualEmail => Self::ManualEmail,
            SequenceStepKindSchema::Call => Self::Call,
            SequenceStepKindSchema::Linkedin => Self::Linkedin,
            SequenceStepKindSchema::Task => Self::Task,
            SequenceStepKindSchema::Wait => Self::Wait,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SequenceEnrollmentStatus)]
pub(crate) enum SequenceEnrollmentStatusSchema {
    Active,
    Paused,
    Completed,
    Exited,
}

impl From<EnrollmentStatus> for SequenceEnrollmentStatusSchema {
    fn from(value: EnrollmentStatus) -> Self {
        match value {
            EnrollmentStatus::Active => Self::Active,
            EnrollmentStatus::Paused => Self::Paused,
            EnrollmentStatus::Completed => Self::Completed,
            EnrollmentStatus::Exited => Self::Exited,
        }
    }
}

impl From<SequenceEnrollmentStatusSchema> for EnrollmentStatus {
    fn from(value: SequenceEnrollmentStatusSchema) -> Self {
        match value {
            SequenceEnrollmentStatusSchema::Active => Self::Active,
            SequenceEnrollmentStatusSchema::Paused => Self::Paused,
            SequenceEnrollmentStatusSchema::Completed => Self::Completed,
            SequenceEnrollmentStatusSchema::Exited => Self::Exited,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SequenceExitReason)]
pub(crate) enum SequenceExitReasonSchema {
    Replied,
    MeetingBooked,
    DealWon,
    DealLost,
    Bounced,
    Objection,
    ContactDeleted,
    Removed,
    ContactUnavailable,
    EnrollerUnavailable,
    SendFailed,
}

impl From<ExitReason> for SequenceExitReasonSchema {
    fn from(value: ExitReason) -> Self {
        match value {
            ExitReason::Replied => Self::Replied,
            ExitReason::MeetingBooked => Self::MeetingBooked,
            ExitReason::DealWon => Self::DealWon,
            ExitReason::DealLost => Self::DealLost,
            ExitReason::Bounced => Self::Bounced,
            ExitReason::Objection => Self::Objection,
            ExitReason::ContactDeleted => Self::ContactDeleted,
            ExitReason::Removed => Self::Removed,
            ExitReason::ContactUnavailable => Self::ContactUnavailable,
            ExitReason::EnrollerUnavailable => Self::EnrollerUnavailable,
            ExitReason::SendFailed => Self::SendFailed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SequenceEnrollmentSource)]
pub(crate) enum SequenceEnrollmentSourceSchema {
    Manual,
    Bulk,
    Automation,
}

impl From<EnrollmentSource> for SequenceEnrollmentSourceSchema {
    fn from(value: EnrollmentSource) -> Self {
        match value {
            EnrollmentSource::Manual => Self::Manual,
            EnrollmentSource::Bulk => Self::Bulk,
            EnrollmentSource::Automation => Self::Automation,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SequenceListParams {
    pub(crate) active: Option<bool>,
    #[param(example = "prospect")]
    pub(crate) search: Option<String>,
}

impl SequenceListParams {
    pub(crate) fn into_query(self) -> SequenceListQuery {
        SequenceListQuery {
            active: self.active,
            search: self.search,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SequenceStepRequest {
    #[schema(example = "0192f0c4-8c1e-7d2a-9f3b-1c2d3e4f5a6b")]
    pub(crate) id: Option<String>,
    pub(crate) kind: SequenceStepKindSchema,
    #[schema(minimum = 1, maximum = 365, example = 2)]
    pub(crate) wait_days: Option<u32>,
    pub(crate) template_id: Option<String>,
    #[schema(example = "Hi {{contact.first_name}}")]
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    #[schema(example = "Call to present the proposal")]
    pub(crate) title: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SequenceRequest {
    #[schema(example = "outbound_prospecting")]
    pub(crate) key: Option<String>,
    #[schema(example = "Outbound prospecting")]
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    #[schema(example = "08:00")]
    pub(crate) window_start: Option<String>,
    #[schema(example = "18:00")]
    pub(crate) window_end: Option<String>,
    #[schema(minimum = 1, maximum = 2000, example = 50)]
    pub(crate) daily_limit: Option<u32>,
    pub(crate) steps: Option<Vec<SequenceStepRequest>>,
}

impl SequenceRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveSequenceCommand {
        SaveSequenceCommand {
            key: self.key,
            name: self.name,
            description: self.description,
            is_active: self.is_active,
            window_start: self.window_start,
            window_end: self.window_end,
            daily_limit: self.daily_limit,
            steps: self.steps.map(|steps| {
                steps
                    .into_iter()
                    .map(|step| StepInput {
                        id: step.id,
                        kind: StepKind::from(step.kind).as_str().to_owned(),
                        wait_days: step.wait_days,
                        template_id: step.template_id,
                        subject: step.subject,
                        body_html: step.body_html,
                        title: step.title,
                    })
                    .collect()
            }),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceStepResponse {
    pub(crate) id: String,
    pub(crate) position: u32,
    pub(crate) kind: SequenceStepKindSchema,
    pub(crate) wait_days: u32,
    pub(crate) template_id: Option<String>,
    pub(crate) subject: String,
    pub(crate) body_html: Option<String>,
    pub(crate) title: String,
}

impl From<SequenceStep> for SequenceStepResponse {
    fn from(step: SequenceStep) -> Self {
        Self {
            id: step.id.to_string(),
            position: step.position,
            kind: step.kind.into(),
            wait_days: step.wait_days,
            template_id: step.template_id.map(|id| id.to_string()),
            subject: step.subject,
            body_html: step.body_html,
            title: step.title,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceEnrollmentCountsResponse {
    pub(crate) active: u64,
    pub(crate) paused: u64,
    pub(crate) completed: u64,
    pub(crate) exited: u64,
}

impl From<EnrollmentCounts> for SequenceEnrollmentCountsResponse {
    fn from(counts: EnrollmentCounts) -> Self {
        Self {
            active: counts.active,
            paused: counts.paused,
            completed: counts.completed,
            exited: counts.exited,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_active: bool,
    #[schema(example = "08:00")]
    pub(crate) window_start: String,
    #[schema(example = "18:00")]
    pub(crate) window_end: String,
    pub(crate) daily_limit: u32,
    pub(crate) steps: Vec<SequenceStepResponse>,
    pub(crate) enrollments: SequenceEnrollmentCountsResponse,
    pub(crate) created_by: Option<String>,
    pub(crate) updated_by: Option<String>,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<SequenceSummary> for SequenceResponse {
    fn from(summary: SequenceSummary) -> Self {
        let sequence = summary.sequence;
        Self {
            id: sequence.id.to_string(),
            key: sequence.key,
            name: sequence.name,
            description: sequence.description,
            is_active: sequence.is_active,
            window_start: format_clock(sequence.window.start_minute),
            window_end: format_clock(sequence.window.end_minute),
            daily_limit: sequence.daily_limit,
            steps: sequence.steps.into_iter().map(Into::into).collect(),
            enrollments: summary.counts.into(),
            created_by: sequence.created_by.map(UserId::into_inner),
            updated_by: sequence.updated_by.map(UserId::into_inner),
            created_at: timestamp(sequence.created_at),
            updated_at: timestamp(sequence.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceStepMetricsResponse {
    pub(crate) position: u32,
    pub(crate) step_id: Option<String>,
    pub(crate) kind: Option<SequenceStepKindSchema>,
    pub(crate) executed: u64,
    pub(crate) sent: u64,
    pub(crate) opened: u64,
    pub(crate) replied: u64,
    pub(crate) bounced: u64,
    pub(crate) failed: u64,
    pub(crate) exits: u64,
}

impl From<StepMetrics> for SequenceStepMetricsResponse {
    fn from(metrics: StepMetrics) -> Self {
        Self {
            position: metrics.position,
            step_id: metrics.step_id.map(|id| id.to_string()),
            kind: metrics.kind.map(Into::into),
            executed: metrics.executed,
            sent: metrics.sent,
            opened: metrics.opened,
            replied: metrics.replied,
            bounced: metrics.bounced,
            failed: metrics.failed,
            exits: metrics.exits,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceMetricsResponse {
    pub(crate) enrollments: SequenceEnrollmentCountsResponse,
    pub(crate) steps: Vec<SequenceStepMetricsResponse>,
}

impl From<SequenceMetrics> for SequenceMetricsResponse {
    fn from(metrics: SequenceMetrics) -> Self {
        Self {
            enrollments: metrics.counts.into(),
            steps: metrics.steps.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnrollContactRequest {
    pub(crate) contact_id: String,
    pub(crate) mailbox_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SequenceEnrollmentListParams {
    pub(crate) sequence_id: Option<String>,
    pub(crate) contact_id: Option<String>,
    pub(crate) status: Option<SequenceEnrollmentStatusSchema>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SequenceEnrollmentStatusParams {
    pub(crate) status: Option<SequenceEnrollmentStatusSchema>,
}

pub(crate) fn parse_uuid(field: &str, value: Option<&str>) -> Result<Option<uuid::Uuid>, String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => uuid::Uuid::parse_str(value)
            .map(Some)
            .map_err(|_| format!("{field} must be a UUID")),
    }
}

impl SequenceEnrollmentListParams {
    pub(crate) fn into_filter(self) -> Result<EnrollmentFilter, String> {
        Ok(EnrollmentFilter {
            id: None,
            sequence_id: parse_uuid("sequence_id", self.sequence_id.as_deref())?,
            contact_id: parse_uuid("contact_id", self.contact_id.as_deref())?,
            status: self.status.map(Into::into),
        })
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SequenceEnrollmentResponse {
    pub(crate) id: String,
    pub(crate) sequence_id: String,
    pub(crate) sequence_name: String,
    pub(crate) contact_id: String,
    pub(crate) contact_name: String,
    pub(crate) contact_email: Option<String>,
    pub(crate) enrolled_by: String,
    pub(crate) enrolled_by_name: Option<String>,
    pub(crate) mailbox_id: Option<String>,
    pub(crate) source: SequenceEnrollmentSourceSchema,
    pub(crate) status: SequenceEnrollmentStatusSchema,
    pub(crate) current_position: u32,
    pub(crate) next_run_at: Option<String>,
    pub(crate) exit_reason: Option<SequenceExitReasonSchema>,
    pub(crate) exit_position: Option<u32>,
    pub(crate) paused_at: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) exited_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<EnrollmentView> for SequenceEnrollmentResponse {
    fn from(enrollment: EnrollmentView) -> Self {
        Self {
            id: enrollment.id.to_string(),
            sequence_id: enrollment.sequence_id.to_string(),
            sequence_name: enrollment.sequence_name,
            contact_id: enrollment.contact_id.to_string(),
            contact_name: enrollment.contact_name,
            contact_email: enrollment.contact_email,
            enrolled_by: enrollment.enrolled_by.into_inner(),
            enrolled_by_name: enrollment.enrolled_by_name,
            mailbox_id: enrollment.mailbox_id.map(|id| id.to_string()),
            source: enrollment.source.into(),
            status: enrollment.status.into(),
            current_position: enrollment.current_position,
            next_run_at: enrollment.next_run_at.map(timestamp),
            exit_reason: enrollment.exit_reason.map(Into::into),
            exit_position: enrollment.exit_position,
            paused_at: enrollment.paused_at.map(timestamp),
            completed_at: enrollment.completed_at.map(timestamp),
            exited_at: enrollment.exited_at.map(timestamp),
            created_at: timestamp(enrollment.created_at),
            updated_at: timestamp(enrollment.updated_at),
        }
    }
}
