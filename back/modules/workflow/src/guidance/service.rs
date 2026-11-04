use aphel_sdk::workflow::WorkflowRecordTypes;
use std::collections::BTreeSet;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{PipelineStage, parse_id, stage_not_found};
use crate::guidance::domain::{
    CHECKLIST_ITEM_NOT_FOUND, CHECKLIST_PATH_PREFIX, ChecklistItem, ChecklistItemInput,
    ChecklistProgress, GuidanceText, MAX_CHECKLIST_ITEMS, MAX_CHECKLIST_LABEL_LENGTH,
    MAX_GUIDANCE_LENGTH, RECORD_CHECKLIST_ITEM_CHECKED, RECORD_CHECKLIST_ITEM_UNCHECKED,
    RECORD_NOT_FOUND, RecordChecklist, RecordChecklistItem, RecordLocation, STAGE_GUIDANCE_UPDATED,
    SaveStageGuidanceCommand, StageGuidance, ToggleChecklistItemCommand,
};
use crate::guidance::store::GuidanceStore;
use crate::store::PipelineStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::{CustomFieldPorts, ReferenceChecker};
use aphel_kernel::application::shared::{FieldChange, RecordRef, RecordType, ensure_version};
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, changes_payload, publish_mutation};
use aphel_sdk::configuration::{is_config_key, key_from_name};

const INVALID_STAGE_GUIDANCE: &str = "invalid stage guidance";

fn record_not_found() -> AppError {
    AppError::not_found_code(RECORD_NOT_FOUND, "record was not found")
}

fn item_not_found() -> AppError {
    AppError::not_found_code(CHECKLIST_ITEM_NOT_FOUND, "checklist item was not found")
}

pub(crate) async fn get_stage_guidance(
    ctx: &Ctx<'_>,
    stage_id: &str,
) -> Result<StageGuidance, AppError> {
    ctx.scope
        .record_filter(&Permission::from_static("workflow.read"))?;
    let stage = stage(ctx, stage_id).await?;
    load(ctx, &stage).await
}

pub(crate) async fn save_stage_guidance(
    ctx: &Ctx<'_>,
    stage_id: &str,
    command: SaveStageGuidanceCommand,
) -> Result<StageGuidance, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let stage = stage(ctx, stage_id).await?;
    let current = load(ctx, &stage).await?;
    if let Some(updated_at) = current.updated_at {
        ensure_version(command.expected_updated_at, updated_at)?;
    }
    let now = ctx.now();
    let mut violations = Vec::new();
    let guidance = command
        .guidance
        .map(|text| text.trim().to_owned())
        .unwrap_or_else(|| current.guidance.clone());
    if guidance.chars().count() > MAX_GUIDANCE_LENGTH {
        violations.push(FieldViolation::new(
            "guidance",
            format!("must have at most {MAX_GUIDANCE_LENGTH} characters"),
        ));
    }
    let items = match command.items {
        Some(inputs) => plan_items(&stage, &current.items, &inputs, now, &mut violations),
        None => current.items.clone(),
    };
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_STAGE_GUIDANCE, violations));
    }
    let proposed = StageGuidance {
        guidance,
        items,
        updated_by: Some(ctx.scope.user_id().clone()),
        updated_at: Some(now),
        ..current.clone()
    };
    let changes = diff_values(
        &current.snapshot(),
        &proposed.snapshot(),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() && current.updated_at.is_some() {
        return Ok(current);
    }
    let mut tx = ctx.begin().await?;
    let repository = GuidanceStore::of(ctx);
    repository
        .save_guidance(
            &mut tx,
            &GuidanceText {
                stage_id: stage.id,
                guidance: proposed.guidance.clone(),
                updated_by: proposed.updated_by.clone(),
                created_at: now,
                updated_at: now,
            },
        )
        .await?;
    for removed in current
        .items
        .iter()
        .filter(|item| !proposed.items.iter().any(|kept| kept.id == item.id))
    {
        repository.delete_item(&mut tx, removed.id).await?;
    }
    for item in &proposed.items {
        match current.items.iter().find(|existing| existing.id == item.id) {
            Some(existing) if same_content(existing, item) => {}
            Some(_) => repository.update_item(&mut tx, item).await?,
            None => repository.insert_item(&mut tx, item).await?,
        }
    }
    let mut payload = changes_payload(&changes);
    payload["stage"] = json!({
        "id": stage.id.to_string(),
        "name": stage.name,
        "pipeline_id": stage.pipeline_id.to_string(),
    });
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::PIPELINE_STAGE, stage.id),
            event_type: STAGE_GUIDANCE_UPDATED,
            payload,
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    load(ctx, &stage).await
}

