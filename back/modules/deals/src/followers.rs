use aphel_sdk::fields::LocalizedLabels;
use aphel_sdk::followers::{FollowedEvent, FollowedEventSource};
use aphel_sdk::shared::DomainEvent;
use async_trait::async_trait;
fn labels(en: &str, pt: &str) -> LocalizedLabels {
    LocalizedLabels(std::collections::BTreeMap::from([
        ("en".into(), en.into()),
        ("pt-BR".into(), pt.into()),
    ]))
}
pub(crate) struct DealsFollowedEvents;
#[async_trait]
impl FollowedEventSource for DealsFollowedEvents {
    fn key(&self) -> &str {
        "deals"
    }
    fn accepts(&self, event_type: &str) -> bool {
        event_type == "deal.stage_changed"
    }
    fn signal(&self, event: &DomainEvent) -> Option<FollowedEvent> {
        Some(FollowedEvent {
            targets: event.record.clone().into_iter().collect(),
            required_records: Vec::new(),
            headline: labels(
                "{{actor}} changed a record you follow",
                "{{actor}} alterou um registro que você segue",
            ),
            body: None,
            body_field: None,
            changes: true,
        })
    }
}
