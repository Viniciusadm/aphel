use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use crate::{MEETING_BOOKED, MEETING_CANCELED, MEETING_RESCHEDULED};
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::triggers::{TriggerCandidate, event_context};
use aphel_kernel::application::shared::{DomainEvent, RecordType};

struct MeetingTrigger {
    key: &'static str,
    label: &'static str,
    description: &'static str,
}

impl AutomationTrigger for MeetingTrigger {
    fn key(&self) -> &'static str {
        self.key
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn supports(&self, record_type: &str) -> bool {
        record_type == RecordType::CONTACT.as_str()
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.key
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| record.record_type == RecordType::CONTACT)
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(self.key, event),
            })
            .collect()
    }
}

pub(crate) fn meeting_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![
        Arc::new(MeetingTrigger {
            key: MEETING_BOOKED,
            label: "Reunião agendada",
            description: "A contact booked a meeting through a scheduling link",
        }),
        Arc::new(MeetingTrigger {
            key: MEETING_RESCHEDULED,
            label: "Reunião remarcada",
            description: "A meeting booked through a scheduling link was rescheduled",
        }),
        Arc::new(MeetingTrigger {
            key: MEETING_CANCELED,
            label: "Reunião cancelada",
            description: "A meeting booked through a scheduling link was canceled",
        }),
    ]
}
