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
pub(crate) struct NotesFollowedEvents;
#[async_trait]
impl FollowedEventSource for NotesFollowedEvents {
    fn key(&self) -> &str {
        "notes"
    }
    fn accepts(&self, event_type: &str) -> bool {
        event_type == "note.created"
    }
    fn signal(&self, event: &DomainEvent) -> Option<FollowedEvent> {
        if event.payload.get("visibility").and_then(Value::as_str) == Some("private") {
            return None;
        }
        Some(FollowedEvent {
            targets: parent(&event.payload).into_iter().collect(),
            required_records: Vec::new(),
            headline: labels(
                "{{actor}} added a note to a record you follow",
                "{{actor}} adicionou uma nota a um registro que você segue",
            ),
            body: None,
            body_field: None,
            changes: false,
        })
    }
    async fn visible(
        &self,
        pool: &sqlx::MySqlPool,
        recipient: &aphel_kernel::domain::UserId,
        event: &DomainEvent,
    ) -> Result<bool, aphel_sdk::AppError> {
        use aphel_contract_crm::CrmPermissions;
        use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
        let repository =
            aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository::new(
                pool.clone(),
            );
        let Some(access) = repository.load_access(recipient).await? else {
            return Ok(false);
        };
        let scope = build_access_scope(recipient.clone(), access);
        if scope
            .filter_for(&aphel_sdk::platform::domain::access::Permission::NOTES_READ)
            .is_none()
        {
            return Ok(false);
        }
        let Some(record) = &event.record else {
            return Ok(false);
        };
        let visible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM notes WHERE id = ? AND deleted_at IS NULL AND visibility = 'team')").bind(record.record_id.to_string()).fetch_one(pool).await?;
        Ok(visible)
    }
}
