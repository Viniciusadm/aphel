use aphel_sdk::workflow::WorkflowRecordTypes;
use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CLOSING_STAGES_REQUIRED, CreateLostReasonCommand, CreatePipelineCommand, CreateStageCommand,
    DEFAULT_PIPELINE_REQUIRED, LAST_LOST_REASON, LOST_REASON_CREATED, LOST_REASON_REORDERED,
    LOST_REASON_UPDATED, LostReason, MAX_LOST_REASONS, MAX_PIPELINE_NAME_LENGTH,
    MAX_PIPELINE_TEAMS, MAX_PIPELINES, MAX_REQUIRED_FIELDS, MAX_STAGES_PER_PIPELINE,
    PIPELINE_ACTIVATED, PIPELINE_CREATED, PIPELINE_DEACTIVATED, PIPELINE_DEFAULT_CHANGED,
    PIPELINE_DELETED, PIPELINE_HAS_RECORDS, PIPELINE_LIMIT_REACHED, PIPELINE_REORDERED,
    PIPELINE_STAGE_CREATED, PIPELINE_STAGE_DEACTIVATED, PIPELINE_STAGE_DELETED,
    PIPELINE_STAGE_REORDERED, PIPELINE_STAGE_UPDATED, PIPELINE_UPDATED, Pipeline,
    PipelineRemovalCommand, PipelineStage, PipelineView, ReorderCommand, STAGE_HAS_RECORDS,
    StageKind, StageRemovalCommand, StageView, UpdateLostReasonCommand, UpdatePipelineCommand,
    UpdateStageCommand, keeps_closing_stages, lost_reason_not_found, map_stage_by_kind,
    normalize_field_keys, normalize_name, open_stage_insert_index, parse_id, pipeline_not_found,
    pipeline_ref, pipeline_visible, require_records_reader, stage_not_found, stage_ref,
    valid_probability, visible_pipelines,
};
use crate::requirements::is_requirable;
use crate::rotting::{PIPELINE_STAGE_ROTTING_TIGHTENED, parse_rotting_days, rotting_tightened};
use crate::seed::DEFAULT_STAGES;
use crate::store::PipelineStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::fields::{FieldDefinitionRepository, FieldSet};
use aphel_kernel::application::query::RecordQueryPorts;
use aphel_kernel::application::shared::{
    Actor, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType, ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};

#[derive(Serialize)]
struct PipelineSnapshot<'a> {
    name: &'a str,
    is_active: bool,
    team_ids: Vec<String>,
}

impl<'a> From<&'a Pipeline> for PipelineSnapshot<'a> {
    fn from(pipeline: &'a Pipeline) -> Self {
        Self {
            name: &pipeline.name,
            is_active: pipeline.is_active,
            team_ids: id_strings(&pipeline.team_ids),
        }
    }
}

#[derive(Serialize)]
struct StageSnapshot<'a> {
    name: &'a str,
    probability: u8,
    rotting_days: Option<u16>,
    kind: &'static str,
    is_active: bool,
    entry_required_fields: &'a [String],
    exit_required_fields: &'a [String],
}

impl<'a> From<&'a PipelineStage> for StageSnapshot<'a> {
    fn from(stage: &'a PipelineStage) -> Self {
        Self {
            name: &stage.name,
            probability: stage.probability,
            rotting_days: stage.rotting_days,
            kind: stage.kind.as_str(),
            is_active: stage.is_active,
            entry_required_fields: &stage.entry_required_fields,
            exit_required_fields: &stage.exit_required_fields,
        }
    }
}

fn id_strings(ids: &[Uuid]) -> Vec<String> {
    ids.iter().map(Uuid::to_string).collect()
}

#[derive(Serialize)]
struct LostReasonSnapshot<'a> {
    name: &'a str,
    is_active: bool,
}

impl<'a> From<&'a LostReason> for LostReasonSnapshot<'a> {
    fn from(reason: &'a LostReason) -> Self {
        Self {
            name: &reason.name,
            is_active: reason.is_active,
        }
    }
}

fn closing_stages_required() -> AppError {
    AppError::conflict_code(
        CLOSING_STAGES_REQUIRED,
        "a pipeline needs at least one active open, won and lost stage",
    )
}

fn name_violation(field: &str) -> FieldViolation {
    FieldViolation::new(
        field,
        format!("must have between 1 and {MAX_PIPELINE_NAME_LENGTH} characters"),
    )
}

fn invalid(message: &str, violations: Vec<FieldViolation>) -> Result<(), AppError> {
    if violations.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_fields(message, violations))
    }
}

fn parse_probability(value: Option<i64>, violations: &mut Vec<FieldViolation>) -> Option<u8> {
    let value = value?;
    let probability = valid_probability(value);
    if probability.is_none() {
        violations.push(FieldViolation::new(
            "probability",
            "must be an integer between 0 and 100",
        ));
    }
    probability
}

fn parse_ordered_ids(ids: &[String], expected: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
    let parsed: Option<Vec<Uuid>> = ids.iter().map(|id| parse_id(id)).collect();
    let valid = parsed.filter(|ordered| {
        let unique: HashSet<&Uuid> = ordered.iter().collect();
        let expected: HashSet<&Uuid> = expected.iter().collect();
        unique.len() == ordered.len() && unique == expected
    });
    valid.ok_or_else(|| {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every item exactly once",
            )],
        )
    })
}

