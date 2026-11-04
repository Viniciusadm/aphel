use aphel_kernel::application::shared::{DomainEvent, RecordType};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::collections::HashMap;
use uuid::Uuid;

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
