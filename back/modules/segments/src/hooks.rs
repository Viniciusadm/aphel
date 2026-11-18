use async_trait::async_trait;
use uuid::Uuid;

use crate::store::SegmentStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::query::{FilterSource, FilterSourceResolver};
use aphel_sdk::platform::database::Db;

pub(crate) struct StoredFilterSources;

#[async_trait]
impl FilterSourceResolver for StoredFilterSources {
    fn field(&self) -> &'static str {
        "segment"
    }

    async fn source(&self, pool: &Db, segment_id: Uuid) -> Result<Option<FilterSource>, AppError> {
        SegmentStore::new(pool.clone()).source(segment_id).await
    }
}
