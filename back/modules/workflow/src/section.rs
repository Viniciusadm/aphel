use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    MAX_REQUIRED_FIELDS, Pipeline, PipelineStage, StageKind, normalize_name, valid_probability,
};
use crate::rotting::{rotting_days_message, valid_rotting_days};
use crate::store::PipelineStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::sections::fields::{package_fields, projected_field_set};
use aphel_sdk::configuration::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, KeyedItem,
    PackageIssue, PlanAction, PlanChange, PlanItem, UnavailableFields, assign_keys,
};
use aphel_sdk::platform::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;

pub use aphel_sdk::workflow::configuration::{PIPELINES_SECTION, stage_item_key};
const PIPELINE_KIND: &str = "pipeline";
const STAGE_KIND: &str = "stage";
pub(crate) const MAX_PACKAGE_PIPELINES: usize = 50;
pub(crate) const MAX_PACKAGE_STAGES: usize = 50;

pub(crate) const STAGE_KIND_MISMATCH: &str = "stage_kind_mismatch";

pub use aphel_sdk::workflow::configuration::{PackagePipeline, PackageStage, parse_pipelines};

pub fn package_pipeline_keys(package: &ConfigPackage) -> Vec<String> {
    package
        .section(PIPELINES_SECTION)
        .and_then(|content| parse_pipelines(content).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|pipeline| pipeline.key)
        .collect()
}

pub async fn keyed_pipelines<K>(
    workflow: &PipelineStore,
    keys: &K,
) -> Result<Vec<(Pipeline, KeyedItem)>, AppError>
where
    K: ConfigKeyRepository,
{
    let all = workflow.list_pipelines().await?;
    let stored = keys
        .keys(ConfigReferenceKind::new("workflow.pipeline"))
        .await?;
    let keyed = assign_keys(
        all.iter()
            .map(|pipeline| (pipeline.id, pipeline.name.as_str())),
        &stored,
    );
    Ok(all.into_iter().zip(keyed).collect())
}

async fn keyed_stages<K>(
    workflow: &PipelineStore,
    keys: &K,
    pipeline_id: Uuid,
) -> Result<Vec<(PipelineStage, KeyedItem)>, AppError>
where
    K: ConfigKeyRepository,
{
    let stages = workflow.list_stages(pipeline_id).await?;
    let ids: BTreeSet<Uuid> = stages.iter().map(|stage| stage.id).collect();
    let stored: HashMap<Uuid, String> = keys
        .keys(ConfigReferenceKind::new("workflow.stage"))
        .await?
        .into_iter()
        .filter(|(id, _)| ids.contains(id))
        .collect();
    let keyed = assign_keys(
        stages.iter().map(|stage| (stage.id, stage.name.as_str())),
        &stored,
    );
    Ok(stages.into_iter().zip(keyed).collect())
}

struct StagePlan {
    item: PlanItem,
    stage: PipelineStage,
    is_new: bool,
    key: String,
}

struct PipelinePlan {
    item: PlanItem,
    pipeline: Pipeline,
    is_new: bool,
    key: String,
    stages: Vec<StagePlan>,
    order: Option<Vec<Uuid>>,
}

pub(crate) struct PipelinesSection<K, D> {
    registry: Arc<RecordTypeRegistry>,
    workflow: PipelineStore,
    keys: K,
    definitions: D,
}