pub async fn list_pipelines(ctx: &Ctx<'_>) -> Result<Vec<PipelineView>, AppError> {
    require_records_reader(ctx.scope)?;
    let workflow = visible(ctx).await?;
    let mut views = Vec::with_capacity(workflow.len());
    for pipeline in workflow {
        views.push(view(ctx, pipeline).await?);
    }
    Ok(views)
}

pub(crate) async fn get_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
) -> Result<PipelineView, AppError> {
    require_records_reader(ctx.scope)?;
    let pipeline = require_pipeline(ctx, pipeline_id).await?;
    view(ctx, pipeline).await
}

pub(crate) async fn create_pipeline(
    ctx: &Ctx<'_>,
    command: CreatePipelineCommand,
) -> Result<PipelineView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let name = normalize_name(&command.name);
    if name.is_none() {
        violations.push(name_violation("name"));
    }
    let team_ids = resolve_teams(ctx, &command.team_ids, &mut violations).await?;
    let now = ctx.now();
    let pipeline_id = Uuid::now_v7();
    let inputs: Vec<(String, Option<i64>, Option<StageKind>)> = if command.stages.is_empty() {
        DEFAULT_STAGES
            .iter()
            .map(|stage| {
                (
                    stage.name.to_owned(),
                    Some(i64::from(stage.probability)),
                    StageKind::parse(stage.kind),
                )
            })
            .collect()
    } else {
        command
            .stages
            .iter()
            .map(|stage| (stage.name.clone(), stage.probability, stage.kind))
            .collect()
    };
    if inputs.len() > MAX_STAGES_PER_PIPELINE {
        violations.push(FieldViolation::new(
            "stages",
            format!("a pipeline can have at most {MAX_STAGES_PER_PIPELINE} stages"),
        ));
    }
    let mut stages = Vec::with_capacity(inputs.len());
    for (index, (stage_name, probability, kind)) in inputs.into_iter().enumerate() {
        let mut stage_violations = Vec::new();
        let stage_name = normalize_name(&stage_name);
        if stage_name.is_none() {
            stage_violations.push(name_violation("name"));
        }
        let kind = kind.unwrap_or(StageKind::Open);
        let probability = parse_probability(probability, &mut stage_violations);
        violations.extend(stage_violations.into_iter().map(|violation| {
            FieldViolation::new(
                format!("stages.{index}.{}", violation.field),
                violation.message,
            )
        }));
        stages.push(PipelineStage {
            id: Uuid::now_v7(),
            pipeline_id,
            name: stage_name.unwrap_or_default(),
            position: index as i32,
            probability: probability.unwrap_or(match kind {
                StageKind::Won => 100,
                _ => 0,
            }),
            rotting_days: None,
            kind,
            is_active: true,
            entry_required_fields: Vec::new(),
            exit_required_fields: Vec::new(),
            created_at: now,
            updated_at: now,
        });
    }
    invalid("invalid pipeline", violations)?;
    if !keeps_closing_stages(&stages) {
        return Err(closing_stages_required());
    }
    let existing = PipelineStore::of(ctx).list_pipelines().await?;
    if existing.len() >= MAX_PIPELINES {
        return Err(AppError::conflict_code(
            PIPELINE_LIMIT_REACHED,
            format!("the organization can have at most {MAX_PIPELINES} workflow"),
        ));
    }
    let pipeline = Pipeline {
        id: pipeline_id,
        name: name.unwrap_or_default(),
        is_default: existing.is_empty(),
        position: existing
            .iter()
            .map(|pipeline| pipeline.position + 1)
            .max()
            .unwrap_or(0),
        is_active: true,
        team_ids,
        created_at: now,
        updated_at: now,
    };

    let mut tx = ctx.begin().await?;
    let repository = PipelineStore::of(ctx);
    repository.insert_pipeline(&mut tx, &pipeline).await?;
    repository
        .replace_workflow_teams(&mut tx, pipeline.id, &pipeline.team_ids)
        .await?;
    for stage in &stages {
        repository.insert_stage(&mut tx, stage).await?;
    }
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_CREATED,
            record: RecordRef::new(RecordType::PIPELINE, pipeline.id),
            payload: json!({
                "name": pipeline.name,
                "team_ids": id_strings(&pipeline.team_ids),
                "stages": stages.iter().map(stage_ref).collect::<Vec<_>>(),
            }),
            changes: diff_changes(
                &Value::Null,
                &PipelineSnapshot::from(&pipeline),
                &DiffOptions::default(),
            )?,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    get_pipeline(ctx, &pipeline.id.to_string()).await
}

