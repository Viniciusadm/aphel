use std::sync::Arc;

use crate::SUBMITTED;
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::triggers::{TriggerCandidate, event_context};
use aphel_kernel::application::shared::DomainEvent;

struct SubmittedTrigger;

impl AutomationTrigger for SubmittedTrigger {
    fn key(&self) -> &'static str {
        SUBMITTED
    }

    fn label(&self) -> &'static str {
        "Formulário de captura enviado"
    }

    fn description(&self) -> &'static str {
        "Quando um visitante envia um formulário de captura (o registro é o contato)"
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == "contacts.contact"
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == SUBMITTED
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| self.supports(record.record_type.as_str()))
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(SUBMITTED, event),
            })
            .collect()
    }
}

pub(crate) fn submitted_trigger() -> Arc<dyn AutomationTrigger> {
    Arc::new(SubmittedTrigger)
}
