use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;

pub const CONTENT_KEY_PREFIX: char = '@';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchRefresh {
    Records(RecordType, Vec<Uuid>),
    Type(RecordType),
    All,
}

#[async_trait]
pub trait SearchContentSource: Send + Sync {
    fn key(&self) -> &'static str;

    fn read_permission(&self) -> Option<Permission>;

    fn applies_to(&self, record_type: &RecordType) -> bool;

    async fn load(
        &self,
        pool: &MySqlPool,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError>;

    fn listens_to(&self, event_type: &str) -> bool;

    async fn affected(
        &self,
        pool: &MySqlPool,
        event: &DomainEvent,
    ) -> Result<Vec<SearchRefresh>, AppError>;
}

pub fn is_content_key(key: &str) -> bool {
    key.starts_with(CONTENT_KEY_PREFIX)
}

pub fn content_key(source: &dyn SearchContentSource) -> String {
    format!("{CONTENT_KEY_PREFIX}{}", source.key())
}

#[derive(Clone, Default)]
pub struct SearchContents {
    pool: Option<MySqlPool>,
    sources: Vec<Arc<dyn SearchContentSource>>,
}

impl SearchContents {
    pub fn new(pool: MySqlPool, sources: Vec<Arc<dyn SearchContentSource>>) -> Self {
        Self {
            pool: Some(pool),
            sources,
        }
    }

    pub fn from_app(pool: MySqlPool) -> Self {
        let sources = aphel_kernel::modules::try_app()
            .map(|app| app.contributions::<dyn SearchContentSource>())
            .unwrap_or_default();
        Self::new(pool, sources)
    }

    pub fn visible_keys(&self, scope: &AccessScope, record_type: &RecordType) -> Vec<String> {
        self.sources
            .iter()
            .filter(|source| source.applies_to(record_type))
            .filter(|source| {
                source
                    .read_permission()
                    .is_none_or(|permission| scope.record_filter(&permission).is_ok())
            })
            .map(|source| content_key(source.as_ref()))
            .collect()
    }

    pub async fn load(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<(String, String)>>, AppError> {
        let mut result: HashMap<Uuid, Vec<(String, String)>> = HashMap::new();
        let Some(pool) = self.pool.as_ref() else {
            return Ok(result);
        };
        if record_ids.is_empty() {
            return Ok(result);
        }
        for source in self
            .sources
            .iter()
            .filter(|source| source.applies_to(record_type))
        {
            let key = content_key(source.as_ref());
            for (record_id, text) in source.load(pool, record_type, record_ids).await? {
                if !text.trim().is_empty() {
                    result
                        .entry(record_id)
                        .or_default()
                        .push((key.clone(), text));
                }
            }
        }
        Ok(result)
    }

    pub fn listens_to(&self, event_type: &str) -> bool {
        self.sources
            .iter()
            .any(|source| source.listens_to(event_type))
    }

    pub async fn affected(&self, event: &DomainEvent) -> Result<Vec<SearchRefresh>, AppError> {
        let mut refreshes = Vec::new();
        let Some(pool) = self.pool.as_ref() else {
            return Ok(refreshes);
        };
        for source in self
            .sources
            .iter()
            .filter(|source| source.listens_to(&event.event_type))
        {
            refreshes.extend(source.affected(pool, event).await?);
        }
        Ok(refreshes)
    }
}