pub(crate) async fn update_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    command: UpdatePipelineCommand,
) -> Result<PipelineView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_pipeline(ctx, pipeline_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut updated = current.clone();
    let mut violations = Vec::new();
    if let Some(name) = command.name.as_deref() {
        match normalize_name(name) {
            Some(name) => updated.name = name,
            None => violations.push(name_violation("name")),
        }
    }
    if let Some(team_ids) = &command.team_ids {
        updated.team_ids = resolve_teams(ctx, team_ids, &mut violations).await?;
    }
    invalid("invalid pipeline", violations)?;
    let changes = diff_changes(
        &PipelineSnapshot::from(&current),
        &PipelineSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return view(ctx, current).await;
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .update_pipeline(&mut tx, &updated)
        .await?;
    if updated.team_ids != current.team_ids {
        PipelineStore::of(ctx)
            .replace_workflow_teams(&mut tx, updated.id, &updated.team_ids)
            .await?;
    }
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_UPDATED,
            record: RecordRef::new(RecordType::PIPELINE, updated.id),
            payload: json!({
                "name": updated.name,
                "team_ids": id_strings(&updated.team_ids),
            }),
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    view(ctx, updated).await
}

pub(crate) async fn set_default_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    ip: Option<String>,
) -> Result<PipelineView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = require_pipeline(ctx, pipeline_id).await?;
    if pipeline.is_default {
        return view(ctx, pipeline).await;
    }
    if !pipeline.is_active {
        return Err(AppError::conflict_code(
            DEFAULT_PIPELINE_REQUIRED,
            "an inactive pipeline cannot be the default one",
        ));
    }
    let previous = PipelineStore::of(ctx)
        .list_pipelines()
        .await?
        .into_iter()
        .find(|candidate| candidate.is_default);
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .set_default_pipeline(&mut tx, pipeline.id)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_DEFAULT_CHANGED,
            record: RecordRef::new(RecordType::PIPELINE, pipeline.id),
            payload: json!({
                "pipeline": pipeline_ref(&pipeline),
                "previous": previous.as_ref().map(pipeline_ref),
            }),
            changes: vec![FieldChange::new(
                "default_pipeline_id",
                json!(previous.as_ref().map(|previous| previous.id.to_string())),
                json!(pipeline.id.to_string()),
            )],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    get_pipeline(ctx, pipeline_id).await
}

pub(crate) async fn activate_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    ip: Option<String>,
) -> Result<PipelineView, AppError> {
    set_pipeline_active(ctx, pipeline_id, true, ip).await
}

pub(crate) async fn deactivate_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    ip: Option<String>,
) -> Result<PipelineView, AppError> {
    set_pipeline_active(ctx, pipeline_id, false, ip).await
}

pub(crate) async fn reorder_pipelines(
    ctx: &Ctx<'_>,
    command: ReorderCommand,
) -> Result<Vec<PipelineView>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let workflow = PipelineStore::of(ctx).list_pipelines().await?;
    let current: Vec<Uuid> = workflow.iter().map(|pipeline| pipeline.id).collect();
    let ordered = parse_ordered_ids(&command.ids, &current)?;
    if ordered != current {
        let mut tx = ctx.begin().await?;
        PipelineStore::of(ctx)
            .reorder_pipelines(&mut tx, &ordered)
            .await?;
        record(
            ctx,
            &mut tx,
            RecordMutation {
                event_type: PIPELINE_REORDERED,
                record: RecordRef::organization(),
                payload: json!({ "pipeline_ids": id_strings(&ordered) }),
                changes: vec![FieldChange::new(
                    "pipeline_order",
                    json!(id_strings(&current)),
                    json!(id_strings(&ordered)),
                )],
                ip: command.ip,
            },
        )
        .await?;
        tx.commit().await?;
    }
    list_pipelines(ctx).await
}

