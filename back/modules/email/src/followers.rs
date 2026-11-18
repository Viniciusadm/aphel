use aphel_sdk::fields::LocalizedLabels;
use aphel_sdk::followers::{FollowedEvent, FollowedEventSource};
use aphel_sdk::platform::database::Db;
use aphel_sdk::shared::DomainEvent;
use async_trait::async_trait;
use serde_json::Value;
fn labels(en: &str, pt: &str) -> LocalizedLabels {
    LocalizedLabels(std::collections::BTreeMap::from([
        ("en".into(), en.into()),
        ("pt-BR".into(), pt.into()),
    ]))
}
pub(crate) struct EmailFollowedEvents;
#[async_trait]
impl FollowedEventSource for EmailFollowedEvents {
    fn key(&self) -> &str {
        "email"
    }
    fn accepts(&self, event_type: &str) -> bool {
        matches!(event_type, "email.received" | "email.sent")
    }
    fn signal(&self, event: &DomainEvent) -> Option<FollowedEvent> {
        if event.payload.get("visibility").and_then(Value::as_str) == Some("private") {
            return None;
        }
        Some(FollowedEvent {
            targets: event.record.clone().into_iter().collect(),
            required_records: Vec::new(),
            headline: labels(
                "A record you follow has a new email",
                "Um registro que você segue tem um novo e-mail",
            ),
            body: None,
            body_field: None,
            changes: false,
        })
    }
    async fn visible(
        &self,
        pool: &Db,
        recipient: &aphel_kernel::domain::UserId,
        event: &DomainEvent,
    ) -> Result<bool, aphel_sdk::AppError> {
        use aphel_contract_crm::CrmPermissions;
        use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
        let access =
            aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository::new(
                pool.clone(),
            );
        let Some(access) = access.load_access(recipient).await? else {
            return Ok(false);
        };
        let scope = build_access_scope(recipient.clone(), access);
        if scope
            .filter_for(&aphel_sdk::platform::domain::access::Permission::EMAILS_READ)
            .is_none()
        {
            return Ok(false);
        }
        let Some(thread) = event.payload.get("thread_id").and_then(Value::as_str) else {
            return Ok(false);
        };
        let visible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id WHERE t.id = ? AND (m.user_id = ? OR COALESCE(t.visibility,m.default_visibility) <> 'private'))")
            .bind(thread).bind(recipient.as_str()).fetch_one(pool).await?;
        Ok(visible)
    }
}
