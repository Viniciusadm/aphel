use std::collections::HashSet;

use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::model::AiSource;
use crate::records;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;

pub(crate) const RECORD_NOT_FOUND: &str = "ai_record_not_found";
pub(crate) const UNSUPPORTED_RECORD_TYPE: &str = "ai_unsupported_record_type";

const MAX_CONTEXT_CHARS: usize = 30_000;
const MAX_NOTE_CHARS: usize = 600;
const MAX_DESCRIPTION_CHARS: usize = 400;
const MAX_TEXT_CHARS: usize = 1_000;
const RELATED_LIMIT: u64 = 10;
const LABEL_KEYS: [&str; 4] = ["name", "display_name", "title", "label"];
const PEOPLE_KEYS: [&str; 3] = ["owner", "contact", "author"];
const NAME_PARTS: [&str; 2] = ["first_name", "last_name"];
const SKIPPED_KEYS: [&str; 4] = ["id", "created_by", "possible_duplicates", "links"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextDepth {
    RecordOnly,
    Full,
}

pub(crate) struct RecordContext {
    pub(crate) text: String,
    pub(crate) sources: Vec<AiSource>,
}

pub(crate) struct FieldPolicy {
    blocked: HashSet<String>,
    allowed_custom: HashSet<String>,
    include_personal: bool,
}

impl FieldPolicy {
    pub(crate) async fn load(
        ctx: &Ctx<'_>,
        record_type: &str,
        include_personal: bool,
    ) -> Result<Self, AppError> {
        let fields = records::fields(ctx, record_type).await?;
        let mut blocked = HashSet::new();
        let mut allowed_custom = HashSet::new();
        for field in fields {
            let excluded = field.sensitive || (!include_personal && field.personal);
            match (field.custom, excluded) {
                (true, false) => {
                    allowed_custom.insert(field.key);
                }
                (false, true) => {
                    blocked.insert(field.key);
                }
                _ => {}
            }
        }
        if NAME_PARTS.iter().any(|part| blocked.contains(*part)) {
            blocked.insert("name".to_owned());
        }
        let references: Vec<String> = blocked
            .iter()
            .filter_map(|key| key.strip_suffix("_id").map(str::to_owned))
            .collect();
        blocked.extend(references);
        Ok(Self {
            blocked,
            allowed_custom,
            include_personal,
        })
    }

    pub(crate) fn sanitize(&self, value: &Value) -> Value {
        let Some(object) = value.as_object() else {
            return Value::Null;
        };
        let mut clean = Map::new();
        for (key, value) in object {
            if SKIPPED_KEYS.contains(&key.as_str())
                || key.ends_with("_id")
                || self.blocked.contains(key)
                || (!self.include_personal && PEOPLE_KEYS.contains(&key.as_str()))
            {
                continue;
            }
            let cleaned = if key == "custom_fields" {
                self.custom_fields(value)
            } else {
                compact(value)
            };
            if !is_empty(&cleaned) {
                clean.insert(key.clone(), cleaned);
            }
        }
        Value::Object(clean)
    }

    fn custom_fields(&self, value: &Value) -> Value {
        let Some(object) = value.as_object() else {
            return Value::Null;
        };
        Value::Object(
            object
                .iter()
                .filter(|(key, _)| self.allowed_custom.contains(*key))
                .map(|(key, value)| (key.clone(), compact(value)))
                .filter(|(_, value)| !is_empty(value))
                .collect(),
        )
    }
}

fn label_of(value: &Value) -> Option<String> {
    LABEL_KEYS
        .iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

fn compact(value: &Value) -> Value {
    match value {
        Value::Object(_) => label_of(value).map(Value::String).unwrap_or(Value::Null),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(compact)
                .filter(|item| !is_empty(item))
                .collect(),
        ),
        Value::String(text) => Value::String(truncate(text, MAX_TEXT_CHARS)),
        other => other.clone(),
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(object) => object.is_empty(),
        _ => false,
    }
}

pub(crate) fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let mut cut: String = text.chars().take(max).collect();
        cut.push('…');
        cut
    }
}

fn project(value: &Value, keys: &[&str]) -> Value {
    Value::Object(
        keys.iter()
            .filter_map(|key| {
                value
                    .get(*key)
                    .map(|field| ((*key).to_owned(), field.clone()))
            })
            .collect(),
    )
}

fn text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn ensure_context_type(ctx: &Ctx<'_>, record_type: &str) -> Result<(), AppError> {
    if ctx
        .registry()
        .definitions()
        .any(|entity| entity.record_type.as_str() == record_type && entity.title_field.is_some())
    {
        Ok(())
    } else {
        Err(AppError::validation_code(
            UNSUPPORTED_RECORD_TYPE,
            "record_type must name a registered entity with a title",
        ))
    }
}

pub(crate) fn record_not_found() -> AppError {
    AppError::not_found_code(RECORD_NOT_FOUND, "record not found")
}

struct Collector<'a> {
    ctx: &'a Ctx<'a>,
    include_personal: bool,
    sources: Vec<AiSource>,
}