pub(crate) async fn delete_pipeline(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    command: PipelineRemovalCommand,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = require_pipeline(ctx, pipeline_id).await?;
    if pipeline.is_default {
        return Err(AppError::conflict_code(
            DEFAULT_PIPELINE_REQUIRED,
            "the default pipeline cannot be deleted; choose another default first",
        ));
    }
    let repository = PipelineStore::of(ctx);
    let usage = repository.pipeline_usage(pipeline.id).await?;
    let source_stages = repository.list_stages(pipeline.id).await?;
    let mut moves: Vec<(PipelineStage, PipelineStage)> = Vec::new();
    let mut target_pipeline = None;
    if usage.total > 0 {
        let Some(target_id) = command
            .target_pipeline_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            return Err(AppError::conflict_code(
                PIPELINE_HAS_RECORDS,
                "the pipeline has records; choose a target pipeline to move them to",
            ));
        };
        let target = match parse_id(target_id).filter(|id| *id != pipeline.id) {
            Some(id) => repository
                .find_pipeline(id)
                .await?
                .filter(|target| target.is_active),
            None => None,
        };
        let Some(target) = target else {
            return Err(AppError::invalid_fields(
                "invalid target pipeline",
                vec![FieldViolation::new(
                    "target_pipeline_id",
                    "must be another active pipeline",
                )],
            ));
        };
        let target_stages = repository.list_stages(target.id).await?;
        let mut violations = Vec::new();
        for source in &source_stages {
            if repository.stage_usage(source.id).await?.total == 0 {
                continue;
            }
            let key = source.id.to_string();
            let mapped = match command.stage_mapping.get(&key) {
                Some(target_stage) => parse_id(target_stage).and_then(|id| {
                    target_stages
                        .iter()
                        .find(|stage| stage.id == id && stage.is_active)
                }),
                None => map_stage_by_kind(&target_stages, source),
            };
            match mapped {
                Some(stage) if stage.kind == source.kind => {
                    moves.push((source.clone(), stage.clone()));
                }
                _ => violations.push(FieldViolation::new(
                    format!("stage_mapping.{key}"),
                    "must be an active stage of the target pipeline with the same kind",
                )),
            }
        }
        invalid("invalid stage mapping", violations)?;
        target_pipeline = Some(target);
    }

    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let mut moved = 0;
    if let Some(target) = &target_pipeline {
        for (source, destination) in &moves {
            moved += move_records(
                ctx,
                &mut tx,
                source,
                Some(destination),
                Some((&pipeline, target)),
            )
            .await?;
        }
    }
    repository.delete_pipeline(&mut tx, pipeline.id).await?;
    record(ctx, &mut tx,
        RecordMutation {
            event_type: PIPELINE_DELETED,
            record: RecordRef::new(RecordType::PIPELINE, pipeline.id),
            payload: json!({
                "name": pipeline.name,
                "target_pipeline": target_pipeline.as_ref().map(pipeline_ref),
                "stage_mapping": moves
                    .iter()
                    .map(|(source, destination)| json!({ "from": stage_ref(source), "to": stage_ref(destination) }))
                    .collect::<Vec<_>>(),
                "moved_records": moved,
                "deleted_at": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            }),
            changes: diff_changes(
                &PipelineSnapshot::from(&pipeline),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip: command.ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn list_stages(
    ctx: &Ctx<'_>,
    pipeline_id: Option<&str>,
    include_inactive: bool,
    search: Option<&str>,
) -> Result<Vec<PipelineStage>, AppError> {
    require_records_reader(ctx.scope)?;
    let workflow = match pipeline_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => vec![require_pipeline(ctx, id).await?],
        None => visible(ctx).await?,
    };
    let mut stages = Vec::new();
    for pipeline in workflow {
        stages.extend(
            PipelineStore::of(ctx)
                .list_stages(pipeline.id)
                .await?
                .into_iter()
                .filter(|stage| include_inactive || stage.is_active)
                .filter(|stage| matches_search(&stage.name, search)),
        );
    }
    Ok(stages)
}

pub async fn get_stage(ctx: &Ctx<'_>, stage_id: &str) -> Result<PipelineStage, AppError> {
    require_records_reader(ctx.scope)?;
    require_stage(ctx, stage_id).await
}

pub(crate) async fn create_stage(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    command: CreateStageCommand,
) -> Result<PipelineStage, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = require_pipeline(ctx, pipeline_id).await?;
    let mut violations = Vec::new();
    let name = normalize_name(&command.name);
    if name.is_none() {
        violations.push(name_violation("name"));
    }
    let kind = command.kind.unwrap_or(StageKind::Open);
    let probability = parse_probability(command.probability, &mut violations);
    let rotting_days = parse_rotting_days(command.rotting_days, &mut violations);
    let entry_required_fields = required_fields(
        ctx,
        &command.entry_required_fields,
        "entry_required_fields",
        &mut violations,
    )
    .await?;
    let exit_required_fields = required_fields(
        ctx,
        &command.exit_required_fields,
        "exit_required_fields",
        &mut violations,
    )
    .await?;
    invalid("invalid stage", violations)?;
    let stages = PipelineStore::of(ctx).list_stages(pipeline.id).await?;
    if stages.len() >= MAX_STAGES_PER_PIPELINE {
        return Err(AppError::invalid_fields(
            "invalid stage",
            vec![FieldViolation::new(
                "name",
                format!("a pipeline can have at most {MAX_STAGES_PER_PIPELINE} stages"),
            )],
        ));
    }

    let now = ctx.now();
    let stage = PipelineStage {
        id: Uuid::now_v7(),
        pipeline_id: pipeline.id,
        name: name.unwrap_or_default(),
        position: stages.len() as i32,
        probability: probability.unwrap_or(match kind {
            StageKind::Won => 100,
            _ => 0,
        }),
        rotting_days: rotting_days.filter(|_| kind == StageKind::Open),
        kind,
        is_active: true,
        entry_required_fields,
        exit_required_fields,
        created_at: now,
        updated_at: now,
    };
    let mut ordered: Vec<Uuid> = stages.iter().map(|stage| stage.id).collect();
    let index = match kind {
        StageKind::Open => open_stage_insert_index(&stages),
        _ => ordered.len(),
    };
    ordered.insert(index, stage.id);

    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx).insert_stage(&mut tx, &stage).await?;
    PipelineStore::of(ctx)
        .reorder_stages(&mut tx, pipeline.id, &ordered)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_STAGE_CREATED,
            record: RecordRef::new(RecordType::PIPELINE_STAGE, stage.id),
            payload: json!({
                "pipeline_id": pipeline.id.to_string(),
                "name": stage.name,
                "kind": stage.kind.as_str(),
                "probability": stage.probability,
                "rotting_days": stage.rotting_days,
            }),
            changes: diff_changes(
                &Value::Null,
                &StageSnapshot::from(&stage),
                &DiffOptions::default(),
            )?,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_stage(ctx, &stage.id.to_string()).await
}

