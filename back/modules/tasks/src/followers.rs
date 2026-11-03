use aphel_sdk::fields::LocalizedLabels;
use aphel_sdk::followers::{FollowedEvent, FollowedEventSource};
use aphel_sdk::shared::{DomainEvent, RecordRef};
use async_trait::async_trait;
use serde_json::Value;
fn parent(value: &Value) -> Option<RecordRef> {
    RecordRef::parse(
        value.get("record_type")?.as_str()?,
        value.get("record_id")?.as_str()?,
    )
    .ok()
}
fn labels(en: &str, pt: &str) -> LocalizedLabels {
    LocalizedLabels(std::collections::BTreeMap::from([
        ("en".into(), en.into()),
        ("pt-BR".into(), pt.into()),
    ]))
}
pub(crate) struct TasksFollowedEvents;
#[async_trait]
impl FollowedEventSource for TasksFollowedEvents {
    fn key(&self) -> &str {
        "tasks"
    }
    fn accepts(&self, event_type: &str) -> bool {
        event_type == crate::domain::ACTIVITY_CREATED
    }
    fn signal(&self, event: &DomainEvent) -> Option<FollowedEvent> {
        let mut targets: Vec<_> = parent(&event.payload).into_iter().collect();
        for link in event
            .payload
            .get("links")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(record) = parent(link)
                && !targets.contains(&record)
            {
                targets.push(record);
            }
        }
        let task = event.record.clone()?;
        Some(FollowedEvent {
            targets,
            required_records: vec![task.clone()],
            headline: labels(
                "{{actor}} added a task to a record you follow",
                "{{actor}} adicionou uma tarefa a um registro que você segue",
            ),
            body: event
                .payload
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_owned),
            body_field: Some((task, "title".into())),
            changes: false,
        })
    }
}
