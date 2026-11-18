use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{DomainEvent, RecordType};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::search::{SearchContentSource, SearchRefresh};

use crate::service::{
    RECORD_TAGGED, RECORD_UNTAGGED, TAG_DELETED, TAG_RECORD_TYPE, TAG_UPDATED, taggable_record_type,
};
use crate::store::TagStore;

pub(crate) struct TagSearchContent;

#[async_trait]
impl SearchContentSource for TagSearchContent {
    fn key(&self) -> &'static str {
        "tags"
    }

    fn read_permission(&self) -> Option<Permission> {
        None
    }

    fn applies_to(&self, record_type: &RecordType) -> bool {
        taggable_record_type(record_type.as_str()).is_ok()
    }

    async fn load(
        &self,
        pool: &Db,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let mut names: HashMap<Uuid, String> = HashMap::new();
        for (record_id, tag) in TagStore::new(pool.clone())
            .tags_of(record_type.as_str(), record_ids)
            .await?
        {
            let text = names.entry(record_id).or_default();
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&tag.name);
        }
        Ok(names)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            RECORD_TAGGED | RECORD_UNTAGGED | TAG_UPDATED | TAG_DELETED
        )
    }

    async fn affected(
        &self,
        pool: &Db,
        event: &DomainEvent,
    ) -> Result<Vec<SearchRefresh>, AppError> {
        let Some(record) = event.record.as_ref() else {
            return Ok(Vec::new());
        };
        match event.event_type.as_str() {
            RECORD_TAGGED | RECORD_UNTAGGED => Ok(vec![SearchRefresh::Records(
                record.record_type.clone(),
                vec![record.record_id],
            )]),
            TAG_DELETED => Ok(vec![SearchRefresh::All]),
            TAG_UPDATED if record.record_type == TAG_RECORD_TYPE => {
                tagged_records(pool, record.record_id).await
            }
            _ => Ok(Vec::new()),
        }
    }
}

async fn tagged_records(pool: &Db, tag_id: Uuid) -> Result<Vec<SearchRefresh>, AppError> {
    let rows = crate::store::tagged_records(pool, tag_id).await?;
    let mut grouped: HashMap<String, Vec<Uuid>> = HashMap::new();
    for (record_type, record_id) in rows {
        if let Ok(record_id) = Uuid::parse_str(&record_id) {
            grouped.entry(record_type).or_default().push(record_id);
        }
    }
    let mut refreshes = Vec::new();
    for (record_type, ids) in grouped {
        if let Ok(record_type) = RecordType::parse(&record_type) {
            refreshes.push(SearchRefresh::Records(record_type, ids));
        }
    }
    Ok(refreshes)
}
