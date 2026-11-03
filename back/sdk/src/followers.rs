use crate::{
    AppError,
    fields::LocalizedLabels,
    shared::{DomainEvent, RecordRef},
};
use aphel_kernel::domain::UserId;
use async_trait::async_trait;
use sqlx::MySqlPool;

pub struct FollowedEvent {
    pub targets: Vec<RecordRef>,
    pub required_records: Vec<RecordRef>,
    pub headline: LocalizedLabels,
    pub body: Option<String>,
    pub body_field: Option<(RecordRef, String)>,
    pub changes: bool,
}

#[async_trait]
pub trait FollowedEventSource: Send + Sync {
    fn key(&self) -> &str;
    fn accepts(&self, event_type: &str) -> bool;
    fn signal(&self, event: &DomainEvent) -> Option<FollowedEvent>;
    async fn visible(
        &self,
        _pool: &MySqlPool,
        _recipient: &UserId,
        _event: &DomainEvent,
    ) -> Result<bool, AppError> {
        Ok(true)
    }
}