impl<K, D> PipelinesSection<K, D>
where
    K: ConfigKeyRepository,
    D: FieldDefinitionRepository,
{
    pub(crate) fn new(
        registry: Arc<RecordTypeRegistry>,
        workflow: PipelineStore,
        keys: K,
        definitions: D,
    ) -> Self {
        Self {
            registry,
            workflow,
            keys,
            definitions,
        }
    }

    async fn reconcile(
        &self,
        package: &[PackagePipeline],
        at: DateTime<Utc>,
    ) -> Result<Vec<PipelinePlan>, AppError> {
        let existing = keyed_pipelines(&self.workflow, &self.keys).await?;
        let last_position = existing
            .iter()
            .map(|(pipeline, _)| pipeline.position)
            .max()
            .unwrap_or(-1);
        let mut used: BTreeSet<Uuid> = BTreeSet::new();
        let mut plans = Vec::new();
        for entry in package {
            let name = normalize_name(&entry.name).unwrap_or_else(|| entry.name.clone());
            let item = PlanItem::new(
                PIPELINES_SECTION,
                PIPELINE_KIND,
                entry.key.clone(),
                name.clone(),
            );
            let matched = existing
                .iter()
                .find(|(pipeline, keyed)| keyed.key == entry.key && !used.contains(&pipeline.id))
                .or_else(|| {
                    entry.default.then(|| {
                        existing.iter().find(|(pipeline, keyed)| {
                            pipeline.is_default && !keyed.explicit && !used.contains(&pipeline.id)
                        })
                    })?
                });
            let Some((current, _)) = matched else {
                let created_before = plans
                    .iter()
                    .filter(|plan: &&PipelinePlan| plan.is_new)
                    .count();
                let becomes_default = entry.default
                    && !existing.iter().any(|(pipeline, _)| pipeline.is_default)
                    && !plans.iter().any(|plan| plan.pipeline.is_default);
                let pipeline = Pipeline {
                    id: Uuid::now_v7(),
                    name,
                    is_default: becomes_default,
                    position: last_position + 1 + created_before as i32,
                    is_active: true,
                    team_ids: Vec::new(),
                    created_at: at,
                    updated_at: at,
                };
                let stages = self.stage_plans(entry, pipeline.id, Vec::new(), at);
                let item = item.created(vec![
                    PlanChange::new("name", Value::Null, json!(pipeline.name)),
                    PlanChange::new(
                        "stages",
                        Value::Null,
                        json!(
                            entry
                                .stages
                                .iter()
                                .map(|stage| stage.name.clone())
                                .collect::<Vec<_>>()
                        ),
                    ),
                ]);
                plans.push(PipelinePlan {
                    item,
                    pipeline,
                    is_new: true,
                    key: entry.key.clone(),
                    stages,
                    order: None,
                });
                continue;
            };
            used.insert(current.id);
            let current_stages = keyed_stages(&self.workflow, &self.keys, current.id).await?;
            let stages = self.stage_plans(entry, current.id, current_stages.clone(), at);
            let order = stage_order(&current_stages, &stages);
            let mut updated = current.clone();
            updated.name = name;
            updated.is_active = true;
            let mut changes = Vec::new();
            if updated.name != current.name {
                changes.push(PlanChange::new(
                    "name",
                    json!(current.name),
                    json!(updated.name),
                ));
            }
            if !current.is_active {
                changes.push(PlanChange::new("active", json!(false), json!(true)));
            }
            if let Some(order) = &order {
                let name_of = |id: &Uuid| {
                    stages
                        .iter()
                        .map(|plan| &plan.stage)
                        .chain(current_stages.iter().map(|(stage, _)| stage))
                        .find(|stage| &stage.id == id)
                        .map(|stage| stage.name.clone())
                        .unwrap_or_default()
                };
                changes.push(PlanChange::new(
                    "stage_order",
                    json!(
                        current_stages
                            .iter()
                            .map(|(stage, _)| stage.name.clone())
                            .collect::<Vec<_>>()
                    ),
                    json!(order.iter().map(name_of).collect::<Vec<_>>()),
                ));
            }
            plans.push(PipelinePlan {
                item: item.compared(changes),
                pipeline: updated,
                is_new: false,
                key: entry.key.clone(),
                stages,
                order,
            });
        }
        Ok(plans)
    }

    fn stage_plans(
        &self,
        entry: &PackagePipeline,
        pipeline_id: Uuid,
        current: Vec<(PipelineStage, KeyedItem)>,
        at: DateTime<Utc>,
    ) -> Vec<StagePlan> {
        let mut plans = Vec::new();
        for (index, stage) in entry.stages.iter().enumerate() {
            let name = normalize_name(&stage.name).unwrap_or_else(|| stage.name.clone());
            let item = PlanItem::new(
                PIPELINES_SECTION,
                STAGE_KIND,
                format!("{}.{}", entry.key, stage.key),
                name.clone(),
            );
            let kind = StageKind::parse(&stage.kind).unwrap_or(StageKind::Open);
            let probability = valid_probability(stage.probability).unwrap_or(0);
            let rotting_days = stage.rotting_days.and_then(valid_rotting_days);
            let entry_fields = stage.entry_required_fields.clone();
            let exit_fields = stage.exit_required_fields.clone();
            match current.iter().find(|(_, keyed)| keyed.key == stage.key) {
                None => {
                    let created = PipelineStage {
                        id: Uuid::now_v7(),
                        pipeline_id,
                        name,
                        position: index as i32,
                        probability,
                        rotting_days: rotting_days.filter(|_| kind == StageKind::Open),
                        kind,
                        is_active: true,
                        entry_required_fields: entry_fields,
                        exit_required_fields: exit_fields,
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = stage_snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(StagePlan {
                        item: item.created(changes),
                        stage: created,
                        is_new: true,
                        key: stage.key.clone(),
                    });
                }
                Some((existing, _)) => {
                    let mut updated = existing.clone();
                    updated.name = name;
                    updated.probability = probability;
                    updated.kind = kind;
                    if rotting_days.is_some() {
                        updated.rotting_days = rotting_days;
                    }
                    if kind != StageKind::Open {
                        updated.rotting_days = None;
                    }
                    updated.is_active = true;
                    updated.entry_required_fields = entry_fields;
                    updated.exit_required_fields = exit_fields;
                    updated.updated_at = at;
                    let changes = diff(&stage_snapshot(existing), &stage_snapshot(&updated));
                    let item = if existing.kind != kind {
                        item.conflicting(
                            changes,
                            STAGE_KIND_MISMATCH,
                            format!(
                                "the organization already has this stage as {} and the package defines it as {}",
                                existing.kind.as_str(),
                                kind.as_str()
                            ),
                            false,
                        )
                    } else {
                        item.compared(changes)
                    };
                    plans.push(StagePlan {
                        item,
                        stage: updated,
                        is_new: false,
                        key: stage.key.clone(),
                    });
                }
            }
        }
        plans
    }

    async fn record_field_keys(
        &self,
        package: &ConfigPackage,
    ) -> Result<BTreeSet<String>, AppError> {
        let mut keys = BTreeSet::new();
        for definition in self
            .registry
            .definitions()
            .filter(|definition| definition.capabilities.has_trait("workflow.staged"))
        {
            let existing = self.definitions.list(&definition.record_type).await?;
            let fields = projected_field_set(definition, existing, &package_fields(package));
            keys.extend(fields.active().map(|field| field.key().to_owned()));
        }
        Ok(keys)
    }
}

fn stage_order(current: &[(PipelineStage, KeyedItem)], plans: &[StagePlan]) -> Option<Vec<Uuid>> {
    let package_ids: Vec<Uuid> = plans.iter().map(|plan| plan.stage.id).collect();
    let mut order = package_ids.clone();
    let mut anchor: Option<Uuid> = None;
    for (stage, _) in current {
        if package_ids.contains(&stage.id) {
            anchor = Some(stage.id);
            continue;
        }
        let index = anchor
            .and_then(|anchor| order.iter().position(|id| *id == anchor))
            .map_or(0, |index| index + 1);
        order.insert(index, stage.id);
        anchor = Some(stage.id);
    }
    let current_ids: Vec<Uuid> = current.iter().map(|(stage, _)| stage.id).collect();
    (order != current_ids).then_some(order)
}

fn stage_snapshot(stage: &PipelineStage) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(stage.name)),
        ("kind".to_owned(), json!(stage.kind.as_str())),
        ("probability".to_owned(), json!(stage.probability)),
        ("rotting_days".to_owned(), json!(stage.rotting_days)),
        ("active".to_owned(), json!(stage.is_active)),
        (
            "entry_required_fields".to_owned(),
            json!(stage.entry_required_fields),
        ),
        (
            "exit_required_fields".to_owned(),
            json!(stage.exit_required_fields),
        ),
    ]
}

