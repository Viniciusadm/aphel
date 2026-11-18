use crate::store::PipelineStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{
    ConfigKeyClient, ConfigKeyRepository, ConfigReferenceKind, ConfigurationReferences, assign_keys,
};
use aphel_sdk::platform::database::Db;
use std::collections::HashMap;
use uuid::Uuid;

const PIPELINE: ConfigReferenceKind = ConfigReferenceKind::new("workflow.pipeline");
const STAGE: ConfigReferenceKind = ConfigReferenceKind::new("workflow.stage");
const LOST_REASON: ConfigReferenceKind = ConfigReferenceKind::new("workflow.lost_reason");

pub(crate) struct References;

#[async_trait::async_trait]
impl ConfigurationReferences for References {
    fn kinds(&self) -> &'static [ConfigReferenceKind] {
        &[PIPELINE, STAGE, LOST_REASON]
    }
    fn package_keys(
        &self,
        package: &aphel_sdk::configuration::ConfigPackage,
        kind: ConfigReferenceKind,
    ) -> std::collections::BTreeSet<String> {
        let pipelines = package
            .section(crate::section::PIPELINES_SECTION)
            .and_then(|content| crate::section::parse_pipelines(content).ok())
            .unwrap_or_default();
        match kind {
            PIPELINE => pipelines.into_iter().map(|entry| entry.key).collect(),
            STAGE => pipelines
                .into_iter()
                .flat_map(|entry| {
                    entry
                        .stages
                        .into_iter()
                        .map(move |stage| format!("{}/{}", entry.key, stage.key))
                })
                .collect(),
            _ => std::collections::BTreeSet::new(),
        }
    }
    async fn keys(
        &self,
        pool: &Db,
        kind: ConfigReferenceKind,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let workflow = PipelineStore::new(pool.clone());
        let keys = ConfigKeyClient::new(pool.clone());
        let mut map: HashMap<ConfigReferenceKind, HashMap<Uuid, String>> = HashMap::new();
        let all_pipelines = workflow.list_pipelines().await?;
        let stored = keys
            .keys(ConfigReferenceKind::new("workflow.pipeline"))
            .await?;
        let pipeline_keys = assign_keys(
            all_pipelines
                .iter()
                .map(|pipeline| (pipeline.id, pipeline.name.as_str())),
            &stored,
        );
        let stored_stages = keys
            .keys(ConfigReferenceKind::new("workflow.stage"))
            .await?;
        for (pipeline, keyed) in all_pipelines.iter().zip(&pipeline_keys) {
            map.entry(PIPELINE)
                .or_default()
                .insert(pipeline.id, keyed.key.clone());
            let stages = workflow.list_stages(pipeline.id).await?;
            let stored: HashMap<Uuid, String> = stages
                .iter()
                .filter_map(|stage| {
                    stored_stages
                        .get(&stage.id)
                        .map(|key| (stage.id, key.clone()))
                })
                .collect();
            for (stage, stage_key) in stages.iter().zip(assign_keys(
                stages.iter().map(|stage| (stage.id, stage.name.as_str())),
                &stored,
            )) {
                map.entry(STAGE)
                    .or_default()
                    .insert(stage.id, format!("{}/{}", keyed.key, stage_key.key));
            }
        }

        let reasons = workflow.list_lost_reasons().await?;
        let stored = keys
            .keys(ConfigReferenceKind::new("workflow.lost_reason"))
            .await?;
        for (reason, keyed) in reasons.iter().zip(assign_keys(
            reasons
                .iter()
                .map(|reason| (reason.id, reason.name.as_str())),
            &stored,
        )) {
            map.entry(LOST_REASON)
                .or_default()
                .insert(reason.id, keyed.key);
        }

        Ok(map.remove(&kind).unwrap_or_default())
    }
}
