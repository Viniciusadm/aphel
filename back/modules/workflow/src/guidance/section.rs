use ::config_packages::application::ConfigReferenceKind;
use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::guidance::domain::{
    ChecklistItem, GuidanceText, MAX_CHECKLIST_ITEMS, MAX_CHECKLIST_LABEL_LENGTH,
    MAX_GUIDANCE_LENGTH,
};
use crate::guidance::store::GuidanceStore;
use crate::section::{PIPELINES_SECTION, keyed_pipelines, parse_pipelines, stage_item_key};
use crate::store::PipelineStore;
use ::config_packages::application::sections::common::{check_key, diff};
use ::config_packages::application::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, PackageIssue,
    PlanChange, PlanItem, assign_keys, parse_section,
};
use ::config_packages::infrastructure::MySqlConfigKeyRepository;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const STAGE_GUIDANCE_SECTION: &str = "stage_guidance";
const STAGE_GUIDANCE_KIND: &str = "stage_guidance";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageChecklistItem {
    pub(crate) key: String,
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageStageGuidance {
    pub(crate) pipeline: String,
    pub(crate) stage: String,
    #[serde(default)]
    pub(crate) guidance: String,
    #[serde(default)]
    pub(crate) items: Vec<PackageChecklistItem>,
}

struct KeyedStage {
    id: Uuid,
    key: String,
    name: String,
}

struct KeyedPipeline {
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

    fn stage(&self, pipeline: &str, stage: &str) -> Option<&KeyedStage> {
        self.by_key(pipeline)?
            .stages
            .iter()
            .find(|candidate| candidate.key == stage)
    }
}

fn snapshot(entry: &PackageStageGuidance) -> Vec<(String, Value)> {
    vec![
        ("guidance".to_owned(), json!(entry.guidance)),
        ("items".to_owned(), json!(entry.items)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageStageGuidance>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

pub(crate) struct StageGuidanceSection {
    guidance: GuidanceStore,
    workflow: PipelineStore,
    keys: MySqlConfigKeyRepository,
}

pub(crate) fn stage_guidance_section(pool: &sqlx::MySqlPool) -> StageGuidanceSection {
    StageGuidanceSection {
        guidance: GuidanceStore::new(pool.clone()),
        workflow: PipelineStore::new(pool.clone()),
        keys: MySqlConfigKeyRepository::new(pool.clone()),
    }
}

impl StageGuidanceSection {
    async fn index(&self) -> Result<Index, AppError> {
        let stored_stages = self
            .keys
            .keys(ConfigReferenceKind::new("workflow.stage"))
            .await?;
        let mut workflow = Vec::new();
        for (pipeline, keyed) in keyed_pipelines(&self.workflow, &self.keys).await? {
            let stages = self.workflow.list_stages(pipeline.id).await?;
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
                key: keyed.key,
                stages: stages
                    .iter()
                    .zip(keys)
                    .map(|(stage, keyed)| KeyedStage {
                        id: stage.id,
                        key: keyed.key,
                        name: stage.name.clone(),
                    })
                    .collect(),
            });
        }
        Ok(Index { workflow })
    }

    async fn current(
        &self,
        stage_id: Uuid,
    ) -> Result<(Option<GuidanceText>, Vec<ChecklistItem>), AppError> {
        Ok((
            self.guidance.guidance(stage_id).await?,
            self.guidance.items(stage_id).await?,
        ))
    }

    async fn plans(
        &self,
        entries: &[PackageStageGuidance],
        index: &Index,
    ) -> Result<Vec<(PlanItem, bool)>, AppError> {
        let mut plans = Vec::new();
        for entry in entries {
            let stage = index.stage(&entry.pipeline, &entry.stage);
            let label = stage.map_or_else(
                || format!("{} / {}", entry.pipeline, entry.stage),
                |stage| stage.name.clone(),
            );
            let item = PlanItem::new(
                STAGE_GUIDANCE_SECTION,
                STAGE_GUIDANCE_KIND,
                format!("{}.{}", entry.pipeline, entry.stage),
                label,
            );
            let current = match stage {
                Some(stage) => {
                    let (text, items) = self.current(stage.id).await?;
                    (text.is_some() || !items.is_empty()).then(|| PackageStageGuidance {
                        pipeline: entry.pipeline.clone(),
                        stage: entry.stage.clone(),
                        guidance: text.map(|text| text.guidance).unwrap_or_default(),
                        items: items.into_iter().map(portable_item).collect(),
                    })
                }
                None => None,
            };
            match current {
                None => {
                    let changes = snapshot(entry)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push((item.created(changes), true));
                }
                Some(current) => {
                    let merged = merged_entry(&current, entry);
                    plans.push((
                        item.compared(diff(&snapshot(&current), &snapshot(&merged))),
                        false,
                    ));
                }
            }
        }
        Ok(plans)
    }
}

fn portable_item(item: ChecklistItem) -> PackageChecklistItem {
    PackageChecklistItem {
        key: item.key,
        label: item.label,
        required: item.is_required,
    }
}

fn merged_entry(
    current: &PackageStageGuidance,
    entry: &PackageStageGuidance,
) -> PackageStageGuidance {
    let mut items = current.items.clone();
    for item in &entry.items {
        match items.iter_mut().find(|existing| existing.key == item.key) {
            Some(existing) => *existing = item.clone(),
            None => items.push(item.clone()),
        }
    }
    PackageStageGuidance {
        guidance: entry.guidance.clone(),
        items,
        ..current.clone()
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for StageGuidanceSection {
    fn key(&self) -> &'static str {
        STAGE_GUIDANCE_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let texts = self.guidance.list_guidance().await?;
        let items = self.guidance.all_items().await?;
        if texts.is_empty() && items.is_empty() {
            return Ok(None);
        }
        let index = self.index().await?;
        let mut stage_ids: Vec<Uuid> = texts.iter().map(|text| text.stage_id).collect();
        for item in &items {
            if !stage_ids.contains(&item.stage_id) {
                stage_ids.push(item.stage_id);
            }
        }
        let mut entries = Vec::new();
        for pipeline in &index.workflow {
            for stage in &pipeline.stages {
                if !stage_ids.contains(&stage.id) {
                    continue;
                }
                entries.push(PackageStageGuidance {
                    pipeline: pipeline.key.clone(),
                    stage: stage.key.clone(),
                    guidance: texts
                        .iter()
                        .find(|text| text.stage_id == stage.id)
                        .map(|text| text.guidance.clone())
                        .unwrap_or_default(),
                    items: items
                        .iter()
                        .filter(|item| item.stage_id == stage.id)
                        .cloned()
                        .map(portable_item)
                        .collect(),
                });
            }
        }
        if entries.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!(entries)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let entries: Vec<PackageStageGuidance> = match parse_section(content) {
            Ok(entries) => entries,
            Err(issue) => return Ok(vec![issue]),
        };
        let packaged: HashMap<String, Vec<String>> = package
            .section(PIPELINES_SECTION)
            .and_then(|content| parse_pipelines(content).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|pipeline| {
                let stages = pipeline.stages.into_iter().map(|stage| stage.key).collect();
                (pipeline.key, stages)
            })
            .collect();
        let index = self.index().await?;
        let mut issues = Vec::new();
        let mut seen = BTreeSet::new();
        for (position, entry) in entries.iter().enumerate() {
            let path = |field: &str| format!("{position}.{field}");
            let stage_known = match packaged.get(&entry.pipeline) {
                Some(stages) => Some(stages.contains(&entry.stage)),
                None => index
                    .by_key(&entry.pipeline)
                    .map(|_| index.stage(&entry.pipeline, &entry.stage).is_some()),
            };
            match stage_known {
                None => issues.push(PackageIssue::new(
                    path("pipeline"),
                    "unknown_pipeline",
                    "must be the key of a pipeline of the package or of the organization",
                )),
                Some(false) => issues.push(PackageIssue::new(
                    path("stage"),
                    "unknown_stage",
                    "must be the key of a stage of the pipeline",
                )),
                Some(true) => {}
            }
            if !seen.insert((entry.pipeline.clone(), entry.stage.clone())) {
                issues.push(PackageIssue::new(
                    path("stage"),
                    "duplicated_key",
                    "is declared more than once",
                ));
            }
            if entry.guidance.chars().count() > MAX_GUIDANCE_LENGTH {
                issues.push(PackageIssue::new(
                    path("guidance"),
                    "invalid_guidance",
                    format!("must have at most {MAX_GUIDANCE_LENGTH} characters"),
                ));
            }
            if entry.items.len() > MAX_CHECKLIST_ITEMS {
                issues.push(PackageIssue::new(
                    path("items"),
                    "too_many_items",
                    format!("can have at most {MAX_CHECKLIST_ITEMS} items"),
                ));
            }
            let mut keys = BTreeSet::new();
            for (index, item) in entry.items.iter().enumerate() {
                check_key(
                    &item.key,
                    &path(&format!("items.{index}.key")),
                    &mut keys,
                    &mut issues,
                );
                let length = item.label.trim().chars().count();
                if length == 0 || length > MAX_CHECKLIST_LABEL_LENGTH {
                    issues.push(PackageIssue::new(
                        path(&format!("items.{index}.label")),
                        "invalid_label",
                        format!("must have between 1 and {MAX_CHECKLIST_LABEL_LENGTH} characters"),
                    ));
                }
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
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let entries = parse(content)?;
        let index = self.index().await?;
        let plans = self.plans(&entries, &index).await?;
        for (entry, (item, is_new)) in entries.iter().zip(plans) {
            if !is_new && !decisions.executes(&item) {
                continue;
            }
            let Some(stage_id) = context
                .resolved(
                    ConfigReferenceKind::new("workflow.stage"),
                    &stage_item_key(&entry.pipeline, &entry.stage),
                )
                .or_else(|| {
                    index
                        .stage(&entry.pipeline, &entry.stage)
                        .map(|stage| stage.id)
                })
            else {
                continue;
            };
            let (text, existing) = self.current(stage_id).await?;
            self.guidance
                .save_guidance(
                    tx,
                    &GuidanceText {
                        stage_id,
                        guidance: entry.guidance.trim().to_owned(),
                        updated_by: Some(scope.user_id().clone()),
                        created_at: text.as_ref().map_or(context.at, |text| text.created_at),
                        updated_at: context.at,
                    },
                )
                .await?;
            let mut position = existing
                .iter()
                .map(|item| item.position)
                .max()
                .unwrap_or(-1);
            for packaged in &entry.items {
                match existing.iter().find(|current| current.key == packaged.key) {
                    Some(current) => {
                        let updated = ChecklistItem {
                            label: packaged.label.trim().to_owned(),
                            is_required: packaged.required,
                            updated_at: context.at,
                            ..current.clone()
                        };
                        if updated != *current {
                            self.guidance.update_item(tx, &updated).await?;
                        }
                    }
                    None => {
                        position += 1;
                        self.guidance
                            .insert_item(
                                tx,
                                &ChecklistItem {
                                    id: Uuid::now_v7(),
                                    stage_id,
                                    key: packaged.key.clone(),
                                    label: packaged.label.trim().to_owned(),
                                    is_required: packaged.required,
                                    position,
                                    created_at: context.at,
                                    updated_at: context.at,
                                },
                            )
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