pub async fn update_stage(
    ctx: &Ctx<'_>,
    stage_id: &str,
    command: UpdateStageCommand,
) -> Result<PipelineStage, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_stage(ctx, stage_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut violations = Vec::new();
    let mut updated = current.clone();
    if let Some(name) = command.name.as_deref() {
        match normalize_name(name) {
            Some(name) => updated.name = name,
            None => violations.push(name_violation("name")),
        }
    }
    if let Some(probability) = parse_probability(command.probability, &mut violations) {
        updated.probability = probability;
    }
    if let Some(kind) = command.kind {
        updated.kind = kind;
    }
    if let Some(rotting_days) = command.rotting_days {
        updated.rotting_days = parse_rotting_days(rotting_days, &mut violations);
    }
    if updated.kind != StageKind::Open {
        updated.rotting_days = None;
    }
    if let Some(keys) = &command.entry_required_fields {
        updated.entry_required_fields =
            required_fields(ctx, keys, "entry_required_fields", &mut violations).await?;
    }
    if let Some(keys) = &command.exit_required_fields {
        updated.exit_required_fields =
            required_fields(ctx, keys, "exit_required_fields", &mut violations).await?;
    }
    invalid("invalid stage", violations)?;
    if updated.kind != current.kind {
        let usage = PipelineStore::of(ctx).stage_usage(current.id).await?;
        if usage.total > 0 {
            return Err(AppError::conflict_code(
                STAGE_HAS_RECORDS,
                "the kind of a stage with records cannot change",
            ));
        }
        let mut stages = PipelineStore::of(ctx)
            .list_stages(current.pipeline_id)
            .await?;
        for stage in stages.iter_mut().filter(|stage| stage.id == current.id) {
            stage.kind = updated.kind;
        }
        if !keeps_closing_stages(&stages) {
            return Err(closing_stages_required());
        }
    }
    save_stage(ctx, current, updated, command.ip).await
}

pub(crate) async fn activate_stage(
    ctx: &Ctx<'_>,
    stage_id: &str,
    ip: Option<String>,
) -> Result<PipelineStage, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_stage(ctx, stage_id).await?;
    let mut updated = current.clone();
    updated.is_active = true;
    save_stage(ctx, current, updated, ip).await
}

pub(crate) async fn reorder_stages(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    command: ReorderCommand,
) -> Result<PipelineView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let pipeline = require_pipeline(ctx, pipeline_id).await?;
    let stages = PipelineStore::of(ctx).list_stages(pipeline.id).await?;
    let current: Vec<Uuid> = stages.iter().map(|stage| stage.id).collect();
    let ordered = parse_ordered_ids(&command.ids, &current)?;
    if ordered == current {
        return view(ctx, pipeline).await;
    }
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .reorder_stages(&mut tx, pipeline.id, &ordered)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_STAGE_REORDERED,
            record: RecordRef::new(RecordType::PIPELINE, pipeline.id),
            payload: json!({
                "stage_ids": ordered.iter().map(Uuid::to_string).collect::<Vec<_>>(),
            }),
            changes: vec![FieldChange::new(
                "stage_order",
                json!(current.iter().map(Uuid::to_string).collect::<Vec<_>>()),
                json!(ordered.iter().map(Uuid::to_string).collect::<Vec<_>>()),
            )],
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    view(ctx, pipeline).await
}

pub(crate) async fn deactivate_stage(
    ctx: &Ctx<'_>,
    stage_id: &str,
    command: StageRemovalCommand,
) -> Result<PipelineStage, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_stage(ctx, stage_id).await?;
    if !current.is_active {
        return Ok(current);
    }
    let stages = PipelineStore::of(ctx)
        .list_stages(current.pipeline_id)
        .await?;
    let remaining: Vec<PipelineStage> = stages
        .iter()
        .cloned()
        .map(|mut stage| {
            if stage.id == current.id {
                stage.is_active = false;
            }
            stage
        })
        .collect();
    if !keeps_closing_stages(&remaining) {
        return Err(closing_stages_required());
    }
    let usage = PipelineStore::of(ctx).stage_usage(current.id).await?;
    let target = removal_target(
        &current,
        &stages,
        command.target_stage_id.as_deref(),
        usage.active,
    )?;

    let now = ctx.now();
    let mut updated = current.clone();
    updated.is_active = false;
    updated.updated_at = now;
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .update_stage(&mut tx, &updated)
        .await?;
    let moved = move_records(ctx, &mut tx, &current, target.as_ref(), None).await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_STAGE_DEACTIVATED,
            record: RecordRef::new(RecordType::PIPELINE_STAGE, current.id),
            payload: json!({
                "pipeline_id": current.pipeline_id.to_string(),
                "name": current.name,
                "target_stage": target.as_ref().map(stage_ref),
                "moved_records": moved,
            }),
            changes: vec![FieldChange::new("is_active", json!(true), json!(false))],
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_stage(ctx, stage_id).await
}

