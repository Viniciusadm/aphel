use aphel_sdk::notifications::NotificationTexts;
use aphel_sdk::notifications::NotificationView;
use aphel_sdk::platform::application::mail::MailLocale;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Map, Value, json};

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe,
};
use aphel_sdk::platform::api::routes::v1::field_permissions::user_field_redaction;
use aphel_sdk::platform::application::fields::{CUSTOM_FIELDS_PREFIX, FieldDefinitionRepository};
use aphel_sdk::platform::application::shared::{DomainEvent, REDACTED, RecordRef};
use aphel_sdk::platform::domain::fields::FieldDefinition;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_sdk::platform::state::runtime_pool;

const MAX_BODY_CHANGES: usize = 3;
const MAX_VALUE_LENGTH: usize = 80;
const EMPTY_VALUE: &str = "—";

pub(crate) const FOLLOWERS_SOURCE_ORDER: i32 = 100;

pub(crate) struct FollowedRecordNotifications;

type FollowedSignal = (
    Option<std::sync::Arc<dyn aphel_sdk::followers::FollowedEventSource>>,
    aphel_sdk::followers::FollowedEvent,
);

fn followed_event(event: &DomainEvent) -> Result<Option<FollowedSignal>, AppError> {
    let sources = aphel_sdk::platform::modules::app()
        .contributions::<dyn aphel_sdk::followers::FollowedEventSource>();
    let mut keys = std::collections::HashSet::new();
    for source in &sources {
        if source.key().is_empty()
            || source.key().len() > 64
            || !source.key().bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b'-')
            })
            || !keys.insert(source.key().to_owned())
        {
            return Err(AppError::Configuration(
                "invalid or duplicate followed-event source".into(),
            ));
        }
    }
    let mut matching = sources
        .into_iter()
        .filter(|source| source.accepts(&event.event_type));
    if let Some(source) = matching.next() {
        if matching.next().is_some() {
            return Err(AppError::Configuration(
                "ambiguous followed-event sources".into(),
            ));
        }
        return Ok(source.signal(event).map(|signal| (Some(source), signal)));
    }
    let Some(record) = event.record.clone() else {
        return Ok(None);
    };
    let Some(prefix) = event.event_type.strip_suffix(".updated") else {
        return Ok(None);
    };
    let registry = aphel_sdk::platform::modules::app().record_types();
    if registry
        .get(&record.record_type)
        .map(|definition| definition.event_prefix.as_str())
        .unwrap_or(record.record_type.as_str())
        != prefix
    {
        return Ok(None);
    }
    Ok(Some((
        None,
        aphel_sdk::followers::FollowedEvent {
            targets: vec![record],
            required_records: Vec::new(),
            body: None,
            body_field: None,
            changes: true,
            headline: aphel_sdk::fields::LocalizedLabels(std::collections::BTreeMap::from([
                ("en".into(), "{{actor}} changed a record you follow".into()),
                (
                    "pt-BR".into(),
                    "{{actor}} alterou um registro que você segue".into(),
                ),
            ])),
        },
    )))
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
        event_type.ends_with(".updated")
            || aphel_sdk::platform::modules::app()
                .contributions::<dyn aphel_sdk::followers::FollowedEventSource>()
                .iter()
                .any(|source| source.accepts(event_type))
    }

    fn order(&self) -> i32 {
        FOLLOWERS_SOURCE_ORDER
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let Some((source, mut signal)) = followed_event(event)? else {
            return Ok(Vec::new());
        };
        if signal.targets.len() > 200 || signal.required_records.len() > 200 {
            return Err(AppError::Configuration(
                "followed-event target limit exceeded".into(),
            ));
        }
        if signal.body_field.as_ref().is_some_and(|(record, _)| {
            !signal.required_records.contains(record) && !signal.targets.contains(record)
        }) {
            return Err(AppError::Configuration(
                "followed-event body lacks a readable record requirement".into(),
            ));
        }
        signal.targets.sort_by(|left, right| {
            left.record_type
                .as_str()
                .cmp(right.record_type.as_str())
                .then_with(|| left.record_id.cmp(&right.record_id))
        });
        signal.targets.dedup();
        let activity = source
            .as_ref()
            .map(|source| source.key())
            .unwrap_or("update");
        let pool = runtime_pool()?;
        let kind = FOLLOWED_RECORD_ACTIVITY_NOTIFICATION;
        let mut seen: Vec<UserId> = context.notified.to_vec();
        seen.extend(context.actor.cloned());
        let changes = if signal.changes {
            payload_changes(&event.payload)
        } else {
            Vec::new()
        };
        let mut intents = Vec::new();
        for record in &signal.targets {
            let followers: Vec<UserId> = context
                .directory
                .followers(record)
                .await?
                .into_iter()
                .filter(|follower| !seen.contains(follower))
                .collect();
            if followers.is_empty() {
                continue;
            }
            let descriptions = if changes.is_empty() {
                None
            } else {
                Some(ChangeDescriptions::load(record).await?)
            };
            for follower in followers {
                if seen.contains(&follower) {
                    continue;
                }
                let Some(title) = context.directory.visible_title(&follower, record).await? else {
                    continue;
                };
                let mut readable = true;
                for required in &signal.required_records {
                    if !context.directory.can_read(&follower, required).await? {
                        readable = false;
                        break;
                    }
                }
                if !readable {
                    continue;
                }
                if let Some(source) = &source
                    && !source.visible(&pool, &follower, event).await?
                {
                    continue;
                }
                let (items, change_body) = match &descriptions {
                    Some(descriptions) => {
                        visible_changes(record, &follower, &changes, descriptions).await?
                    }
                    None => (Vec::new(), None),
                };
                if !changes.is_empty() && items.is_empty() {
                    continue;
                }
                let mut body = signal.body.clone().or(change_body);
                if let Some((body_record, field)) = &signal.body_field {
                    let redaction =
                        user_field_redaction(&pool, Some(&follower), &body_record.record_type)
                            .await?;
                    if redaction.hides_path(field) {
                        body = None;
                    }
                }
                let mut data = Map::new();
                data.insert("activity".to_owned(), json!(activity));
                data.insert("headline".to_owned(), json!(signal.headline.0));
                data.insert("event_type".to_owned(), json!(event.event_type));
                data.insert("record_type".to_owned(), json!(record.record_type.as_str()));
                data.insert("record_id".to_owned(), json!(record.record_id.to_string()));
                if !items.is_empty() {
                    data.insert("changes".to_owned(), Value::Array(items));
                }
                seen.push(follower.clone());
                intents.push(
                    NotificationIntent::new(
                        follower,
                        kind,
                        title.clone(),
                        format!(
                            "{}:{}:{}",
                            event_dedupe(event, kind),
                            record.record_type.as_str(),
                            record.record_id
                        ),
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
        let labels: aphel_sdk::fields::LocalizedLabels =
            serde_json::from_value(view.data.get("headline")?.clone()).ok()?;
        Some(labels.get(locale.html_lang()).replace("{{actor}}", actor))
    }
}
