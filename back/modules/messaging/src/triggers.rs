use std::sync::Arc;

use crate::{MESSAGE_RECEIVED, MESSAGE_SENT};
use aphel_contract_automation::application::AutomationTrigger;
use aphel_contract_automation::application::triggers::{TriggerCandidate, event_context};
use aphel_sdk::platform::application::shared::DomainEvent;

struct MessagingTrigger {
    event_type: &'static str,
    label: &'static str,
    description: &'static str,
}

impl AutomationTrigger for MessagingTrigger {
    fn key(&self) -> &'static str {
        self.event_type
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn supports(&self, record_type: &str) -> bool {
        matches!(record_type, "people.person" | "deals.deal")
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.event_type
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        event
            .record
            .iter()
            .filter(|record| self.supports(record.record_type.as_str()))
            .map(|record| TriggerCandidate {
                record: record.clone(),
                context: event_context(self.event_type, event),
            })
            .collect()
    }
}

fn trigger(
    event_type: &'static str,
    label: &'static str,
    description: &'static str,
) -> Arc<dyn AutomationTrigger> {
    Arc::new(MessagingTrigger {
        event_type,
        label,
        description,
    })
}

pub(crate) fn messaging_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![
        trigger(
            MESSAGE_RECEIVED,
            "Mensagem recebida",
            "Quando chega uma mensagem por WhatsApp ou e-mail de um contato (ou de uma conversa vinculada à oportunidade)",
        ),
        trigger(
            MESSAGE_SENT,
            "Mensagem enviada",
            "Quando alguém responde uma conversa pela caixa de entrada",
        ),
    ]
}