pub(crate) async fn delete_stage(
    ctx: &Ctx<'_>,
    stage_id: &str,
    command: StageRemovalCommand,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_stage(ctx, stage_id).await?;
    let stages = PipelineStore::of(ctx)
        .list_stages(current.pipeline_id)
        .await?;
    let remaining: Vec<PipelineStage> = stages
        .iter()
        .filter(|stage| stage.id != current.id)
        .cloned()
        .collect();
    if !keeps_closing_stages(&remaining) {
        return Err(closing_stages_required());
    }
    let usage = PipelineStore::of(ctx).stage_usage(current.id).await?;
    let target = removal_target(
        &current,
        &stages,
        command.target_stage_id.as_deref(),
        usage.total,
    )?;

    let mut tx = ctx.begin().await?;
    let moved = move_records(ctx, &mut tx, &current, target.as_ref(), None).await?;
    PipelineStore::of(ctx)
        .delete_stage(&mut tx, current.id)
        .await?;
    let ordered: Vec<Uuid> = remaining.iter().map(|stage| stage.id).collect();
    PipelineStore::of(ctx)
        .reorder_stages(&mut tx, current.pipeline_id, &ordered)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_STAGE_DELETED,
            record: RecordRef::new(RecordType::PIPELINE_STAGE, current.id),
            payload: json!({
                "pipeline_id": current.pipeline_id.to_string(),
                "name": current.name,
                "target_stage": target.as_ref().map(stage_ref),
                "moved_records": moved,
            }),
            changes: diff_changes(
                &StageSnapshot::from(&current),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip: command.ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn list_lost_reasons(
    ctx: &Ctx<'_>,
    include_inactive: bool,
    search: Option<&str>,
) -> Result<Vec<LostReason>, AppError> {
    require_records_reader(ctx.scope)?;
    let reasons = PipelineStore::of(ctx).list_lost_reasons().await?;
    Ok(reasons
        .into_iter()
        .filter(|reason| include_inactive || reason.is_active)
        .filter(|reason| matches_search(&reason.name, search))
        .collect())
}

pub(crate) async fn get_lost_reason(
    ctx: &Ctx<'_>,
    reason_id: &str,
) -> Result<LostReason, AppError> {
    require_records_reader(ctx.scope)?;
    require_lost_reason(ctx, reason_id).await
}

pub(crate) async fn create_lost_reason(
    ctx: &Ctx<'_>,
    command: CreateLostReasonCommand,
) -> Result<LostReason, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let name = normalize_name(&command.name).ok_or_else(|| {
        AppError::invalid_fields("invalid lost reason", vec![name_violation("name")])
    })?;
    let reasons = PipelineStore::of(ctx).list_lost_reasons().await?;
    if reasons.len() >= MAX_LOST_REASONS {
        return Err(AppError::invalid_fields(
            "invalid lost reason",
            vec![FieldViolation::new(
                "name",
                format!("the organization can have at most {MAX_LOST_REASONS} lost reasons"),
            )],
        ));
    }
    let now = ctx.now();
    let reason = LostReason {
        id: Uuid::now_v7(),
        name,
        position: reasons
            .iter()
            .map(|reason| reason.position + 1)
            .max()
            .unwrap_or(0),
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .insert_lost_reason(&mut tx, &reason)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: LOST_REASON_CREATED,
            record: RecordRef::new(RecordType::LOST_REASON, reason.id),
            payload: json!({ "name": reason.name }),
            changes: diff_changes(
                &Value::Null,
                &LostReasonSnapshot::from(&reason),
                &DiffOptions::default(),
            )?,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_lost_reason(ctx, &reason.id.to_string()).await
}

pub(crate) async fn update_lost_reason(
    ctx: &Ctx<'_>,
    reason_id: &str,
    command: UpdateLostReasonCommand,
) -> Result<LostReason, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_lost_reason(ctx, reason_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut updated = current.clone();
    if let Some(name) = command.name.as_deref() {
        updated.name = normalize_name(name).ok_or_else(|| {
            AppError::invalid_fields("invalid lost reason", vec![name_violation("name")])
        })?;
    }
    if let Some(is_active) = command.is_active {
        updated.is_active = is_active;
    }
    if !updated.is_active && current.is_active {
        let reasons = PipelineStore::of(ctx).list_lost_reasons().await?;
        if !reasons
            .iter()
            .any(|reason| reason.is_active && reason.id != current.id)
        {
            return Err(AppError::conflict_code(
                LAST_LOST_REASON,
                "at least one lost reason must stay active",
            ));
        }
    }
    let changes = diff_changes(
        &LostReasonSnapshot::from(&current),
        &LostReasonSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .update_lost_reason(&mut tx, &updated)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: LOST_REASON_UPDATED,
            record: RecordRef::new(RecordType::LOST_REASON, updated.id),
            payload: json!({ "name": updated.name, "is_active": updated.is_active }),
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    require_lost_reason(ctx, reason_id).await
}

pub(crate) async fn reorder_lost_reasons(
    ctx: &Ctx<'_>,
    command: ReorderCommand,
) -> Result<Vec<LostReason>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let reasons = PipelineStore::of(ctx).list_lost_reasons().await?;
    let current: Vec<Uuid> = reasons.iter().map(|reason| reason.id).collect();
    let ordered = parse_ordered_ids(&command.ids, &current)?;
    if ordered != current {
        let mut tx = ctx.begin().await?;
        PipelineStore::of(ctx)
            .reorder_lost_reasons(&mut tx, &ordered)
            .await?;
        let ids = |list: &[Uuid]| json!(list.iter().map(Uuid::to_string).collect::<Vec<_>>());
        record(
            ctx,
            &mut tx,
            RecordMutation {
                event_type: LOST_REASON_REORDERED,
                record: RecordRef::organization(),
                payload: json!({ "lost_reason_ids": ids(&ordered) }),
                changes: vec![FieldChange::new(
                    "lost_reason_order",
                    ids(&current),
                    ids(&ordered),
                )],
                ip: command.ip,
            },
        )
        .await?;
        tx.commit().await?;
    }
    PipelineStore::of(ctx).list_lost_reasons().await
}

async fn view(ctx: &Ctx<'_>, pipeline: Pipeline) -> Result<PipelineView, AppError> {
    let stages = PipelineStore::of(ctx).list_stages(pipeline.id).await?;
    let counts: HashMap<Uuid, u64> = PipelineStore::of(ctx)
        .stage_record_counts(ctx, pipeline.id)
        .await?;
    Ok(PipelineView {
        stages: stages
            .into_iter()
            .map(|stage| StageView {
                record_count: counts.get(&stage.id).copied().unwrap_or(0),
                stage,
            })
            .collect(),
        pipeline,
    })
}

async fn save_stage(
    ctx: &Ctx<'_>,
    current: PipelineStage,
    mut updated: PipelineStage,
    ip: Option<String>,
) -> Result<PipelineStage, AppError> {
    let changes = diff_changes(
        &StageSnapshot::from(&current),
        &StageSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .update_stage(&mut tx, &updated)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: PIPELINE_STAGE_UPDATED,
            record: RecordRef::new(RecordType::PIPELINE_STAGE, updated.id),
            payload: json!({
                "pipeline_id": updated.pipeline_id.to_string(),
                "name": updated.name,
                "kind": updated.kind.as_str(),
                "probability": updated.probability,
                "rotting_days": updated.rotting_days,
                "is_active": updated.is_active,
                "entry_required_fields": updated.entry_required_fields,
                "exit_required_fields": updated.exit_required_fields,
            }),
            changes,
            ip,
        },
    )
    .await?;
    if rotting_tightened(current.rotting_days, updated.rotting_days) {
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(
                    Actor::from_scope(ctx.scope),
                    PIPELINE_STAGE_ROTTING_TIGHTENED,
                    updated.updated_at,
                )
                .with_record(RecordRef::new(RecordType::PIPELINE_STAGE, updated.id))
                .with_payload(json!({
                    "pipeline_id": updated.pipeline_id.to_string(),
                    "stage_id": updated.id.to_string(),
                    "old": current.rotting_days,
                    "new": updated.rotting_days,
                })),
            )
            .await?;
    }
    tx.commit().await?;
    require_stage(ctx, &updated.id.to_string()).await
}

