use aphel_kernel::application::config_packages::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CriterionInput, DEFAULT_HOT_FROM, DEFAULT_WARM_FROM, MAX_MODEL_NAME_LENGTH, ScoreBands,
    ScoringModel,
};
use crate::engine::is_scored_record_type;
use crate::service::{CriteriaDraft, build_criteria, recalculate_job};
use crate::store::ModelStore;
use aphel_kernel::application::config_packages::sections::common::{check_key, diff};
use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::application::shared::JobQueue;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;

pub(crate) const SCORING_MODELS_SECTION: &str = "scoring_models";
pub(crate) const SCORING_MODEL_ITEM: &str = "scoring_model";

fn default_active() -> bool {
    true
}

fn default_warm_from() -> i32 {
    DEFAULT_WARM_FROM
}

fn default_hot_from() -> i32 {
    DEFAULT_HOT_FROM
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageScoringCriterion {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) filter: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) signal: Option<String>,
    pub(crate) points: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_points: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) decay_days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageScoringModel {
    pub(crate) key: String,
    pub(crate) record_type: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) primary: bool,
    #[serde(default = "default_active")]
    pub(crate) active: bool,
    #[serde(default = "default_warm_from")]
    pub(crate) warm_from: i32,
    #[serde(default = "default_hot_from")]
    pub(crate) hot_from: i32,
    #[serde(default)]
    pub(crate) criteria: Vec<PackageScoringCriterion>,
}

impl PackageScoringCriterion {
    fn input(&self) -> CriterionInput {
        CriterionInput {
            key: Some(self.key.clone()),
            label: self.label.clone(),
            kind: self.kind.clone(),
            filter: self.filter.clone(),
            signal: self.signal.clone(),
            points: self.points,
            max_points: self.max_points,
            decay_days: self.decay_days,
        }
    }
}

struct ModelPlan {
    item: PlanItem,
    model: ScoringModel,
    is_new: bool,
    conflicting: bool,
}

fn snapshot(model: &ScoringModel) -> Vec<(String, Value)> {
    let full = model.snapshot();
    [
        "record_type",
        "name",
        "description",
        "is_primary",
        "is_active",
        "warm_from",
        "hot_from",
        "criteria",
    ]
    .into_iter()
    .map(|property| (property.to_owned(), full[property].clone()))
    .collect()
}

fn parse(content: &Value) -> Result<Vec<PackageScoringModel>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

pub(crate) struct ScoringModelsSection {
    models: ModelStore,
    jobs: MySqlJobQueue,
}

pub(crate) fn scoring_models_section(pool: &sqlx::MySqlPool) -> ScoringModelsSection {
    ScoringModelsSection::new(
        ModelStore::new(pool.clone()),
        MySqlJobQueue::new(pool.clone()),
    )
}

