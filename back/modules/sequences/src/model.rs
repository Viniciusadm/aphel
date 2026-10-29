use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    EnrollmentSource, EnrollmentStatus, ExitReason, RunStatus, SendWindow, StepKind,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;

pub(crate) const SEQUENCE_RECORD_TYPE: RecordType = RecordType::from_static("sequence");
pub(crate) const SEQUENCE_ENROLLMENT_RECORD_TYPE: RecordType =
    RecordType::from_static("sequence_enrollment");

pub(crate) const SEQUENCE_CREATED: &str = "sequence.created";
pub(crate) const SEQUENCE_UPDATED: &str = "sequence.updated";
pub(crate) const SEQUENCE_DELETED: &str = "sequence.deleted";
pub(crate) const SEQUENCE_STEP_EXECUTED: &str = "sequence.step_executed";
pub(crate) const SEQUENCE_ENROLLMENT_PAUSED: &str = "sequence_enrollment.paused";
pub(crate) const SEQUENCE_ENROLLMENT_RESUMED: &str = "sequence_enrollment.resumed";

pub(crate) const SEQUENCE_NOT_FOUND: &str = "sequence_not_found";
pub(crate) const SEQUENCE_KEY_TAKEN: &str = "sequence_key_taken";
pub(crate) const SEQUENCE_INACTIVE: &str = "sequence_inactive";
pub(crate) const SEQUENCE_EMPTY: &str = "sequence_empty";
pub(crate) const SEQUENCE_ALREADY_ENROLLED: &str = "sequence_already_enrolled";
pub(crate) const SEQUENCE_ENROLLMENT_NOT_FOUND: &str = "sequence_enrollment_not_found";
pub(crate) const SEQUENCE_ENROLLMENT_FINISHED: &str = "sequence_enrollment_finished";
pub(crate) const SEQUENCE_CONTACT_WITHOUT_EMAIL: &str = "sequence_contact_without_email";
pub(crate) const SEQUENCE_CONTACT_OBJECTED: &str = "sequence_contact_objected";
pub(crate) const NO_MAILBOX_AVAILABLE: &str = "no_mailbox_available";

pub(crate) const MAX_SUBJECT_LENGTH: usize = 998;
pub(crate) const MAX_BODY_BYTES: usize = 512 * 1024;
pub(crate) const MAX_TITLE_LENGTH: usize = 200;
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 1000;
pub(crate) const MAX_FAILURE_REASON_LENGTH: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStep {
    pub(crate) id: Uuid,
    pub(crate) position: u32,
    pub(crate) kind: StepKind,
    pub(crate) wait_days: u32,
    pub(crate) template_id: Option<Uuid>,
    pub(crate) subject: String,
    pub(crate) body_html: Option<String>,
    pub(crate) title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceView {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) window: SendWindow,
    pub(crate) daily_limit: u32,
    pub(crate) steps: Vec<SequenceStep>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl SequenceView {
    pub(crate) fn has_auto_email(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.kind == StepKind::AutoEmail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EnrollmentCounts {
    pub(crate) active: u64,
    pub(crate) paused: u64,
    pub(crate) completed: u64,
    pub(crate) exited: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceSummary {
    pub(crate) sequence: SequenceView,
    pub(crate) counts: EnrollmentCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepInput {
    pub(crate) id: Option<String>,
    pub(crate) kind: String,
    pub(crate) wait_days: Option<u32>,
    pub(crate) template_id: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveSequenceCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) window_start: Option<String>,
    pub(crate) window_end: Option<String>,
    pub(crate) daily_limit: Option<u32>,
    pub(crate) steps: Option<Vec<StepInput>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SequenceListQuery {
    pub(crate) active: Option<bool>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentView {
    pub(crate) id: Uuid,
    pub(crate) sequence_id: Uuid,
    pub(crate) sequence_key: String,
    pub(crate) sequence_name: String,
    pub(crate) contact_id: Uuid,
    pub(crate) contact_name: String,
    pub(crate) contact_email: Option<String>,
    pub(crate) enrolled_by: UserId,
    pub(crate) enrolled_by_name: Option<String>,
    pub(crate) mailbox_id: Option<Uuid>,
    pub(crate) source: EnrollmentSource,
    pub(crate) status: EnrollmentStatus,
    pub(crate) current_position: u32,
    pub(crate) next_run_at: Option<DateTime<Utc>>,
    pub(crate) last_email_id: Option<Uuid>,
    pub(crate) exit_reason: Option<ExitReason>,
    pub(crate) exit_position: Option<u32>,
    pub(crate) paused_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) exited_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewEnrollment {
    pub(crate) id: Uuid,
    pub(crate) sequence_id: Uuid,
    pub(crate) contact_id: Uuid,
    pub(crate) enrolled_by: UserId,
    pub(crate) mailbox_id: Option<Uuid>,
    pub(crate) source: EnrollmentSource,
    pub(crate) next_run_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EnrollmentFilter {
    pub(crate) id: Option<Uuid>,
    pub(crate) sequence_id: Option<Uuid>,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) status: Option<EnrollmentStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollCommand {
    pub sequence_id: String,
    pub contact_id: String,
    pub mailbox_id: Option<String>,
    pub source: EnrollmentSource,
    pub enroll_as: Option<UserId>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepRun {
    pub(crate) id: Uuid,
    pub(crate) sequence_id: Uuid,
    pub(crate) enrollment_id: Uuid,
    pub(crate) step_id: Option<Uuid>,
    pub(crate) position: u32,
    pub(crate) kind: StepKind,
    pub(crate) status: RunStatus,
    pub(crate) contact_id: Uuid,
    pub(crate) mailbox_id: Option<Uuid>,
    pub(crate) email_id: Option<Uuid>,
    pub(crate) thread_id: Option<Uuid>,
    pub(crate) activity_id: Option<Uuid>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StepMetrics {
    pub(crate) position: u32,
    pub(crate) step_id: Option<Uuid>,
    pub(crate) kind: Option<StepKind>,
    pub(crate) executed: u64,
    pub(crate) sent: u64,
    pub(crate) opened: u64,
    pub(crate) replied: u64,
    pub(crate) bounced: u64,
    pub(crate) failed: u64,
    pub(crate) exits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceMetrics {
    pub(crate) counts: EnrollmentCounts,
    pub(crate) steps: Vec<StepMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueEnrollment {
    pub(crate) enrollment_id: Uuid,
    pub(crate) position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContactTarget {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceEmail {
    pub(crate) contact_id: Uuid,
    pub(crate) to: String,
    pub(crate) mailbox_id: Option<Uuid>,
    pub(crate) reply_to_email_id: Option<Uuid>,
    pub(crate) template_id: Option<Uuid>,
    pub(crate) subject: String,
    pub(crate) body_html: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SentSequenceEmail {
    pub(crate) email_id: Uuid,
    pub(crate) thread_id: Uuid,
    pub(crate) mailbox_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedDraft {
    pub(crate) subject: String,
    pub(crate) body_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceTask {
    pub(crate) activity_type_key: &'static str,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) owner_id: UserId,
    pub(crate) contact_id: Uuid,
    pub(crate) due_at: DateTime<Utc>,
}

pub(crate) fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
