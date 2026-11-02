use aphel_contract_crm::CrmPermissions;
use std::collections::BTreeSet;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::directory::ScoringDirectory;
use crate::domain::{
    BehaviorSignal, CreateScoringModelCommand, CriterionInput, CriterionKind, CriterionRule,
    MAX_BAND_THRESHOLD, MAX_CAP, MAX_CRITERIA, MAX_CRITERION_LABEL_LENGTH, MAX_DECAY_DAYS,
    MAX_MODEL_DESCRIPTION_LENGTH, MAX_MODEL_NAME_LENGTH, MAX_MODELS, MAX_POINTS, RecordScoreView,
    SCORE_FIELD, SCORING_MODEL_CREATED, SCORING_MODEL_DELETED, SCORING_MODEL_NOT_FOUND,
    SCORING_MODEL_RECORD, SCORING_MODEL_UPDATED, SCORING_RECALCULATE_JOB, SCORING_RECORD_NOT_FOUND,
    ScoreBands, ScoringCriterion, ScoringModel, UpdateScoringModelCommand,
};
use crate::engine::is_scored_record_type;
use crate::store::{ModelStore, ScoreStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::config_packages::{is_config_key, key_from_name};
use aphel_kernel::application::fields::EntityDefinition;
use aphel_kernel::application::query::Filter;
use aphel_kernel::application::shared::{FieldChange, Job, JobQueue, RecordRef, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, changes_payload, publish_mutation};

const INVALID_SCORING_MODEL: &str = "invalid scoring model";
const KEY_MESSAGE: &str = "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)";

fn model_not_found() -> AppError {
    AppError::not_found_code(SCORING_MODEL_NOT_FOUND, "scoring model was not found")
}

fn record_not_found() -> AppError {
    AppError::not_found_code(SCORING_RECORD_NOT_FOUND, "record was not found")
}

fn parse_model_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| model_not_found())
}

pub(crate) fn filter_fields(filter: &Filter) -> Vec<&str> {
    fn visit<'a>(filter: &'a Filter, fields: &mut Vec<&'a str>) {
        match filter {
            Filter::Group(group) => {
                for child in &group.conditions {
                    visit(child, fields);
                }
            }
            Filter::Condition(condition) => fields.push(condition.field.as_str()),
            Filter::Related(related) => visit(&related.related.filter, fields),
        }
    }
    let mut fields = Vec::new();
    visit(filter, &mut fields);
    fields
}

pub(crate) fn recalculate_job(model_id: Uuid) -> Job {
    Job::new(
        SCORING_RECALCULATE_JOB,
        json!({ "model_id": model_id.to_string() }),
    )
}

fn threshold(
    value: Option<i64>,
    field: &str,
    fallback: i32,
    violations: &mut Vec<FieldViolation>,
) -> i32 {
    match value {
        None => fallback,
        Some(value) if (-MAX_BAND_THRESHOLD..=MAX_BAND_THRESHOLD).contains(&value) => {
            i32::try_from(value).unwrap_or(fallback)
        }
        Some(_) => {
            violations.push(FieldViolation::new(
                field,
                format!("must be between -{MAX_BAND_THRESHOLD} and {MAX_BAND_THRESHOLD}"),
            ));
            fallback
        }
    }
}

fn bounded(value: i64, min: i64, max: i64) -> Option<i32> {
    (min..=max)
        .contains(&value)
        .then(|| i32::try_from(value).ok())
        .flatten()
}

pub(crate) struct CriteriaDraft<'a> {
    pub(crate) inputs: &'a [CriterionInput],
    pub(crate) current: &'a [ScoringCriterion],
}

