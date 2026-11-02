use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

const CUSTOM_OBJECT_UPDATED: &str = "custom_object.updated";
use crate::application::contents::SearchRefresh;
use crate::application::ports::SearchPorts;
use crate::application::use_cases::{ReindexCursor, ReindexOutcome, SearchUseCases};
use aphel_kernel::application::shared::{
    DomainEvent, EventConsumer, Job, JobContext, JobHandler, JobQueue, RecordType,
};
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::AppError;

pub const SEARCH_INDEX_CONSUMER: &str = "search.index";
pub const SEARCH_REINDEX_JOB: &str = "search.reindex";
pub const REINDEX_TIME_BUDGET: Duration = Duration::from_secs(240);

const FIELD_EVENT_PREFIX: &str = "field.";
const FIELD_EVENTS: &[&str] = &[
    "field.updated",
    "field.activated",
    "field.deactivated",
    CUSTOM_OBJECT_UPDATED,
];
const MERGED_ID_KEY: &str = "merged_id";

pub fn reindex_job(record_type: Option<&RecordType>, cursor: Option<&ReindexCursor>) -> Job {
    let mut payload = json!({});
    let mut unique_key = SEARCH_REINDEX_JOB.to_owned();
    if let Some(record_type) = record_type {
        payload["record_type"] = json!(record_type.as_str());
        unique_key.push(':');
        unique_key.push_str(record_type.as_str());
    }
    if let Some(cursor) = cursor {
        payload["cursor"] = json!({
            "record_type": cursor.record_type.as_str(),
            "after": cursor.after.map(|after| after.to_string()),
            "started_at": cursor.started_at.to_rfc3339(),
        });
        unique_key.push_str(&format!(
            ":{}:{}",
            cursor.record_type.as_str(),
            cursor
                .after
                .map(|after| after.to_string())
                .unwrap_or_default()
        ));
    }
    Job::new(SEARCH_REINDEX_JOB, payload).with_unique_key(unique_key)
}

fn parse_cursor(value: &Value) -> Result<Option<ReindexCursor>, AppError> {
    let Some(cursor) = value.get("cursor").filter(|cursor| !cursor.is_null()) else {
        return Ok(None);
    };
    let invalid = || AppError::Internal("invalid search reindex cursor".to_owned());
    let record_type = cursor
        .get("record_type")
        .and_then(Value::as_str)
        .map(RecordType::parse)
        .transpose()?
        .ok_or_else(invalid)?;
    let after = cursor
        .get("after")
        .and_then(Value::as_str)
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| invalid())?;
    let started_at = cursor
        .get("started_at")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(invalid)?;
    Ok(Some(ReindexCursor {
        record_type,
        after,
        started_at,
    }))
}

pub struct SearchIndexConsumer<P> {
    use_cases: SearchUseCases<P>,
}

impl<P> SearchIndexConsumer<P>
where
    P: SearchPorts,
{
    pub fn new(ports: P) -> Self {
        Self {
            use_cases: SearchUseCases::new(ports),
        }
    }

    async fn refresh(&self, refreshes: Vec<SearchRefresh>) -> Result<(), AppError> {
        for refresh in refreshes {
            match refresh {
                SearchRefresh::Records(record_type, ids) => {
                    if self.registered(record_type.as_str()).is_some() && !ids.is_empty() {
                        self.use_cases.reindex_records(&record_type, &ids).await?;
                    }
                }
                SearchRefresh::Type(record_type) => {
                    if self.registered(record_type.as_str()).is_some() {
                        self.use_cases
                            .ports()
                            .jobs()
                            .enqueue_now(reindex_job(Some(&record_type), None))
                            .await?;
                    }
                }
                SearchRefresh::All => {
                    self.use_cases
                        .ports()
                        .jobs()
                        .enqueue_now(reindex_job(None, None))
                        .await?;
                }
            }
        }
        Ok(())
    }

    fn registered(&self, record_type: &str) -> Option<RecordType> {
        RecordType::parse(record_type)
            .ok()
            .filter(|record_type| self.use_cases.ports().registry().get(record_type).is_some())
    }
}

#[async_trait]
impl<P> EventConsumer for SearchIndexConsumer<P>
where
    P: SearchPorts,
{
    fn name(&self) -> &str {
        SEARCH_INDEX_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        if self.use_cases.ports().contents().listens_to(event_type) {
            return true;
        }
        if event_type.starts_with(FIELD_EVENT_PREFIX) {
            return FIELD_EVENTS.contains(&event_type);
        }
        if event_type == CUSTOM_OBJECT_UPDATED {
            return true;
        }
        event_type.rsplit_once('.').is_some_and(|(prefix, _)| {
            is_custom_record_type(prefix) || self.registered(prefix).is_some()
        })
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        if self
            .use_cases
            .ports()
            .contents()
            .listens_to(&event.event_type)
        {
            let refreshes = self.use_cases.ports().contents().affected(event).await?;
            return self.refresh(refreshes).await;
        }
        if FIELD_EVENTS.contains(&event.event_type.as_str()) {
            let Some(record_type) = event
                .payload
                .get("record_type")
                .and_then(Value::as_str)
                .and_then(|record_type| self.registered(record_type))
            else {
                return Ok(());
            };
            self.use_cases
                .ports()
                .jobs()
                .enqueue_now(reindex_job(Some(&record_type), None))
                .await?;
            return Ok(());
        }
        let Some(record) = event.record.as_ref() else {
            return Ok(());
        };
        if self.registered(record.record_type.as_str()).is_none() {
            return Ok(());
        }
        let mut ids = vec![record.record_id];
        if let Some(merged) = event
            .payload
            .get(MERGED_ID_KEY)
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
        {
            ids.push(merged);
        }
        self.use_cases
            .reindex_records(&record.record_type, &ids)
            .await?;
        Ok(())
    }
}

pub struct SearchReindexHandler<P> {
    use_cases: SearchUseCases<P>,
}

impl<P> SearchReindexHandler<P>
where
    P: SearchPorts,
{
    pub fn new(ports: P) -> Self {
        Self {
            use_cases: SearchUseCases::new(ports),
        }
    }
}

#[async_trait]
impl<P> JobHandler for SearchReindexHandler<P>
where
    P: SearchPorts,
{
    fn kind(&self) -> &str {
        SEARCH_REINDEX_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let record_type = payload
            .get("record_type")
            .and_then(Value::as_str)
            .map(RecordType::parse)
            .transpose()?;
        let cursor = parse_cursor(&payload)?;
        let deadline = Instant::now() + REINDEX_TIME_BUDGET;
        match self
            .use_cases
            .reindex_organization(cursor, record_type.as_ref(), Some(deadline))
            .await?
        {
            ReindexOutcome::Finished(summary) => {
                tracing::info!(
                    indexed = summary.indexed,
                    removed = summary.removed,
                    "search reindex finished"
                );
            }
            ReindexOutcome::Paused(summary, cursor) => {
                tracing::info!(
                    indexed = summary.indexed,
                    removed = summary.removed,
                    record_type = cursor.record_type.as_str(),
                    "search reindex paused, continuing in a new job"
                );
                self.use_cases
                    .ports()
                    .jobs()
                    .enqueue_now(reindex_job(record_type.as_ref(), Some(&cursor)))
                    .await?;
            }
        }
        Ok(())
    }
}
