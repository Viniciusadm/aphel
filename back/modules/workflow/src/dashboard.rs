use aphel_sdk::dashboard::DashboardReferenceCatalog;
use aphel_sdk::{AppError, ReferenceKind};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::MySqlPool;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

pub(crate) struct WorkflowDashboardReferences;
#[async_trait]
impl DashboardReferenceCatalog for WorkflowDashboardReferences {
    fn kind(&self) -> ReferenceKind {
        ReferenceKind::new("workflow.pipeline")
    }
    fn package_keys(&self, sections: &BTreeMap<String, Value>) -> Vec<String> {
        sections
            .get(crate::section::PIPELINES_SECTION)
            .and_then(|value| crate::section::parse_pipelines(value).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|pipeline| pipeline.key)
            .collect()
    }
    async fn keys(&self, pool: &MySqlPool) -> Result<HashMap<Uuid, String>, AppError> {
        Ok(crate::section::keyed_pipelines(
            &crate::store::PipelineStore::new(pool.clone()),
            &config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone()),
        )
        .await?
        .into_iter()
        .map(|(pipeline, keyed)| (pipeline.id, keyed.key))
        .collect())
    }
}
