use uuid::Uuid;

use crate::domain::StepKind;
use aphel_kernel::domain::UserId;
use aphel_kernel::modules::Hook;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunningStep {
    pub(crate) enrollment_id: Uuid,
    pub(crate) sequence_id: Uuid,
    pub(crate) contact_id: Uuid,
    pub(crate) enrolled_by: UserId,
    pub(crate) position: u32,
    pub(crate) kind: StepKind,
    pub(crate) template_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StepRunning;

impl Hook for StepRunning {
    type Args = RunningStep;
}
