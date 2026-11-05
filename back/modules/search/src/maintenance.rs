use crate::application::{ReindexOutcome, SearchUseCases, reindex_job};
use crate::infrastructure::MySqlSearchPorts;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::Job;
use aphel_sdk::search::{SearchMaintenance, SearchReindexSummary};
use sqlx::MySqlPool;
use std::sync::Arc;

pub(crate) struct Maintenance;

#[async_trait::async_trait]
impl SearchMaintenance for Maintenance {
    fn reindex_job(&self) -> Job {
        reindex_job(None, None)
    }

    async fn rebuild(
        &self,
        pool: &MySqlPool,
        registry: &Arc<RecordTypeRegistry>,
    ) -> Result<SearchReindexSummary, AppError> {
        let outcome = SearchUseCases::new(MySqlSearchPorts::new(pool.clone(), registry.clone()))
            .reindex_organization(None, None, None)
            .await?;
        let (ReindexOutcome::Finished(summary) | ReindexOutcome::Paused(summary, _)) = outcome;
        Ok(SearchReindexSummary {
            indexed: summary.indexed,
            removed: summary.removed,
        })
    }
}
