use std::sync::Arc;

use crate::{CANCELLED, COMPLETED, DEAL, EXPIRED, REFUSED, SENT, SIGNED};
use aphel_kernel::application::automations::AutomationTrigger;
use aphel_kernel::application::automations::triggers::{TriggerCandidate, event_context};
use aphel_kernel::application::shared::DomainEvent;

struct EsignTrigger {
    event_type: &'static str,
    label: &'static str,
    description: &'static str,
}

impl AutomationTrigger for EsignTrigger {
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
        record_type == DEAL
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
    Arc::new(EsignTrigger {
        event_type,
        label,
        description,
    })
}

pub(crate) fn esign_triggers() -> Vec<Arc<dyn AutomationTrigger>> {
    vec![
        trigger(
            SENT,
            "Documento enviado para assinatura",
            "Quando um PDF da oportunidade é enviado para assinatura eletrônica",
        ),
        trigger(
            SIGNED,
            "Documento assinado por um signatário",
            "Quando um dos signatários assina o documento",
        ),
        trigger(
            REFUSED,
            "Assinatura recusada",
            "Quando um signatário recusa o documento",
        ),
        trigger(
            CANCELLED,
            "Assinatura cancelada",
            "Quando a solicitação de assinatura é cancelada",
        ),
        trigger(
            COMPLETED,
            "Assinatura concluída",
            "Quando todos os signatários assinam e o documento é finalizado",
        ),
        trigger(
            EXPIRED,
            "Assinatura expirada",
            "Quando o prazo da solicitação termina sem todas as assinaturas",
        ),
    ]
}
