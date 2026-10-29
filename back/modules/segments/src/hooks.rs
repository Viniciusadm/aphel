use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::store::SegmentStore;
use aphel_kernel::application::query::{SegmentSource, SegmentSourceResolver};
use aphel_kernel::errors::AppError;

pub(crate) struct StoredSegmentSources;

#[async_trait]
impl SegmentSourceResolver for StoredSegmentSources {
    async fn source(
        &self,
        pool: &MySqlPool,
        segment_id: Uuid,
    ) -> Result<Option<SegmentSource>, AppError> {
        SegmentStore::new(pool.clone()).source(segment_id).await
    }
}
