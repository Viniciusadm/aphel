use aphel_sdk::configuration::ConfigReferenceKind;
use aphel_sdk::platform::database::Db;
use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEFAULT_RENEWAL_DAYS_BEFORE, DEFAULT_RENEWAL_TITLE_PREFIX, MAX_RENEWAL_DAYS_BEFORE,
    MAX_RENEWAL_TITLE_PREFIX_LENGTH, RenewalOwnerMode, RenewalSettings,
};
use crate::store::RenewalStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::ConfigKeyClient;
use aphel_sdk::configuration::sections::common::diff;
use aphel_sdk::configuration::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, PackageIssue,
    PlanChange, PlanItem, assign_keys, parse_section,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::workflow::configuration::{PIPELINES_SECTION, parse_pipelines, stage_item_key};
use aphel_sdk::workflow::{Pipeline, StageKind, WorkflowReader};

pub(crate) const PIPELINE_RENEWALS_SECTION: &str = "pipeline_renewals";
const PIPELINE_RENEWAL_KIND: &str = "pipeline_renewal";

fn default_enabled() -> bool {
    true
}

fn default_days() -> u16 {
    DEFAULT_RENEWAL_DAYS_BEFORE
}

fn default_prefix() -> String {
    DEFAULT_RENEWAL_TITLE_PREFIX.to_owned()
}