impl Collector<'_> {
    async fn policy(&self, record_type: &str) -> Option<FieldPolicy> {
        FieldPolicy::load(self.ctx, record_type, self.include_personal)
            .await
            .ok()
    }

    fn source(&mut self, record_type: &str, item: &Value, label: Option<String>) {
        let id = text_field(item, "id");
        if !id.is_empty() {
            self.sources.push(AiSource {
                record_type: record_type.to_owned(),
                record_id: id,
                label,
            });
        }
    }

    async fn notes(&mut self, record_type: &str, record_id: Uuid) -> Vec<Value> {
        let items = records::notes(self.ctx, record_type, record_id, RELATED_LIMIT)
            .await
            .unwrap_or_default();
        let mut notes = Vec::new();
        for item in items {
            let mut note = Map::new();
            note.insert(
                "date".to_owned(),
                item.get("created_at").cloned().unwrap_or(Value::Null),
            );
            if self.include_personal
                && let Some(author) = item.get("author").and_then(label_of)
            {
                note.insert("author".to_owned(), json!(author));
            }
            note.insert(
                "body".to_owned(),
                json!(truncate(&text_field(&item, "body"), MAX_NOTE_CHARS)),
            );
            self.source("notes.note", &item, None);
            notes.push(Value::Object(note));
        }
        notes
    }

    async fn activities(&mut self, record_type: &str, record_id: Uuid) -> Vec<Value> {
        let Some(policy) = self.policy("activities.activity").await else {
            return Vec::new();
        };
        let items = records::activities(self.ctx, record_type, record_id, RELATED_LIMIT)
            .await
            .unwrap_or_default();
        let mut activities = Vec::new();
        for item in items {
            let mut projected = project(
                &item,
                &[
                    "title",
                    "activities.activity_type",
                    "status",
                    "priority",
                    "due_at",
                    "completed_at",
                ],
            );
            if let (Some(object), Some(description)) = (
                projected.as_object_mut(),
                item.get("description").and_then(Value::as_str),
            ) {
                object.insert(
                    "description".to_owned(),
                    json!(truncate(description, MAX_DESCRIPTION_CHARS)),
                );
            }
            self.source(
                "activities.activity",
                &item,
                item.get("title").and_then(Value::as_str).map(str::to_owned),
            );
            activities.push(policy.sanitize(&projected));
        }
        activities
    }

    async fn related(
        &mut self,
        record_type: &str,
        link_field: &str,
        record_id: Uuid,
        keys: &[&str],
    ) -> Vec<Value> {
        let Some(policy) = self.policy(record_type).await else {
            return Vec::new();
        };
        let items = match record_type {
            "deals.deal" => records::deals(self.ctx, link_field, record_id, RELATED_LIMIT).await,
            "contacts.contact" => {
                records::contacts(self.ctx, link_field, record_id, RELATED_LIMIT).await
            }
            _ => return Vec::new(),
        }
        .unwrap_or_default();
        let mut related = Vec::new();
        for item in items {
            self.source(record_type, &item, label_of(&item));
            related.push(policy.sanitize(&project(&item, keys)));
        }
        related
    }
}

const DEAL_KEYS: [&str; 8] = [
    "title",
    "amount",
    "currency",
    "status",
    "stage",
    "expected_close_date",
    "closed_at",
    "last_activity_at",
];

pub(crate) async fn build(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    include_personal: bool,
    depth: ContextDepth,
) -> Result<RecordContext, AppError> {
    ensure_context_type(ctx, record_type)?;
    let record_id = Uuid::parse_str(record_id).map_err(|_| record_not_found())?;
    let record = records::load(ctx, record_type, record_id)
        .await?
        .ok_or_else(record_not_found)?;
    let policy = FieldPolicy::load(ctx, record_type, include_personal).await?;
    let mut collector = Collector {
        ctx,
        include_personal,
        sources: vec![AiSource {
            record_type: record_type.to_owned(),
            record_id: record_id.to_string(),
            label: label_of(&record),
        }],
    };
    let mut context = Map::new();
    context.insert("record_type".to_owned(), json!(record_type));
    context.insert("record".to_owned(), policy.sanitize(&record));
    if depth == ContextDepth::Full {
        let notes = collector.notes(record_type, record_id).await;
        let activities = collector.activities(record_type, record_id).await;
        context.insert("recent_notes".to_owned(), Value::Array(notes));
        context.insert("recent_activities".to_owned(), Value::Array(activities));
        match record_type {
            "contacts.contact" => {
                let deals = collector
                    .related("deals.deal", "contact_id", record_id, &DEAL_KEYS)
                    .await;
                context.insert("deals".to_owned(), Value::Array(deals));
            }
            "companies.company" => {
                let contacts = collector
                    .related(
                        "contacts.contact",
                        "company_id",
                        record_id,
                        &["name", "job_title"],
                    )
                    .await;
                let deals = collector
                    .related("deals.deal", "company_id", record_id, &DEAL_KEYS)
                    .await;
                context.insert("contacts".to_owned(), Value::Array(contacts));
                context.insert("deals".to_owned(), Value::Array(deals));
            }
            _ => {}
        }
    }
    let text = serde_json::to_string(&Value::Object(context))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(RecordContext {
        text: truncate(&text, MAX_CONTEXT_CHARS),
        sources: collector.sources,
    })
}
