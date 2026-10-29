use async_trait::async_trait;
use std::collections::HashMap;

use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::domain::EmailVisibility;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use timeline::domain::{TimelineEntry, TimelineEventExtension, TimelineRequest};

pub(crate) const EMAIL_EVENT_SQL: &str =
    "(e.event_type LIKE 'email.%' OR e.event_type LIKE 'email!_thread.%' ESCAPE '!')";

pub(crate) fn push_email_visibility(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    subject: &RecordRef,
    can_read_emails: bool,
) {
    builder.push(" AND (NOT ");
    builder.push(EMAIL_EVENT_SQL);
    if can_read_emails {
        builder
            .push(" OR (e.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND e.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push(
                " AND EXISTS (SELECT 1 FROM email_threads et JOIN mailboxes em ON em.id = et.mailbox_id \
                 WHERE et.id = JSON_UNQUOTE(JSON_EXTRACT(e.payload, '$.thread_id')) \
                 AND (em.user_id = ",
            )
            .push_bind(scope.user_id().as_str().to_owned())
            .push(" OR COALESCE(et.visibility, em.default_visibility) <> 'private')))");
    }
    builder.push(")");
}

fn payload_text(entry: &TimelineEntry, key: &str) -> Option<String> {
    entry
        .metadata
        .get("payload")
        .and_then(|payload| payload.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn is_email_entry(entry: &TimelineEntry) -> bool {
    entry
        .metadata
        .get("event_type")
        .and_then(Value::as_str)
        .is_some_and(|event_type| {
            event_type.starts_with("email.") || event_type.starts_with("email_thread.")
        })
}

pub(crate) async fn enrich_email_entries(
    pool: &MySqlPool,
    scope: &AccessScope,
    entries: &mut [TimelineEntry],
) -> Result<(), AppError> {
    let threads: Vec<String> = entries
        .iter()
        .filter(|entry| is_email_entry(entry))
        .filter_map(|entry| payload_text(entry, "thread_id"))
        .collect();
    if threads.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT t.id, t.subject, t.snippet, t.visibility, m.default_visibility, m.user_id \
         FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id WHERE t.id IN (",
    );
    let mut separated = builder.separated(", ");
    for id in &threads {
        separated.push_bind(id.clone());
    }
    builder.push(")");
    let rows = builder.build().fetch_all(pool).await?;
    let mut thread_info: HashMap<String, (String, String, &'static str)> = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let visibility: Option<String> = row.try_get("visibility")?;
        let default: String = row.try_get("default_visibility")?;
        let owner: String = row.try_get("user_id")?;
        let effective = visibility
            .as_deref()
            .and_then(EmailVisibility::parse)
            .or_else(|| EmailVisibility::parse(&default))
            .unwrap_or_default();
        let access = effective.access_for(owner == scope.user_id().as_str());
        let access = if access.can_read_content() {
            "full"
        } else if access.can_see() {
            "metadata"
        } else {
            "none"
        };
        thread_info.insert(
            id,
            (row.try_get("subject")?, row.try_get("snippet")?, access),
        );
    }
    let emails: Vec<String> = entries
        .iter()
        .filter(|entry| is_email_entry(entry))
        .filter_map(|entry| payload_text(entry, "email_id"))
        .collect();
    let mut messages: HashMap<String, (String, String, String, Value)> = HashMap::new();
    if !emails.is_empty() {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT e.id, e.subject, e.preview, e.direction, p.role, p.address, p.name, p.contact_id \
             FROM email_messages e LEFT JOIN email_participants p ON p.message_id = e.id AND p.role = 'from' \
             WHERE e.id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in &emails {
            separated.push_bind(id.clone());
        }
        builder.push(")");
        for row in builder.build().fetch_all(pool).await? {
            let id: String = row.try_get("id")?;
            let address: Option<String> = row.try_get("address")?;
            let from = match address {
                Some(address) => json!({
                    "role": "from",
                    "address": address,
                    "name": row.try_get::<Option<String>, _>("name")?,
                    "contact_id": row.try_get::<Option<String>, _>("contact_id")?,
                }),
                None => Value::Null,
            };
            messages.insert(
                id,
                (
                    row.try_get("subject")?,
                    row.try_get("preview")?,
                    row.try_get("direction")?,
                    from,
                ),
            );
        }
    }
    for entry in entries.iter_mut().filter(|entry| is_email_entry(entry)) {
        let Some(thread_id) = payload_text(entry, "thread_id") else {
            continue;
        };
        let Some((thread_subject, thread_snippet, access)) = thread_info.get(&thread_id) else {
            continue;
        };
        if *access == "none" {
            continue;
        }
        let message = payload_text(entry, "email_id").and_then(|id| messages.get(&id));
        let full = *access == "full";
        let (subject, snippet, direction, from) = match message {
            Some((subject, preview, direction, from)) => (
                if subject.is_empty() {
                    thread_subject.clone()
                } else {
                    subject.clone()
                },
                preview.clone(),
                Value::String(direction.clone()),
                from.clone(),
            ),
            None => (
                thread_subject.clone(),
                thread_snippet.clone(),
                entry
                    .metadata
                    .get("payload")
                    .and_then(|payload| payload.get("direction"))
                    .cloned()
                    .unwrap_or(Value::Null),
                Value::Null,
            ),
        };
        if let Some(metadata) = entry.metadata.as_object_mut() {
            metadata.insert(
                "email".to_owned(),
                json!({
                    "thread_id": thread_id,
                    "subject": subject,
                    "direction": direction,
                    "access": access,
                    "snippet": if full { Value::String(snippet) } else { Value::Null },
                    "from": from,
                }),
            );
        }
    }
    Ok(())
}

pub(crate) struct EmailTimeline;

#[async_trait]
impl TimelineEventExtension for EmailTimeline {
    fn restrict(
        &self,
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        subject: &RecordRef,
        request: &TimelineRequest,
    ) {
        push_email_visibility(builder, scope, subject, request.emails);
    }

    async fn enrich(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        entries: &mut [TimelineEntry],
    ) -> Result<(), AppError> {
        enrich_email_entries(pool, scope, entries).await
    }
}