pub(crate) fn build_criteria(
    draft: CriteriaDraft<'_>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<(ScoringCriterion, Option<usize>)> {
    if draft.inputs.len() > MAX_CRITERIA {
        violations.push(FieldViolation::new(
            "criteria",
            format!("must have at most {MAX_CRITERIA} criteria"),
        ));
    }
    let mut keys = BTreeSet::new();
    let mut criteria = Vec::new();
    for (index, input) in draft.inputs.iter().enumerate() {
        let path = |field: &str| format!("criteria.{index}.{field}");
        let label = input.label.trim().to_owned();
        if label.is_empty() || label.chars().count() > MAX_CRITERION_LABEL_LENGTH {
            violations.push(FieldViolation::new(
                path("label"),
                format!("must have between 1 and {MAX_CRITERION_LABEL_LENGTH} characters"),
            ));
        }
        let key = match input.key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() => key.to_owned(),
            _ => {
                let base: String = key_from_name(&label).chars().take(56).collect();
                let base = base.trim_end_matches('_').to_owned();
                let mut candidate = base.clone();
                let mut suffix = 2;
                while keys.contains(&candidate)
                    || draft.inputs.iter().any(|other| {
                        other.key.as_deref().map(str::trim) == Some(candidate.as_str())
                    })
                {
                    candidate = format!("{base}_{suffix}");
                    suffix += 1;
                }
                candidate
            }
        };
        if !is_config_key(&key) {
            violations.push(FieldViolation::new(path("key"), KEY_MESSAGE));
        } else if !keys.insert(key.clone()) {
            violations.push(FieldViolation::new(
                path("key"),
                "is used by another criterion of the model",
            ));
        }
        let points = bounded(input.points, -MAX_POINTS, MAX_POINTS).filter(|points| *points != 0);
        if points.is_none() {
            violations.push(FieldViolation::new(
                path("points"),
                format!("must be a non-zero integer between -{MAX_POINTS} and {MAX_POINTS}"),
            ));
        }
        let rule = match CriterionKind::parse(input.kind.trim()) {
            Some(CriterionKind::Profile) => {
                if input.signal.is_some()
                    || input.max_points.is_some()
                    || input.decay_days.is_some()
                {
                    violations.push(FieldViolation::new(
                        path("kind"),
                        "profile criteria take only a filter and points",
                    ));
                }
                match &input.filter {
                    Some(value) if !value.is_null() => {
                        match serde_json::from_value::<Filter>(value.clone()) {
                            Ok(filter) => {
                                if filter_fields(&filter).contains(&SCORE_FIELD) {
                                    violations.push(FieldViolation::new(
                                        path("filter"),
                                        "cannot use the score field",
                                    ));
                                }
                                Some(CriterionRule::Profile { filter })
                            }
                            Err(_) => {
                                violations.push(FieldViolation::new(
                                    path("filter"),
                                    "must be a valid filter",
                                ));
                                None
                            }
                        }
                    }
                    _ => {
                        violations.push(FieldViolation::new(
                            path("filter"),
                            "is required for profile criteria",
                        ));
                        None
                    }
                }
            }
            Some(CriterionKind::Behavior) => {
                if input.filter.as_ref().is_some_and(|value| !value.is_null()) {
                    violations.push(FieldViolation::new(
                        path("filter"),
                        "behavior criteria do not take a filter",
                    ));
                }
                let signal = input
                    .signal
                    .as_deref()
                    .map(str::trim)
                    .and_then(BehaviorSignal::parse);
                if signal.is_none() {
                    violations.push(FieldViolation::new(
                        path("signal"),
                        format!("must be one of {}", BehaviorSignal::values()),
                    ));
                }
                let max_points = match input.max_points {
                    None => None,
                    Some(value) => {
                        let cap = bounded(value, 1, MAX_CAP);
                        if cap.is_none() {
                            violations.push(FieldViolation::new(
                                path("max_points"),
                                format!("must be between 1 and {MAX_CAP}"),
                            ));
                        }
                        cap
                    }
                };
                let decay_days = match input.decay_days {
                    None => None,
                    Some(value) => {
                        let days = bounded(value, 1, MAX_DECAY_DAYS);
                        if days.is_none() {
                            violations.push(FieldViolation::new(
                                path("decay_days"),
                                format!("must be between 1 and {MAX_DECAY_DAYS}"),
                            ));
                        }
                        days
                    }
                };
                signal.map(|signal| CriterionRule::Behavior {
                    signal,
                    max_points,
                    decay_days,
                })
            }
            None => {
                violations.push(FieldViolation::new(
                    path("kind"),
                    "must be profile or behavior",
                ));
                None
            }
        };
        if let (Some(rule), Some(points)) = (rule, points) {
            let id = draft
                .current
                .iter()
                .find(|current| current.key == key)
                .map_or_else(Uuid::now_v7, |current| current.id);
            criteria.push((
                ScoringCriterion {
                    id,
                    key,
                    label,
                    points,
                    rule,
                    position: i32::try_from(index).unwrap_or(i32::MAX),
                },
                Some(index),
            ));
        }
    }
    criteria
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
) -> Result<Vec<ScoringModel>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    let record_type = record_type.map(str::trim).filter(|value| !value.is_empty());
    ModelStore::of(ctx).list(record_type).await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<ScoringModel, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    require(ctx, id).await
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateScoringModelCommand,
) -> Result<ScoringModel, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    let record_type = command.record_type.trim().to_owned();
    let definition = ctx.registry().require(&record_type)?.clone();
    let mut violations = Vec::new();
    if !is_scored_record_type(definition.record_type.as_str()) {
        violations.push(FieldViolation::new(
            "record_type",
            "does not support scoring models",
        ));
    }
    let existing = ModelStore::of(ctx).list(None).await?;
    if existing.len() >= MAX_MODELS {
        violations.push(FieldViolation::new(
            "record_type",
            format!("the organization can have at most {MAX_MODELS} scoring models"),
        ));
    }
    let key = match command.key.as_deref().map(str::trim) {
        Some(key) if !key.is_empty() => key.to_owned(),
        _ => free_key(&existing, &key_from_name(&command.name)),
    };
    if !is_config_key(&key) {
        violations.push(FieldViolation::new("key", KEY_MESSAGE));
    } else if existing.iter().any(|model| model.key == key) {
        violations.push(FieldViolation::new(
            "key",
            "is already used by another scoring model",
        ));
    }
    let defaults = ScoreBands::default();
    let bands = ScoreBands {
        warm_from: threshold(
            command.warm_from,
            "warm_from",
            defaults.warm_from,
            &mut violations,
        ),
        hot_from: threshold(
            command.hot_from,
            "hot_from",
            defaults.hot_from,
            &mut violations,
        ),
    };
    let criteria = build_criteria(
        CriteriaDraft {
            inputs: &command.criteria,
            current: &[],
        },
        &mut violations,
    );
    let now = ctx.now();
    let is_primary = command.is_primary.unwrap_or_else(|| {
        !existing
            .iter()
            .any(|model| model.record_type == definition.record_type.as_str() && model.is_primary)
    });
    let model = ScoringModel {
        id: Uuid::now_v7(),
        key,
        record_type: definition.record_type.as_str().to_owned(),
        name: command.name.trim().to_owned(),
        description: command
            .description
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        is_primary,
        is_active: command.is_active.unwrap_or(true),
        bands,
        criteria: Vec::new(),
        recalculated_at: None,
        created_by: Some(scope.user_id().clone()),
        updated_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let model = validated(ctx, &definition, model, criteria, violations).await?;
    let changes = diff_values(&Value::Null, &model.snapshot(), &DiffOptions::default())?;
    let mut tx = ctx.begin().await?;
    if model.is_primary {
        ModelStore::of(ctx)
            .clear_primary(&mut tx, &model.record_type, model.id)
            .await?;
    }
    ModelStore::of(ctx).insert(&mut tx, &model).await?;
    publish(
        ctx,
        &mut tx,
        &model,
        SCORING_MODEL_CREATED,
        changes,
        command.ip,
    )
    .await?;
    if model.is_active {
        ctx.jobs()
            .enqueue(&mut tx, recalculate_job(model.id))
            .await?;
    }
    tx.commit().await?;
    Ok(model)
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateScoringModelCommand,
) -> Result<ScoringModel, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    let current = require(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let definition = ctx.registry().require(&current.record_type)?.clone();
    let mut violations = Vec::new();
    let mut model = current.clone();
    if let Some(name) = command.name {
        model.name = name.trim().to_owned();
    }
    if let Some(description) = command.description {
        model.description = description
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
    }
    if let Some(is_primary) = command.is_primary {
        model.is_primary = is_primary;
    }
    if let Some(is_active) = command.is_active {
        model.is_active = is_active;
    }
    model.bands = ScoreBands {
        warm_from: threshold(
            command.warm_from,
            "warm_from",
            current.bands.warm_from,
            &mut violations,
        ),
        hot_from: threshold(
            command.hot_from,
            "hot_from",
            current.bands.hot_from,
            &mut violations,
        ),
    };
    let criteria = match &command.criteria {
        Some(inputs) => build_criteria(
            CriteriaDraft {
                inputs,
                current: &current.criteria,
            },
            &mut violations,
        ),
        None => current
            .criteria
            .iter()
            .cloned()
            .map(|criterion| (criterion, None))
            .collect(),
    };
    let mut model = validated(ctx, &definition, model, criteria, violations).await?;
    let changes = diff_values(
        &current.snapshot(),
        &model.snapshot(),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    model.updated_by = Some(scope.user_id().clone());
    model.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    if model.is_primary && !current.is_primary {
        ModelStore::of(ctx)
            .clear_primary(&mut tx, &model.record_type, model.id)
            .await?;
    }
    ModelStore::of(ctx).update(&mut tx, &model).await?;
    publish(
        ctx,
        &mut tx,
        &model,
        SCORING_MODEL_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    let rescore = changes_affect_scores(&current, &model);
    if model.is_active && rescore {
        ctx.jobs()
            .enqueue(&mut tx, recalculate_job(model.id))
            .await?;
    }
    tx.commit().await?;
    Ok(model)
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    let model = require(ctx, id).await?;
    let changes = diff_values(&model.snapshot(), &Value::Null, &DiffOptions::default())?;
    let mut tx = ctx.begin().await?;
    ModelStore::of(ctx).delete(&mut tx, model.id).await?;
    publish(ctx, &mut tx, &model, SCORING_MODEL_DELETED, changes, ip).await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn recalculate(ctx: &Ctx<'_>, id: &str) -> Result<(), AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SCORING_MANAGE)?;
    let model = require(ctx, id).await?;
    ctx.jobs().enqueue_now(recalculate_job(model.id)).await?;
    Ok(())
}

pub(crate) async fn record_scores(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<RecordScoreView>, AppError> {
    let scope = ctx.scope;
    let definition = ctx.registry().require(record_type.trim())?;
    scope.require(&definition.read_permission)?;
    let record_id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_found())?;
    if !ScoringDirectory::of(ctx)
        .record_visible(scope, definition, record_id)
        .await?
    {
        return Err(record_not_found());
    }
    let models: Vec<ScoringModel> = ModelStore::of(ctx)
        .list(Some(definition.record_type.as_str()))
        .await?
        .into_iter()
        .filter(|model| model.is_active)
        .collect();
    if models.is_empty() {
        return Ok(Vec::new());
    }
    let record = RecordRef::new(definition.record_type.clone(), record_id);
    let scores = ScoreStore::of(ctx).record_scores(&record).await?;
    let mut views: Vec<RecordScoreView> = models
        .into_iter()
        .map(|model| {
            let score = scores.iter().find(|score| score.model_id == model.id);
            RecordScoreView {
                score: score.map_or(0, |score| score.score),
                band: score.map_or_else(|| model.bands.band_for(0), |score| score.band),
                explanation: score
                    .map(|score| score.explanation.clone())
                    .unwrap_or_default(),
                calculated_at: score.map(|score| score.calculated_at),
                model,
            }
        })
        .collect();
    views.sort_by(|left, right| {
        right
            .model
            .is_primary
            .cmp(&left.model.is_primary)
            .then_with(|| left.model.name.cmp(&right.model.name))
    });
    Ok(views)
}

pub(crate) const PLUGIN_SIGNAL_SOURCE_PREFIX: &str = "plugin:";
const MAX_SIGNAL_SOURCE_LENGTH: usize = 90;

pub(crate) async fn record_signal(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    signal: BehaviorSignal,
    source_key: &str,
) -> Result<bool, AppError> {
    let scope = ctx.scope;
    let definition = ctx.registry().require(record_type.trim())?;
    if !is_scored_record_type(definition.record_type.as_str()) {
        return Err(record_not_found());
    }
    let permission = definition.update_permission.clone().ok_or_else(|| {
        AppError::forbidden_code(
            "permission_denied",
            "records of this type cannot be updated",
        )
    })?;
    scope.require(&permission)?;
    let record_id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_found())?;
    if !ScoringDirectory::of(ctx)
        .record_accessible(scope, definition, &permission, record_id)
        .await?
    {
        return Err(record_not_found());
    }
    let source_key = source_key.trim();
    if source_key.is_empty() || source_key.chars().count() > MAX_SIGNAL_SOURCE_LENGTH {
        return Err(AppError::invalid_fields(
            "invalid scoring signal",
            vec![FieldViolation::new(
                "source",
                format!("must have between 1 and {MAX_SIGNAL_SOURCE_LENGTH} characters"),
            )],
        ));
    }
    let record = RecordRef::new(definition.record_type.clone(), record_id);
    let mut tx = ctx.begin().await?;
    let recorded = crate::store::ScoreStore::of(ctx)
        .record_signal_in_tx(
            &mut tx,
            &record,
            signal,
            &format!("{PLUGIN_SIGNAL_SOURCE_PREFIX}{source_key}"),
            None,
            ctx.now(),
        )
        .await?;
    if recorded {
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            scope,
            ctx.now(),
            RecordMutation {
                record: record.clone(),
                event_type: "scoring.signal_recorded",
                payload: json!({ "signal": signal.as_str(), "source": source_key }),
                changes: vec![FieldChange::new(
                    "signal",
                    Value::Null,
                    json!(signal.as_str()),
                )],
                ip: None,
            },
        )
        .await?;
        ctx.jobs()
            .enqueue(
                &mut tx,
                aphel_kernel::application::shared::Job::new(
                    crate::domain::SCORING_RECALCULATE_JOB,
                    json!({ "record_type": record.record_type.as_str(), "record_id": record_id }),
                ),
            )
            .await?;
    }
    tx.commit().await?;
    Ok(recorded)
}

async fn validated(
    ctx: &Ctx<'_>,
    definition: &EntityDefinition,
    mut model: ScoringModel,
    criteria: Vec<(ScoringCriterion, Option<usize>)>,
    mut violations: Vec<FieldViolation>,
) -> Result<ScoringModel, AppError> {
    let scope = ctx.scope;
    let name_length = model.name.chars().count();
    if name_length == 0 || name_length > MAX_MODEL_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_MODEL_NAME_LENGTH} characters"),
        ));
    }
    if model
        .description
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_MODEL_DESCRIPTION_LENGTH)
    {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_MODEL_DESCRIPTION_LENGTH} characters"),
        ));
    }
    if model.bands.warm_from >= model.bands.hot_from {
        violations.push(FieldViolation::new(
            "hot_from",
            "must be greater than warm_from",
        ));
    }
    for (criterion, index) in &criteria {
        let (Some(filter), Some(index)) = (criterion.rule.filter(), index) else {
            continue;
        };
        for violation in ScoringDirectory::of(ctx)
            .filter_violations(scope, definition, filter)
            .await?
        {
            let field = match violation.field.strip_prefix("filter") {
                Some(rest) => format!("criteria.{index}.filter{rest}"),
                None => format!("criteria.{index}.filter.{}", violation.field),
            };
            violations.push(FieldViolation::new(field, violation.message));
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_SCORING_MODEL, violations));
    }
    model.criteria = criteria
        .into_iter()
        .map(|(criterion, _)| criterion)
        .collect();
    Ok(model)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    model: &ScoringModel,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let mut payload = changes_payload(&changes);
    payload["model"] = model.summary();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(SCORING_MODEL_RECORD, model.id),
            event_type,
            payload,
            changes,
            ip,
        },
    )
    .await
}

async fn require(ctx: &Ctx<'_>, id: &str) -> Result<ScoringModel, AppError> {
    let id = parse_model_id(id)?;
    ModelStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(model_not_found)
}

fn changes_affect_scores(current: &ScoringModel, next: &ScoringModel) -> bool {
    current.is_active != next.is_active
        || current.bands != next.bands
        || current
            .criteria
            .iter()
            .map(ScoringCriterion::snapshot)
            .collect::<Vec<_>>()
            != next
                .criteria
                .iter()
                .map(ScoringCriterion::snapshot)
                .collect::<Vec<_>>()
}

fn free_key(existing: &[ScoringModel], base: &str) -> String {
    let base: String = base.chars().take(56).collect();
    let base = base.trim_end_matches('_').to_owned();
    let mut candidate = base.clone();
    let mut suffix = 2;
    while existing.iter().any(|model| model.key == candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    candidate
}