fn default_owner() -> String {
    RenewalOwnerMode::Original.as_str().to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackagePipelineRenewal {
    pub(crate) pipeline: String,
    #[serde(default = "default_enabled")]
    pub(crate) enabled: bool,
    #[serde(default = "default_days")]
    pub(crate) days_before: u16,
    pub(crate) target_pipeline: String,
    pub(crate) target_stage: String,
    #[serde(default = "default_prefix")]
    pub(crate) title_prefix: String,
    #[serde(default = "default_owner")]
    pub(crate) owner: String,
}

struct KeyedStage {
    id: Uuid,
    key: String,
    kind: StageKind,
    is_active: bool,
}

struct KeyedPipeline {
    pipeline: Pipeline,
    key: String,
    stages: Vec<KeyedStage>,
}

struct Index {
    workflow: Vec<KeyedPipeline>,
}

impl Index {
    fn by_key(&self, key: &str) -> Option<&KeyedPipeline> {
        self.workflow.iter().find(|pipeline| pipeline.key == key)
    }

    fn by_id(&self, id: Uuid) -> Option<&KeyedPipeline> {
        self.workflow
            .iter()
            .find(|pipeline| pipeline.pipeline.id == id)
    }
}

fn snapshot(entry: &PackagePipelineRenewal) -> Vec<(String, Value)> {
    vec![
        ("enabled".to_owned(), json!(entry.enabled)),
        ("days_before".to_owned(), json!(entry.days_before)),
        ("target_pipeline".to_owned(), json!(entry.target_pipeline)),
        ("target_stage".to_owned(), json!(entry.target_stage)),
        ("title_prefix".to_owned(), json!(entry.title_prefix)),
        ("owner".to_owned(), json!(entry.owner)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackagePipelineRenewal>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

pub(crate) struct PipelineRenewalsSection {
    renewals: RenewalStore,
    pool: Db,
    keys: ConfigKeyClient,
}

pub(crate) fn pipeline_renewals_section(pool: &Db) -> PipelineRenewalsSection {
    PipelineRenewalsSection {
        renewals: RenewalStore::new(pool.clone()),
        pool: pool.clone(),
        keys: ConfigKeyClient::new(pool.clone()),
    }
}

impl PipelineRenewalsSection {
    async fn index(&self) -> Result<Index, AppError> {
        let stored_stages = self
            .keys
            .keys(ConfigReferenceKind::new(
                aphel_contract_crm::keys::PIPELINE_STAGE,
            ))
            .await?;
        let reader = WorkflowReader::new(
            self.pool.clone(),
            aphel_sdk::platform::modules::app().services(),
        )?;
        let pipelines = reader.list_pipelines().await?;
        let stored = self
            .keys
            .keys(ConfigReferenceKind::new(aphel_contract_crm::keys::PIPELINE))
            .await?;
        let keyed = assign_keys(
            pipelines
                .iter()
                .map(|pipeline| (pipeline.id, pipeline.name.as_str())),
            &stored,
        );
        let mut workflow = Vec::new();
        for (pipeline, keyed) in pipelines.into_iter().zip(keyed) {
            let stages = reader.list_stages(pipeline.id).await?;
            let ids: BTreeSet<Uuid> = stages.iter().map(|stage| stage.id).collect();
            let stored: HashMap<Uuid, String> = stored_stages
                .iter()
                .filter(|(id, _)| ids.contains(id))
                .map(|(id, key)| (*id, key.clone()))
                .collect();
            let keys = assign_keys(
                stages.iter().map(|stage| (stage.id, stage.name.as_str())),
                &stored,
            );
            workflow.push(KeyedPipeline {
                stages: stages
                    .iter()
                    .zip(keys)
                    .map(|(stage, keyed)| KeyedStage {
                        id: stage.id,
                        key: keyed.key,
                        kind: stage.kind,
                        is_active: stage.is_active,
                    })
                    .collect(),
                key: keyed.key,
                pipeline,
            });
        }
        Ok(Index { workflow })
    }

    fn portable(
        &self,
        index: &Index,
        settings: &RenewalSettings,
    ) -> Option<PackagePipelineRenewal> {
        let source = index.by_id(settings.pipeline_id)?;
        let target = index.by_id(settings.target_pipeline_id)?;
        let stage = target
            .stages
            .iter()
            .find(|stage| stage.id == settings.target_stage_id)?;
        Some(PackagePipelineRenewal {
            pipeline: source.key.clone(),
            enabled: settings.is_enabled,
            days_before: settings.days_before,
            target_pipeline: target.key.clone(),
            target_stage: stage.key.clone(),
            title_prefix: settings.title_prefix.clone(),
            owner: settings.owner_mode.as_str().to_owned(),
        })
    }

    async fn plans(
        &self,
        entries: &[PackagePipelineRenewal],
        index: &Index,
    ) -> Result<Vec<(PlanItem, bool)>, AppError> {
        let existing = self.renewals.list().await?;
        let mut plans = Vec::new();
        for entry in entries {
            let label = index.by_key(&entry.pipeline).map_or_else(
                || entry.pipeline.clone(),
                |source| source.pipeline.name.clone(),
            );
            let item = PlanItem::new(
                PIPELINE_RENEWALS_SECTION,
                PIPELINE_RENEWAL_KIND,
                entry.pipeline.clone(),
                label,
            );
            let current = index.by_key(&entry.pipeline).and_then(|source| {
                existing
                    .iter()
                    .find(|settings| settings.pipeline_id == source.pipeline.id)
            });
            match current {
                None => {
                    let changes = snapshot(entry)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push((item.created(changes), true));
                }
                Some(current) => {
                    let current = self
                        .portable(index, current)
                        .map(|portable| snapshot(&portable))
                        .unwrap_or_default();
                    plans.push((item.compared(diff(&current, &snapshot(entry))), false));
                }
            }
        }
        Ok(plans)
    }
}

#[async_trait]
impl ConfigSection<Tx> for PipelineRenewalsSection {
    fn key(&self) -> &'static str {
        PIPELINE_RENEWALS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let settings = self.renewals.list().await?;
        if settings.is_empty() {
            return Ok(None);
        }
        let index = self.index().await?;
        let entries: Vec<PackagePipelineRenewal> = settings
            .iter()
            .filter_map(|settings| self.portable(&index, settings))
            .collect();
        Ok(Some(json!(entries)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let entries: Vec<PackagePipelineRenewal> = match parse_section(content) {
            Ok(entries) => entries,
            Err(issue) => return Ok(vec![issue]),
        };
        let packaged: HashMap<String, Vec<(String, String)>> = package
            .section(PIPELINES_SECTION)
            .and_then(|content| parse_pipelines(content).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|pipeline| {
                let stages = pipeline
                    .stages
                    .into_iter()
                    .map(|stage| (stage.key, stage.kind))
                    .collect();
                (pipeline.key, stages)
            })
            .collect();
        let index = self.index().await?;
        let known = |key: &str| packaged.contains_key(key) || index.by_key(key).is_some();
        let mut issues = Vec::new();
        let mut seen = BTreeSet::new();
        for (position, entry) in entries.iter().enumerate() {
            let path = |field: &str| format!("{position}.{field}");
            if !known(&entry.pipeline) {
                issues.push(PackageIssue::new(
                    path("pipeline"),
                    "unknown_pipeline",
                    "must be the key of a pipeline of the package or of the organization",
                ));
            } else if !seen.insert(entry.pipeline.clone()) {
                issues.push(PackageIssue::new(
                    path("pipeline"),
                    "duplicated_key",
                    "is declared more than once",
                ));
            }
            let stage_ok = match packaged.get(&entry.target_pipeline) {
                Some(stages) => stages.iter().any(|(key, kind)| {
                    key == &entry.target_stage && kind == StageKind::Open.as_str()
                }),
                None => match index.by_key(&entry.target_pipeline) {
                    Some(target) => target.stages.iter().any(|stage| {
                        stage.key == entry.target_stage
                            && stage.kind == StageKind::Open
                            && stage.is_active
                    }),
                    None => {
                        issues.push(PackageIssue::new(
                            path("target_pipeline"),
                            "unknown_pipeline",
                            "must be the key of a pipeline of the package or of the organization",
                        ));
                        true
                    }
                },
            };
            if !stage_ok {
                issues.push(PackageIssue::new(
                    path("target_stage"),
                    "unknown_stage",
                    "must be the key of an active open stage of the target pipeline",
                ));
            }
            if entry.days_before > MAX_RENEWAL_DAYS_BEFORE {
                issues.push(PackageIssue::new(
                    path("days_before"),
                    "invalid_days_before",
                    format!("must be between 0 and {MAX_RENEWAL_DAYS_BEFORE}"),
                ));
            }
            if entry.title_prefix.chars().count() > MAX_RENEWAL_TITLE_PREFIX_LENGTH {
                issues.push(PackageIssue::new(
                    path("title_prefix"),
                    "invalid_title_prefix",
                    format!("must have at most {MAX_RENEWAL_TITLE_PREFIX_LENGTH} characters"),
                ));
            }
            if RenewalOwnerMode::parse(&entry.owner).is_none() {
                issues.push(PackageIssue::new(
                    path("owner"),
                    "invalid_owner",
                    "must be original or none",
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let entries = parse(content)?;
        let index = self.index().await?;
        Ok(self
            .plans(&entries, &index)
            .await?
            .into_iter()
            .map(|(item, _)| item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let entries = parse(content)?;
        let index = self.index().await?;
        let plans = self.plans(&entries, &index).await?;
        let existing = self.renewals.list().await?;
        let pipeline_id = |key: &str| {
            context
                .resolved(
                    ConfigReferenceKind::new(aphel_contract_crm::keys::PIPELINE),
                    key,
                )
                .or_else(|| index.by_key(key).map(|pipeline| pipeline.pipeline.id))
        };
        for (entry, (item, is_new)) in entries.iter().zip(plans) {
            if !is_new && !decisions.executes(&item) {
                continue;
            }
            let (Some(source_id), Some(target_id)) = (
                pipeline_id(&entry.pipeline),
                pipeline_id(&entry.target_pipeline),
            ) else {
                continue;
            };
            let Some(stage_id) = context
                .resolved(
                    ConfigReferenceKind::new(aphel_contract_crm::keys::PIPELINE_STAGE),
                    &stage_item_key(&entry.target_pipeline, &entry.target_stage),
                )
                .or_else(|| {
                    index.by_id(target_id).and_then(|target| {
                        target
                            .stages
                            .iter()
                            .find(|stage| stage.key == entry.target_stage)
                            .map(|stage| stage.id)
                    })
                })
            else {
                continue;
            };
            let current = existing
                .iter()
                .find(|settings| settings.pipeline_id == source_id);
            let settings = RenewalSettings {
                pipeline_id: source_id,
                is_enabled: entry.enabled,
                days_before: entry.days_before.min(MAX_RENEWAL_DAYS_BEFORE),
                target_pipeline_id: target_id,
                target_stage_id: stage_id,
                title_prefix: entry.title_prefix.clone(),
                owner_mode: RenewalOwnerMode::parse(&entry.owner).unwrap_or_default(),
                updated_by: Some(scope.user_id().clone()),
                created_at: current.map_or(context.at, |current| current.created_at),
                updated_at: context.at,
            };
            self.renewals.save(tx, &settings).await?;
        }
        Ok(())
    }
}