fn removal_target(
    current: &PipelineStage,
    stages: &[PipelineStage],
    target_id: Option<&str>,
    records: u64,
) -> Result<Option<PipelineStage>, AppError> {
    let Some(target_id) = target_id.map(str::trim).filter(|id| !id.is_empty()) else {
        if records > 0 {
            return Err(AppError::conflict_code(
                STAGE_HAS_RECORDS,
                "the stage has records; choose a target stage to move them to",
            ));
        }
        return Ok(None);
    };
    let target = parse_id(target_id)
        .and_then(|id| stages.iter().find(|stage| stage.id == id))
        .filter(|stage| stage.id != current.id && stage.is_active && stage.kind == current.kind)
        .cloned();
    match target {
        Some(target) => Ok(Some(target)),
        None => Err(AppError::invalid_fields(
            "invalid target stage",
            vec![FieldViolation::new(
                "target_stage_id",
                "must be another active stage of the same pipeline and kind",
            )],
        )),
    }
}

async fn move_records(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    from: &PipelineStage,
    target: Option<&PipelineStage>,
    pipelines: Option<(&Pipeline, &Pipeline)>,
) -> Result<u64, AppError> {
    use aphel_sdk::workflow::{StageMove, StagedRecordSource, WorkflowReference};
    let Some(target) = target else {
        return Ok(0);
    };
    let movement = StageMove {
        from: WorkflowReference {
            id: from.id,
            name: from.name.clone(),
        },
        to: WorkflowReference {
            id: target.id,
            name: target.name.clone(),
        },
        target_pipeline_id: target.pipeline_id,
        pipelines: pipelines.map(|(from, to)| {
            (
                WorkflowReference {
                    id: from.id,
                    name: from.name.clone(),
                },
                WorkflowReference {
                    id: to.id,
                    name: to.name.clone(),
                },
            )
        }),
    };
    let mut count = 0;
    for (_, source) in aphel_kernel::modules::app().all_contributions::<dyn StagedRecordSource>() {
        count += source.move_records(ctx, tx, &movement).await?;
    }
    Ok(count)
}

async fn record(ctx: &Ctx<'_>, tx: &mut MySqlTx, mutation: RecordMutation) -> Result<(), AppError> {
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        mutation,
    )
    .await
}

async fn require_pipeline(ctx: &Ctx<'_>, pipeline_id: &str) -> Result<Pipeline, AppError> {
    let Some(id) = parse_id(pipeline_id) else {
        return Err(pipeline_not_found());
    };
    PipelineStore::of(ctx)
        .find_pipeline(id)
        .await?
        .filter(|pipeline| pipeline_visible(ctx.scope, pipeline))
        .ok_or_else(pipeline_not_found)
}

async fn require_stage(ctx: &Ctx<'_>, stage_id: &str) -> Result<PipelineStage, AppError> {
    let Some(id) = parse_id(stage_id) else {
        return Err(stage_not_found());
    };
    let stage = PipelineStore::of(ctx)
        .find_stage(id)
        .await?
        .ok_or_else(stage_not_found)?;
    let visible = PipelineStore::of(ctx)
        .find_pipeline(stage.pipeline_id)
        .await?
        .is_some_and(|pipeline| pipeline_visible(ctx.scope, &pipeline));
    if !visible {
        return Err(stage_not_found());
    }
    Ok(stage)
}

