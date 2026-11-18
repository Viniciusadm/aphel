use aphel_contract_crm::CrmPermissions;
use aphel_sdk::platform::database::Db;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_sdk::messaging::mentions::mentions_to_text;
use aphel_sdk::platform::application::shared::{DomainEvent, RecordType};
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::search::{SearchContentSource, SearchRefresh};

use crate::domain::{NOTE_CREATED, NOTE_DELETED, NOTE_UPDATED, NoteVisibility};

const MAX_NOTES_PER_RECORD: usize = 50;

pub(crate) struct NoteSearchContent;

#[async_trait]
impl SearchContentSource for NoteSearchContent {
    fn key(&self) -> &'static str {
        "notes"
    }

    fn read_permission(&self) -> Option<Permission> {
        Some(Permission::NOTES_READ)
    }

    fn applies_to(&self, _record_type: &RecordType) -> bool {
        true
    }

    async fn load(
        &self,
        pool: &Db,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let mut texts: HashMap<Uuid, String> = HashMap::new();
        if record_ids.is_empty() {
            return Ok(texts);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT record_id, body FROM notes WHERE deleted_at IS NULL AND visibility = ",
        );
        builder
            .push_bind(NoteVisibility::Team.as_str())
            .push(" AND record_type = ")
            .push_bind(record_type.as_str().to_owned())
            .push(" AND record_id IN (");
        {
            let mut ids = builder.separated(", ");
            for id in record_ids {
                ids.push_bind(id.to_string());
            }
        }
        builder.push(") ORDER BY pinned DESC, created_at DESC");
        let mut counts: HashMap<Uuid, usize> = HashMap::new();
        for row in builder.build().fetch_all(pool).await? {
            let record_id: String = row.try_get("record_id")?;
            let Ok(record_id) = Uuid::parse_str(&record_id) else {
                continue;
            };
            let count = counts.entry(record_id).or_default();
            if *count >= MAX_NOTES_PER_RECORD {
                continue;
            }
            *count += 1;
            let body: String = row.try_get("body")?;
            let text = texts.entry(record_id).or_default();
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&mentions_to_text(&body));
        }
        Ok(texts)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        matches!(event_type, NOTE_CREATED | NOTE_UPDATED | NOTE_DELETED)
    }

    async fn affected(
        &self,
        _pool: &Db,
        event: &DomainEvent,
    ) -> Result<Vec<SearchRefresh>, AppError> {
        let record_type = event
            .payload
            .get("record_type")
            .and_then(Value::as_str)
            .and_then(|value| RecordType::parse(value).ok());
        let record_id = event
            .payload
            .get("record_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        Ok(match (record_type, record_id) {
            (Some(record_type), Some(record_id)) => {
                vec![SearchRefresh::Records(record_type, vec![record_id])]
            }
            _ => Vec::new(),
        })
    }
}
