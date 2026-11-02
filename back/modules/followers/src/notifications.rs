use ::notifications::application::NotificationView;
use ::notifications::application::texts::{NotificationTexts, data_text};
use aphel_kernel::application::mail::MailLocale;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Map, Value, json};

use ::notifications::application::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe,
};
use ::notifications::domain::NotificationKind;
use aphel_kernel::api::routes::v1::field_permissions::user_field_redaction;
use aphel_kernel::application::fields::{CUSTOM_FIELDS_PREFIX, FieldDefinitionRepository};
use aphel_kernel::application::shared::{DomainEvent, REDACTED, RecordRef};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::fields::FieldDefinition;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::state::runtime_pool;

const ACTIVITY_CREATED: &str = "activity.created";
const NOTE_CREATED: &str = "note.created";
const EMAIL_RECEIVED: &str = "email.received";
const EMAIL_SENT: &str = "email.sent";
const DEAL_STAGE_CHANGED: &str = "deal.stage_changed";
const UPDATED_SUFFIX: &str = ".updated";
const PRIVATE_VISIBILITY: &str = "private";
const MAX_BODY_CHANGES: usize = 3;
const MAX_VALUE_LENGTH: usize = 80;
const EMPTY_VALUE: &str = "—";

pub(crate) const FOLLOWERS_SOURCE_ORDER: i32 = 100;

const FOLLOWED_EVENTS: [&str; 5] = [
    ACTIVITY_CREATED,
    NOTE_CREATED,
    EMAIL_RECEIVED,
    EMAIL_SENT,
    DEAL_STAGE_CHANGED,
];

pub(crate) struct FollowedRecordNotifications;

fn text(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn parent_record(payload: &Value) -> Option<RecordRef> {
    let record_type = payload.get("record_type").and_then(Value::as_str)?;
    let record_id = payload.get("record_id").and_then(Value::as_str)?;
    RecordRef::parse(record_type, record_id).ok()
}

fn is_private(payload: &Value) -> bool {
    payload.get("visibility").and_then(Value::as_str) == Some(PRIVATE_VISIBILITY)
}

fn followed_targets(event: &DomainEvent) -> (Vec<RecordRef>, &'static str) {
    let payload = &event.payload;
    match event.event_type.as_str() {
        ACTIVITY_CREATED => {
            let mut records: Vec<RecordRef> = parent_record(payload).into_iter().collect();
            for link in payload
                .get("links")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(record) = parent_record(link)
                    && !records.contains(&record)
                {
                    records.push(record);
                }
            }
            (records, "activity")
        }
        NOTE_CREATED => {
            if is_private(payload) {
                return (Vec::new(), "note");
            }
            (parent_record(payload).into_iter().collect(), "note")
        }
        EMAIL_RECEIVED | EMAIL_SENT => {
            if is_private(payload) {
                return (Vec::new(), "email");
            }
            (event.record.clone().into_iter().collect(), "email")
        }
        event_type => {
            let record = event.record.clone().filter(|record| {
                event_type == DEAL_STAGE_CHANGED
                    || event_type
                        .strip_suffix(UPDATED_SUFFIX)
                        .and_then(|key| {
                            aphel_kernel::application::shared::RecordType::parse(key).ok()
                        })
                        .is_some_and(|key| key == record.record_type)
            });
            (record.into_iter().collect(), "update")
        }
    }
}

struct Change {
    field: String,
    old: Value,
    new: Value,
}