async fn visible(ctx: &Ctx<'_>) -> Result<Vec<Pipeline>, AppError> {
    Ok(visible_pipelines(
        ctx.scope,
        PipelineStore::of(ctx).list_pipelines().await?,
    ))
}

async fn set_pipeline_active(
    ctx: &Ctx<'_>,
    pipeline_id: &str,
    active: bool,
    ip: Option<String>,
) -> Result<PipelineView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = require_pipeline(ctx, pipeline_id).await?;
    if current.is_active == active {
        return view(ctx, current).await;
    }
    if !active && current.is_default {
        return Err(AppError::conflict_code(
            DEFAULT_PIPELINE_REQUIRED,
            "the default pipeline cannot be deactivated; choose another default first",
        ));
    }
    let mut updated = current.clone();
    updated.is_active = active;
    updated.updated_at = ctx.now();
    let action = if active {
        PIPELINE_ACTIVATED
    } else {
        PIPELINE_DEACTIVATED
    };
    let mut tx = ctx.begin().await?;
    PipelineStore::of(ctx)
        .update_pipeline(&mut tx, &updated)
        .await?;
    record(
        ctx,
        &mut tx,
        RecordMutation {
            event_type: action,
            record: RecordRef::new(RecordType::PIPELINE, updated.id),
            payload: json!({ "name": updated.name }),
            changes: vec![FieldChange::new(
                "is_active",
                json!(current.is_active),
                json!(active),
            )],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    view(ctx, updated).await
}

async fn resolve_teams(
    ctx: &Ctx<'_>,
    team_ids: &[String],
    violations: &mut Vec<FieldViolation>,
) -> Result<Vec<Uuid>, AppError> {
    let mut parsed: Vec<Uuid> = Vec::with_capacity(team_ids.len());
    for value in team_ids {
        match parse_id(value) {
            Some(id) if !parsed.contains(&id) => parsed.push(id),
            Some(_) => {}
            None => {
                violations.push(FieldViolation::new("team_ids", "must list valid team ids"));
                return Ok(Vec::new());
            }
        }
    }
    if parsed.len() > MAX_PIPELINE_TEAMS {
        violations.push(FieldViolation::new(
            "team_ids",
            format!("a pipeline can be restricted to at most {MAX_PIPELINE_TEAMS} teams"),
        ));
        return Ok(Vec::new());
    }
    if parsed.is_empty() {
        return Ok(parsed);
    }
    let existing = PipelineStore::of(ctx).existing_team_ids(&parsed).await?;
    if parsed.iter().any(|id| !existing.contains(id)) {
        violations.push(FieldViolation::new(
            "team_ids",
            "must list teams of the organization",
        ));
        return Ok(Vec::new());
    }
    parsed.sort();
    Ok(parsed)
}

async fn required_fields(
    ctx: &Ctx<'_>,
    keys: &[String],
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Result<Vec<String>, AppError> {
    let keys = normalize_field_keys(keys);
    if keys.is_empty() {
        return Ok(keys);
    }
    if keys.len() > MAX_REQUIRED_FIELDS {
        violations.push(FieldViolation::new(
            field,
            format!("can list at most {MAX_REQUIRED_FIELDS} fields"),
        ));
        return Ok(Vec::new());
    }
    let query = ctx.query();
    let mut available = HashSet::new();
    for definition in query
        .registry()
        .definitions()
        .filter(|definition| definition.capabilities.has_trait("workflow.staged"))
    {
        let fields = FieldSet::new(
            definition,
            query.definitions().list(&definition.record_type).await?,
        );
        available.extend(
            fields
                .active()
                .filter(|field| is_requirable(&fields, field.key()))
                .map(|field| field.key().to_owned()),
        );
    }
    for key in &keys {
        if !available.contains(key) {
            violations.push(FieldViolation::new(
                format!("{field}.{key}"),
                "is not a field that can be required",
            ));
        }
    }
    Ok(keys)
}

async fn require_lost_reason(ctx: &Ctx<'_>, reason_id: &str) -> Result<LostReason, AppError> {
    let Some(id) = parse_id(reason_id) else {
        return Err(lost_reason_not_found());
    };
    PipelineStore::of(ctx)
        .find_lost_reason(id)
        .await?
        .ok_or_else(lost_reason_not_found)
}

fn matches_search(name: &str, search: Option<&str>) -> bool {
    match search.map(str::trim).filter(|search| !search.is_empty()) {
        Some(search) => name.to_lowercase().contains(&search.to_lowercase()),
        None => true,
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct StageFieldChoice {
    pub key: String,
    pub label: String,
}
pub(crate) async fn requirable_fields(ctx: &Ctx<'_>) -> Result<Vec<StageFieldChoice>, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut choices = std::collections::BTreeMap::new();
    for definition in ctx
        .registry()
        .definitions()
        .filter(|definition| definition.capabilities.has_trait("workflow.staged"))
    {
        if ctx
            .scope
            .record_filter(&definition.read_permission)
            .is_err()
        {
            continue;
        }
        let fields = ctx
            .field_set(ctx.scope, definition.record_type.as_str())
            .await?;
        for field in fields
            .active()
            .filter(|field| is_requirable(&fields, field.key()))
        {
            choices
                .entry(field.key().to_owned())
                .or_insert(StageFieldChoice {
                    key: field.key().to_owned(),
                    label: field.definition.label.clone(),
                });
        }
    }
    Ok(choices.into_values().collect())
}