pub(crate) async fn record_checklist(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<RecordChecklist, AppError> {
    let definition = ctx.registry().require(record_type)?;
    let visibility = ctx.scope.record_filter(&definition.read_permission)?;
    let record_type = &definition.record_type;
    let (record_id, location) =
        visible_record(ctx, &visibility, record_type.as_str(), record_id).await?;
    checklist(ctx, record_type, record_id, location).await
}

pub(crate) async fn toggle_checklist_item(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
    item_id: &str,
    command: ToggleChecklistItemCommand,
) -> Result<RecordChecklist, AppError> {
    let definition = ctx.registry().require(record_type)?;
    let permission = definition
        .action_permission("update")
        .ok_or_else(record_not_found)?;
    let visibility = ctx.scope.record_filter(&permission)?;
    let record_type = &definition.record_type;
    let (record_id, location) =
        visible_record(ctx, &visibility, record_type.as_str(), record_id).await?;
    let item_id = parse_id(item_id).ok_or_else(item_not_found)?;
    let repository = GuidanceStore::of(ctx);
    let item = repository
        .find_item(item_id)
        .await?
        .ok_or_else(item_not_found)?;
    let stage = PipelineStore::of(ctx)
        .find_stage(item.stage_id)
        .await?
        .ok_or_else(item_not_found)?;
    if stage.pipeline_id != location.pipeline_id {
        return Err(item_not_found());
    }
    let progress = repository.progress(record_type, record_id).await?;
    let was_done = progress.iter().any(|entry| entry.item_id == item.id);
    if was_done == command.done {
        return checklist(ctx, record_type, record_id, location).await;
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if command.done {
        repository
            .mark_done(
                &mut tx,
                record_type,
                record_id,
                &ChecklistProgress {
                    item_id: item.id,
                    done_by: Some(ctx.scope.user_id().clone()),
                    done_by_name: None,
                    done_at: now,
                },
            )
            .await?;
    } else {
        repository
            .mark_pending(&mut tx, record_type, record_id, item.id)
            .await?;
    }
    let changes = vec![FieldChange {
        field: format!("{CHECKLIST_PATH_PREFIX}{}", item.key),
        old: Value::Bool(was_done),
        new: Value::Bool(command.done),
    }];
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: RecordRef::new(record_type.clone(), record_id),
            event_type: if command.done {
                RECORD_CHECKLIST_ITEM_CHECKED
            } else {
                RECORD_CHECKLIST_ITEM_UNCHECKED
            },
            payload: json!({
                "item": {
                    "id": item.id.to_string(),
                    "key": item.key,
                    "label": item.label,
                    "required": item.is_required,
                },
                "stage": {
                    "id": stage.id.to_string(),
                    "name": stage.name,
                    "pipeline_id": stage.pipeline_id.to_string(),
                },
                "done": command.done,
            }),
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    checklist(ctx, record_type, record_id, location).await
}

async fn checklist(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
    record_id: Uuid,
    location: RecordLocation,
) -> Result<RecordChecklist, AppError> {
    let stage = PipelineStore::of(ctx)
        .find_stage(location.stage_id)
        .await?
        .ok_or_else(stage_not_found)?;
    let guidance = load(ctx, &stage).await?;
    let progress = GuidanceStore::of(ctx)
        .progress(record_type, record_id)
        .await?;
    let items = guidance
        .items
        .iter()
        .map(|item| RecordChecklistItem {
            item: item.clone(),
            progress: progress
                .iter()
                .find(|entry| entry.item_id == item.id)
                .cloned(),
        })
        .collect();
    Ok(RecordChecklist {
        record_type: record_type.clone(),
        record_id,
        guidance,
        items,
    })
}

async fn visible_record(
    ctx: &Ctx<'_>,
    visibility: &RecordScopeFilter,
    record_type: &str,
    record_id: &str,
) -> Result<(Uuid, RecordLocation), AppError> {
    let id = parse_id(record_id).ok_or_else(record_not_found)?;
    let definition = ctx.registry().require(record_type)?;
    let visible = ctx
        .fields()
        .references()
        .existing_records(ctx.scope, definition, visibility, &[id])
        .await?;
    if !visible.contains(&id) {
        return Err(record_not_found());
    }
    let source = aphel_kernel::modules::app()
        .contributions::<dyn aphel_sdk::workflow::StagedRecordSource>()
        .into_iter()
        .find(|source| source.record_type() == definition.record_type);
    let location = if let Some(source) = source {
        source
            .location(ctx.pool(), id)
            .await?
            .ok_or_else(record_not_found)?
    } else {
        use aphel_sdk::workflow::WorkflowTraits;
        let mapping = definition
            .capabilities
            .stage_mapping()
            .ok_or_else(record_not_found)?;
        let record = ctx
            .load_record(record_type, id)
            .await?
            .ok_or_else(record_not_found)?;
        let value = |key: &str| {
            record
                .get(key)
                .or_else(|| {
                    record
                        .get("custom_fields")
                        .and_then(|fields| fields.get(key))
                })
                .and_then(Value::as_str)
                .and_then(parse_id)
        };
        RecordLocation {
            pipeline_id: value(&mapping.pipeline_field).ok_or_else(record_not_found)?,
            stage_id: value(&mapping.stage_field).ok_or_else(record_not_found)?,
        }
    };
    Ok((id, location))
}

async fn stage(ctx: &Ctx<'_>, stage_id: &str) -> Result<PipelineStage, AppError> {
    let id = parse_id(stage_id).ok_or_else(stage_not_found)?;
    PipelineStore::of(ctx)
        .find_stage(id)
        .await?
        .ok_or_else(stage_not_found)
}

async fn load(ctx: &Ctx<'_>, stage: &PipelineStage) -> Result<StageGuidance, AppError> {
    let repository = GuidanceStore::of(ctx);
    let text = repository.guidance(stage.id).await?;
    let items = repository.items(stage.id).await?;
    Ok(StageGuidance {
        stage_id: stage.id,
        pipeline_id: stage.pipeline_id,
        stage_name: stage.name.clone(),
        guidance: text
            .as_ref()
            .map(|text| text.guidance.clone())
            .unwrap_or_default(),
        items,
        updated_by: text.as_ref().and_then(|text| text.updated_by.clone()),
        updated_at: text.as_ref().map(|text| text.updated_at),
    })
}

fn same_content(left: &ChecklistItem, right: &ChecklistItem) -> bool {
    left.key == right.key
        && left.label == right.label
        && left.is_required == right.is_required
        && left.position == right.position
}

fn plan_items(
    stage: &PipelineStage,
    current: &[ChecklistItem],
    inputs: &[ChecklistItemInput],
    now: chrono::DateTime<chrono::Utc>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<ChecklistItem> {
    if inputs.len() > MAX_CHECKLIST_ITEMS {
        violations.push(FieldViolation::new(
            "items",
            format!("can have at most {MAX_CHECKLIST_ITEMS} items"),
        ));
        return current.to_vec();
    }
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut used_ids: BTreeSet<Uuid> = BTreeSet::new();
    let mut items = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        let path = |field: &str| format!("items.{index}.{field}");
        let label = input.label.trim().to_owned();
        let length = label.chars().count();
        if length == 0 || length > MAX_CHECKLIST_LABEL_LENGTH {
            violations.push(FieldViolation::new(
                path("label"),
                format!("must have between 1 and {MAX_CHECKLIST_LABEL_LENGTH} characters"),
            ));
        }
        let by_id = input
            .id
            .as_deref()
            .and_then(parse_id)
            .and_then(|id| current.iter().find(|item| item.id == id));
        let explicit_key = input
            .key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty());
        let existing = by_id
            .or_else(|| explicit_key.and_then(|key| current.iter().find(|item| item.key == key)));
        let existing = existing.filter(|item| !used_ids.contains(&item.id));
        let key = match (explicit_key, existing) {
            (Some(key), _) => key.to_owned(),
            (None, Some(existing)) => existing.key.clone(),
            (None, None) => {
                let base = key_from_name(&label);
                let mut candidate = base.clone();
                let mut suffix = 2;
                while taken.contains(&candidate)
                    || current.iter().any(|item| {
                        item.key == candidate
                            && !inputs.iter().any(|other| {
                                other.id.as_deref().and_then(parse_id) == Some(item.id)
                            })
                    })
                {
                    candidate = format!("{base}_{suffix}");
                    suffix += 1;
                }
                candidate
            }
        };
        if !is_config_key(&key) {
            violations.push(FieldViolation::new(
                path("key"),
                "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
            ));
        } else if !taken.insert(key.clone()) {
            violations.push(FieldViolation::new(path("key"), "is used by another item"));
        }
        let position = i32::try_from(index).unwrap_or(i32::MAX);
        let item = match existing {
            Some(existing) => {
                used_ids.insert(existing.id);
                ChecklistItem {
                    key,
                    label,
                    is_required: input.required,
                    position,
                    updated_at: now,
                    ..existing.clone()
                }
            }
            None => ChecklistItem {
                id: Uuid::now_v7(),
                stage_id: stage.id,
                key,
                label,
                is_required: input.required,
                position,
                created_at: now,
                updated_at: now,
            },
        };
        items.push(item);
    }
    items
}