impl ScoringModelsSection {
    pub(crate) fn new(models: ModelStore, jobs: MySqlJobQueue) -> Self {
        Self { models, jobs }
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageScoringModel],
        at: DateTime<Utc>,
    ) -> Result<Vec<ModelPlan>, AppError> {
        let existing = self.models.list(None).await?;
        let mut plans = Vec::new();
        for entry in package {
            let item = PlanItem::new(
                SCORING_MODELS_SECTION,
                SCORING_MODEL_ITEM,
                entry.key.clone(),
                entry.name.trim(),
            );
            let inputs: Vec<CriterionInput> = entry
                .criteria
                .iter()
                .map(PackageScoringCriterion::input)
                .collect();
            let current = existing.iter().find(|model| model.key == entry.key);
            let mut violations = Vec::new();
            let criteria = build_criteria(
                CriteriaDraft {
                    inputs: &inputs,
                    current: current.map_or(&[], |model| model.criteria.as_slice()),
                },
                &mut violations,
            )
            .into_iter()
            .map(|(criterion, _)| criterion)
            .collect::<Vec<_>>();
            let description = entry
                .description
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            let bands = ScoreBands {
                warm_from: entry.warm_from,
                hot_from: entry.hot_from,
            };
            match current {
                None => {
                    let model = ScoringModel {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        record_type: entry.record_type.trim().to_owned(),
                        name: entry.name.trim().to_owned(),
                        description,
                        is_primary: entry.primary,
                        is_active: entry.active,
                        bands,
                        criteria,
                        recalculated_at: None,
                        created_by: Some(scope.user_id().clone()),
                        updated_by: Some(scope.user_id().clone()),
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = snapshot(&model)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(ModelPlan {
                        item: item.created(changes),
                        model,
                        is_new: true,
                        conflicting: false,
                    });
                }
                Some(current) => {
                    let mut criteria: Vec<_> = criteria;
                    let kept: Vec<_> = current
                        .criteria
                        .iter()
                        .filter(|kept| !criteria.iter().any(|criterion| criterion.key == kept.key))
                        .cloned()
                        .collect();
                    criteria.extend(kept);
                    for (position, criterion) in criteria.iter_mut().enumerate() {
                        criterion.position = i32::try_from(position).unwrap_or(i32::MAX);
                    }
                    let updated = ScoringModel {
                        name: entry.name.trim().to_owned(),
                        description,
                        is_primary: entry.primary,
                        is_active: entry.active,
                        bands,
                        criteria,
                        updated_by: Some(scope.user_id().clone()),
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    let conflicting = current.record_type != entry.record_type.trim();
                    let item = if conflicting {
                        item.conflicting(
                            changes,
                            "record_type_mismatch",
                            format!(
                                "the scoring model {} already exists for the record type {}",
                                current.key, current.record_type
                            ),
                            false,
                        )
                    } else {
                        item.compared(changes)
                    };
                    plans.push(ModelPlan {
                        item,
                        model: updated,
                        is_new: false,
                        conflicting,
                    });
                }
            }
        }
        Ok(plans)
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for ScoringModelsSection {
    fn key(&self) -> &'static str {
        SCORING_MODELS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let models = self.models.list(None).await?;
        if models.is_empty() {
            return Ok(None);
        }
        let entries: Vec<PackageScoringModel> = models
            .into_iter()
            .map(|model| PackageScoringModel {
                key: model.key,
                record_type: model.record_type,
                name: model.name,
                description: model.description,
                primary: model.is_primary,
                active: model.is_active,
                warm_from: model.bands.warm_from,
                hot_from: model.bands.hot_from,
                criteria: model
                    .criteria
                    .into_iter()
                    .map(|criterion| PackageScoringCriterion {
                        key: criterion.key,
                        label: criterion.label,
                        kind: criterion.rule.kind().as_str().to_owned(),
                        filter: criterion
                            .rule
                            .filter()
                            .and_then(|filter| serde_json::to_value(filter).ok()),
                        signal: criterion
                            .rule
                            .signal()
                            .map(|signal| signal.as_str().to_owned()),
                        points: i64::from(criterion.points),
                        max_points: criterion.rule.max_points().map(i64::from),
                        decay_days: criterion.rule.decay_days().map(i64::from),
                    })
                    .collect(),
            })
            .collect();
        Ok(Some(json!(entries)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let models: Vec<PackageScoringModel> = match parse_section(content) {
            Ok(models) => models,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        let mut keys = BTreeSet::new();
        let mut primaries = BTreeSet::new();
        for (index, model) in models.iter().enumerate() {
            let path = |field: &str| format!("{index}.{field}");
            check_key(&model.key, &path("key"), &mut keys, &mut issues);
            let record_type = model.record_type.trim();
            if !is_scored_record_type(record_type) {
                issues.push(PackageIssue::new(
                    path("record_type"),
                    "invalid_record_type",
                    "does not support scoring models",
                ));
            }
            if model.primary && !primaries.insert(record_type.to_owned()) {
                issues.push(PackageIssue::new(
                    path("primary"),
                    "duplicated_primary",
                    "only one scoring model can be primary for a record type",
                ));
            }
            let name = model.name.trim().chars().count();
            if name == 0 || name > MAX_MODEL_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    path("name"),
                    "invalid_name",
                    format!("must have between 1 and {MAX_MODEL_NAME_LENGTH} characters"),
                ));
            }
            if model.warm_from >= model.hot_from {
                issues.push(PackageIssue::new(
                    path("hot_from"),
                    "invalid_bands",
                    "must be greater than warm_from",
                ));
            }
            let inputs: Vec<CriterionInput> = model
                .criteria
                .iter()
                .map(PackageScoringCriterion::input)
                .collect();
            let mut violations = Vec::new();
            build_criteria(
                CriteriaDraft {
                    inputs: &inputs,
                    current: &[],
                },
                &mut violations,
            );
            issues.extend(violations.into_iter().map(|violation| {
                PackageIssue::new(
                    path(&violation.field),
                    "invalid_criterion",
                    violation.message,
                )
            }));
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(scope, &parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
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
        for plan in self.reconcile(scope, &parse(content)?, context.at).await? {
            let written = if plan.is_new {
                if plan.model.is_primary {
                    self.models
                        .clear_primary(tx, &plan.model.record_type, plan.model.id)
                        .await?;
                }
                self.models.insert(tx, &plan.model).await?;
                true
            } else if !plan.conflicting && decisions.executes(&plan.item) {
                if plan.model.is_primary {
                    self.models
                        .clear_primary(tx, &plan.model.record_type, plan.model.id)
                        .await?;
                }
                self.models.update(tx, &plan.model).await?;
                true
            } else {
                false
            };
            if written && plan.model.is_active {
                self.jobs
                    .enqueue(tx, recalculate_job(plan.model.id))
                    .await?;
            }
            context.resolve(
                ConfigReferenceKind::new("scoring.model"),
                &plan.model.key,
                plan.model.id,
            );
        }
        Ok(())
    }
}