fn payload_changes(payload: &Value) -> Vec<Change> {
    payload
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|change| {
            Some(Change {
                field: change.get("field").and_then(Value::as_str)?.to_owned(),
                old: change.get("old").cloned().unwrap_or(Value::Null),
                new: change.get("new").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn definition_key(field: &str) -> &str {
    let field = field
        .strip_prefix(CUSTOM_FIELDS_PREFIX)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(field);
    field.split('.').next().unwrap_or(field)
}

fn truncated(value: &str) -> String {
    if value.chars().count() <= MAX_VALUE_LENGTH {
        return value.to_owned();
    }
    let mut short: String = value.chars().take(MAX_VALUE_LENGTH).collect();
    short.push('…');
    short
}

fn display_value(value: &Value, definition: Option<&FieldDefinition>) -> String {
    match value {
        Value::Null => EMPTY_VALUE.to_owned(),
        Value::String(text) if text.is_empty() => EMPTY_VALUE.to_owned(),
        Value::String(text) => definition
            .and_then(|definition| definition.option_by_value(text))
            .map_or_else(|| truncated(text), |option| truncated(&option.label)),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(items) => {
            let items: Vec<String> = items
                .iter()
                .map(|item| display_value(item, definition))
                .collect();
            truncated(&items.join(", "))
        }
        Value::Object(object) => ["name", "title", "display_name", "label"]
            .into_iter()
            .find_map(|key| object.get(key).and_then(Value::as_str))
            .map_or_else(
                || truncated(&Value::Object(object.clone()).to_string()),
                truncated,
            ),
    }
}

struct ChangeDescriptions {
    definitions: HashMap<String, FieldDefinition>,
}

impl ChangeDescriptions {
    async fn load(record: &RecordRef) -> Result<Self, AppError> {
        let pool = runtime_pool()?;
        let definitions = MySqlFieldDefinitionRepository::new(pool)
            .list(&record.record_type)
            .await?
            .into_iter()
            .map(|definition| (definition.key.as_str().to_owned(), definition))
            .collect();
        Ok(Self { definitions })
    }

    fn definition(&self, field: &str) -> Option<&FieldDefinition> {
        self.definitions.get(definition_key(field))
    }

    fn describe(&self, change: &Change) -> (Value, String) {
        let definition = self.definition(&change.field);
        let label = definition.map_or_else(
            || change.field.clone(),
            |definition| definition.label.clone(),
        );
        let sensitive = definition.is_some_and(|definition| definition.is_sensitive);
        let (old, new) = if sensitive {
            (json!(REDACTED), json!(REDACTED))
        } else {
            (change.old.clone(), change.new.clone())
        };
        let line = if sensitive {
            label.clone()
        } else {
            format!(
                "{label}: {} → {}",
                display_value(&old, definition),
                display_value(&new, definition)
            )
        };
        (
            json!({ "field": change.field, "label": label, "old": old, "new": new }),
            line,
        )
    }
}

async fn visible_changes(
    record: &RecordRef,
    recipient: &UserId,
    changes: &[Change],
    descriptions: &ChangeDescriptions,
) -> Result<(Vec<Value>, Option<String>), AppError> {
    if changes.is_empty() {
        return Ok((Vec::new(), None));
    }
    let pool = runtime_pool()?;
    let redaction = user_field_redaction(&pool, Some(recipient), &record.record_type).await?;
    let mut items = Vec::new();
    let mut lines = Vec::new();
    for change in changes
        .iter()
        .filter(|change| !redaction.hides_path(&change.field))
    {
        let (item, line) = descriptions.describe(change);
        items.push(item);
        lines.push(line);
    }
    if lines.is_empty() {
        return Ok((items, None));
    }
    let hidden = lines.len().saturating_sub(MAX_BODY_CHANGES);
    let mut body: Vec<String> = lines.into_iter().take(MAX_BODY_CHANGES).collect();
    if hidden > 0 {
        body.push(format!("+{hidden}"));
    }
    Ok((items, Some(body.join("\n"))))
}

#[async_trait]
impl NotificationSource for FollowedRecordNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        FOLLOWED_EVENTS.contains(&event_type) || event_type.ends_with(UPDATED_SUFFIX)
    }

    fn order(&self) -> i32 {
        FOLLOWERS_SOURCE_ORDER
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let (records, activity) = followed_targets(event);
        let kind = FOLLOWED_RECORD_ACTIVITY_NOTIFICATION;
        let mut seen: Vec<UserId> = context.notified.to_vec();
        seen.extend(context.actor.cloned());
        let changes = if activity == "update" {
            payload_changes(&event.payload)
        } else {
            Vec::new()
        };
        let mut intents = Vec::new();
        for record in records {
            let followers: Vec<UserId> = context
                .directory
                .followers(&record)
                .await?
                .into_iter()
                .filter(|follower| !seen.contains(follower))
                .collect();
            if followers.is_empty() {
                continue;
            }
            let title = context
                .directory
                .record_summary(&record)
                .await?
                .map(|summary| summary.title)
                .or_else(|| {
                    (activity == "update")
                        .then(|| {
                            ["title", "name", "full_name"]
                                .into_iter()
                                .find_map(|key| text(&event.payload, key))
                        })
                        .flatten()
                })
                .unwrap_or_default();
            let descriptions = if changes.is_empty() {
                None
            } else {
                Some(ChangeDescriptions::load(&record).await?)
            };
            for follower in followers {
                if seen.contains(&follower) {
                    continue;
                }
                seen.push(follower.clone());
                if !context.directory.can_read(&follower, &record).await? {
                    continue;
                }
                let (items, change_body) = match &descriptions {
                    Some(descriptions) => {
                        visible_changes(&record, &follower, &changes, descriptions).await?
                    }
                    None => (Vec::new(), None),
                };
                if !changes.is_empty() && items.is_empty() {
                    continue;
                }
                let body = match activity {
                    "activity" | "note" => text(&event.payload, "title"),
                    _ => change_body,
                };
                let mut data = Map::new();
                data.insert("activity".to_owned(), json!(activity));
                data.insert("event_type".to_owned(), json!(event.event_type));
                data.insert("record_type".to_owned(), json!(record.record_type.as_str()));
                data.insert("record_id".to_owned(), json!(record.record_id.to_string()));
                if !items.is_empty() {
                    data.insert("changes".to_owned(), Value::Array(items));
                }
                intents.push(
                    NotificationIntent::new(
                        follower,
                        kind,
                        title.clone(),
                        event_dedupe(event, kind),
                    )
                    .with_body(body)
                    .with_record(Some(record.clone()))
                    .with_data(Value::Object(data)),
                );
            }
        }
        Ok(intents)
    }
}

pub(crate) const FOLLOWED_RECORD_ACTIVITY_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("followed_record_activity");

pub(crate) struct FollowerNotificationTexts;

impl NotificationTexts for FollowerNotificationTexts {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String> {
        if view.kind != FOLLOWED_RECORD_ACTIVITY_NOTIFICATION {
            return None;
        }
        let en = locale == MailLocale::En;
        Some(match data_text(view, "activity") {
            Some("activity") if en => format!("{actor} added an activity to a record you follow"),
            Some("activity") => {
                format!("{actor} adicionou uma atividade a um registro que você segue")
            }
            Some("note") if en => format!("{actor} added a note to a record you follow"),
            Some("note") => format!("{actor} adicionou uma nota a um registro que você segue"),
            Some("email") if en => "A record you follow has a new email".to_owned(),
            Some("email") => "Um registro que você segue tem um novo e-mail".to_owned(),
            _ if en => format!("{actor} changed a record you follow"),
            _ => format!("{actor} alterou um registro que você segue"),
        })
    }
}
