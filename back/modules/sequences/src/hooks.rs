use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::StepKind;
use aphel_kernel::domain::UserId;
use aphel_kernel::modules::{Hook, PluginHookArgs};

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

pub(crate) const STEP_RUNNING: &str = "sequence.step_running";

impl PluginHookArgs for RunningStep {
    fn plugin_record_id(&self) -> String {
        self.contact_id.to_string()
    }

    fn plugin_payload(&self) -> Value {
        json!({
            "enrollment_id": self.enrollment_id,
            "sequence_id": self.sequence_id,
            "contact_id": self.contact_id,
            "enrolled_by": self.enrolled_by.as_str(),
            "position": self.position,
            "kind": self.kind.as_str(),
            "template_id": self.template_id,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StepRunning;

impl Hook for StepRunning {
    type Args = RunningStep;
}