#[async_trait]
impl<K, D> ConfigSection<Tx> for PipelinesSection<K, D>
where
    K: ConfigKeyRepository<Tx = Tx>,
    D: FieldDefinitionRepository,
{
    fn key(&self) -> &'static str {
        PIPELINES_SECTION
    }

    fn prune(&self, content: &mut Value, unavailable: &UnavailableFields) -> Vec<String> {
        let mut removed = Vec::new();
        let stages = content
            .as_array_mut()
            .into_iter()
            .flatten()
            .filter_map(|pipeline| pipeline.get_mut("stages").and_then(Value::as_array_mut))
            .flatten();
        for stage in stages {
            for property in ["entry_required_fields", "exit_required_fields"] {
                let Some(keys) = stage.get_mut(property).and_then(Value::as_array_mut) else {
                    continue;
                };
                keys.retain(|key| match key.as_str() {
                    Some(key)
                        if self
                            .registry
                            .definitions()
                            .filter(|definition| {
                                definition.capabilities.has_trait("workflow.staged")
                            })
                            .any(|definition| {
                                unavailable.contains(definition.record_type.as_str(), key)
                            }) =>
                    {
                        removed.push(key.to_owned());
                        false
                    }
                    _ => true,
                });
            }
        }
        removed
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut workflow = Vec::new();
        for (pipeline, keyed) in keyed_pipelines(&self.workflow, &self.keys).await? {
            if !pipeline.is_active {
                continue;
            }
            let stages = keyed_stages(&self.workflow, &self.keys, pipeline.id)
                .await?
                .into_iter()
                .filter(|(stage, _)| stage.is_active)
                .map(|(stage, keyed)| PackageStage {
                    key: keyed.key,
                    name: stage.name,
                    kind: stage.kind.as_str().to_owned(),
                    probability: i64::from(stage.probability),
                    rotting_days: stage.rotting_days.map(i64::from),
                    entry_required_fields: stage.entry_required_fields,
                    exit_required_fields: stage.exit_required_fields,
                })
                .collect();
            workflow.push(PackagePipeline {
                key: keyed.key,
                name: pipeline.name,
                default: pipeline.is_default,
                stages,
            });
        }
        Ok(Some(
            serde_json::to_value(workflow)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        ))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let workflow = match parse_pipelines(content) {
            Ok(workflow) => workflow,
            Err(issue) => return Ok(vec![issue]),
        };
        let record_fields = self.record_field_keys(package).await?;
        let mut issues = Vec::new();
        let mut pipeline_keys = BTreeSet::new();
        if workflow.iter().filter(|pipeline| pipeline.default).count() > 1 {
            issues.push(PackageIssue::new(
                "",
                "multiple_defaults",
                "only one pipeline can be the default",
            ));
        }
        if workflow.len() > MAX_PACKAGE_PIPELINES {
            issues.push(PackageIssue::new(
                "",
                "too_many_pipelines",
                format!("a package can have at most {MAX_PACKAGE_PIPELINES} workflow"),
            ));
        }
        for (index, pipeline) in workflow.iter().enumerate() {
            let path = index.to_string();
            check_key(
                &pipeline.key,
                &format!("{path}.key"),
                &mut pipeline_keys,
                &mut issues,
            );
            if normalize_name(&pipeline.name).is_none() {
                issues.push(PackageIssue::new(
                    format!("{path}.name"),
                    "invalid_name",
                    "must have between 1 and 140 characters",
                ));
            }
            if pipeline.stages.len() > MAX_PACKAGE_STAGES {
                issues.push(PackageIssue::new(
                    format!("{path}.stages"),
                    "too_many_stages",
                    format!("a pipeline can have at most {MAX_PACKAGE_STAGES} stages"),
                ));
            }
            let kinds: BTreeSet<&str> = pipeline
                .stages
                .iter()
                .map(|stage| stage.kind.as_str())
                .collect();
            if !["open", "won", "lost"]
                .iter()
                .all(|kind| kinds.contains(kind))
            {
                issues.push(PackageIssue::new(
                    format!("{path}.stages"),
                    "missing_closing_stages",
                    "a pipeline needs at least one open, one won and one lost stage",
                ));
            }
            let mut stage_keys = BTreeSet::new();
            for (stage_index, stage) in pipeline.stages.iter().enumerate() {
                let stage_path = format!("{path}.stages.{stage_index}");
                check_key(
                    &stage.key,
                    &format!("{stage_path}.key"),
                    &mut stage_keys,
                    &mut issues,
                );
                if normalize_name(&stage.name).is_none() {
                    issues.push(PackageIssue::new(
                        format!("{stage_path}.name"),
                        "invalid_name",
                        "must have between 1 and 140 characters",
                    ));
                }
                if StageKind::parse(&stage.kind).is_none() {
                    issues.push(PackageIssue::new(
                        format!("{stage_path}.kind"),
                        "invalid_stage_kind",
                        "must be open, won or lost",
                    ));
                }
                if valid_probability(stage.probability).is_none() {
                    issues.push(PackageIssue::new(
                        format!("{stage_path}.probability"),
                        "invalid_probability",
                        "must be between 0 and 100",
                    ));
                }
                if stage
                    .rotting_days
                    .is_some_and(|days| valid_rotting_days(days).is_none())
                {
                    issues.push(PackageIssue::new(
                        format!("{stage_path}.rotting_days"),
                        "invalid_rotting_days",
                        rotting_days_message(),
                    ));
                }
                for (property, keys) in [
                    ("entry_required_fields", &stage.entry_required_fields),
                    ("exit_required_fields", &stage.exit_required_fields),
                ] {
                    if keys.len() > MAX_REQUIRED_FIELDS {
                        issues.push(PackageIssue::new(
                            format!("{stage_path}.{property}"),
                            "too_many_required_fields",
                            format!("at most {MAX_REQUIRED_FIELDS} fields"),
                        ));
                    }
                    for (key_index, key) in keys.iter().enumerate() {
                        if !record_fields.contains(key) {
                            issues.push(PackageIssue::new(
                                format!("{stage_path}.{property}.{key_index}"),
                                "unknown_field",
                                format!("record field {key} exists neither in the package nor in the organization"),
                            ));
                        }
                    }
                }
            }
        }
        let existing = keyed_pipelines(&self.workflow, &self.keys).await?;
        let created = workflow
            .iter()
            .filter(|pipeline| !existing.iter().any(|(_, keyed)| keyed.key == pipeline.key))
            .count();
        if existing.len() + created > MAX_PACKAGE_PIPELINES {
            issues.push(PackageIssue::new(
                "",
                "too_many_pipelines",
                format!("the organization would have more than {MAX_PACKAGE_PIPELINES} workflow"),
            ));
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let workflow =
            parse_pipelines(content).map_err(|issue| AppError::Validation(issue.message))?;
        let mut items = Vec::new();
        for plan in self
            .reconcile(&workflow, DateTime::<Utc>::UNIX_EPOCH)
            .await?
        {
            items.push(plan.item);
            items.extend(plan.stages.into_iter().map(|stage| stage.item));
        }
        Ok(items)
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let workflow =
            parse_pipelines(content).map_err(|issue| AppError::Validation(issue.message))?;
        for plan in self.reconcile(&workflow, context.at).await? {
            let pipeline_id = plan.pipeline.id;
            if plan.is_new {
                self.workflow.insert_pipeline(tx, &plan.pipeline).await?;
            } else if plan.item.action == PlanAction::Update
                && plan
                    .item
                    .changes
                    .iter()
                    .any(|change| change.property != "stage_order")
            {
                self.workflow.update_pipeline(tx, &plan.pipeline).await?;
            }
            self.keys
                .assign(
                    tx,
                    ConfigReferenceKind::new("workflow.pipeline"),
                    pipeline_id,
                    &plan.key,
                )
                .await?;
            context.resolve(
                ConfigReferenceKind::new("workflow.pipeline"),
                &plan.key,
                pipeline_id,
            );
            for stage in &plan.stages {
                if stage.is_new {
                    self.workflow.insert_stage(tx, &stage.stage).await?;
                } else if decisions.executes(&stage.item) {
                    self.workflow.update_stage(tx, &stage.stage).await?;
                }
                if stage.item.action != PlanAction::SkipConflict || decisions.executes(&stage.item)
                {
                    self.keys
                        .assign(
                            tx,
                            ConfigReferenceKind::new("workflow.stage"),
                            stage.stage.id,
                            &stage.key,
                        )
                        .await?;
                    context.resolve(
                        ConfigReferenceKind::new("workflow.stage"),
                        &stage_item_key(&plan.key, &stage.key),
                        stage.stage.id,
                    );
                }
            }
            if let Some(order) = &plan.order {
                self.workflow.reorder_stages(tx, pipeline_id, order).await?;
            }
        }
        Ok(())
    }
}
