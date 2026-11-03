use aphel_sdk::workflow::WorkflowTraits;
use std::collections::BTreeSet;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::PipelineStage;
use crate::guidance::domain::{CHECKLIST_PATH_PREFIX, STAGE_CHECKLIST_INCOMPLETE};
use crate::guidance::store::GuidanceStore;
use crate::requirements::{ensure_stage_requirements, needs_custom_values};
use crate::store::PipelineStore;
use aphel_kernel::application::fields::{
    CUSTOM_FIELDS_PREFIX, CustomFieldReader, FieldSet, WriteMode,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::fields::{MySqlCustomFieldPorts, MySqlFieldDefinitionRepository};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;

fn field_value<'a>(values: &'a Value, field: &str) -> Option<&'a Value> {
    values
        .get(field)
        .or_else(|| values.get(CUSTOM_FIELDS_PREFIX)?.get(field))
}

fn field_change<'a>(
    write: &'a aphel_kernel::application::fields::RecordWrite<'_>,
    field: &str,
) -> Option<&'a aphel_kernel::application::shared::FieldChange> {
    write
        .changed(field)
        .or_else(|| write.changed(&format!("{CUSTOM_FIELDS_PREFIX}.{field}")))
}

fn uuid_of(value: Option<&Value>) -> Option<Uuid> {
    value
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
}

fn text_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

async fn is_advance(
    workflow: &PipelineStore,
    from: Uuid,
    to: Uuid,
    old_status: &str,
    new_status: &str,
    mapping: &aphel_sdk::workflow::StagedMapping,
) -> Result<bool, AppError> {
    if mapping
        .completed_states
        .iter()
        .any(|state| state == new_status)
    {
        return Ok(!mapping
            .completed_states
            .iter()
            .any(|state| state == old_status));
    }
    if mapping
        .active_state
        .as_deref()
        .is_some_and(|state| old_status != state || new_status != state)
        || from == to
    {
        return Ok(false);
    }
    let (Some(from), Some(to)) = (
        workflow.find_stage(from).await?,
        workflow.find_stage(to).await?,
    ) else {
        return Ok(false);
    };
    Ok(from.pipeline_id == to.pipeline_id && to.position > from.position)
}

pub(crate) async fn check_stage_checklist(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    let write = args.write();
    let Some(mapping) = args
        .registry
        .get(&write.record.record_type)
        .and_then(|definition| definition.capabilities.stage_mapping())
    else {
        return Ok(());
    };
    let stage_field = mapping.stage_field.as_str();
    let status_field = mapping.state_field.as_deref().unwrap_or("");
    if write.mode != WriteMode::Update {
        return Ok(());
    }
    let stage_change = field_change(&write, stage_field);
    let status_change = field_change(&write, status_field);
    if stage_change.is_none() && status_change.is_none() {
        return Ok(());
    }
    let Some(to) = uuid_of(field_value(write.values, stage_field)) else {
        return Ok(());
    };
    let from = match stage_change {
        Some(change) => match uuid_of(Some(&change.old)) {
            Some(from) => from,
            None => return Ok(()),
        },
        None => to,
    };
    let new_status = text_of(field_value(write.values, status_field)).unwrap_or_default();
    let old_status = match status_change {
        Some(change) => text_of(Some(&change.old)).unwrap_or(new_status),
        None => new_status,
    };
    let workflow = PipelineStore::new(ctx.pool.clone());
    if !is_advance(&workflow, from, to, old_status, new_status, &mapping).await? {
        return Ok(());
    }
    let guidance = GuidanceStore::new(ctx.pool.clone());
    let required: Vec<_> = guidance
        .items(from)
        .await?
        .into_iter()
        .filter(|item| item.is_required)
        .collect();
    if required.is_empty() {
        return Ok(());
    }
    let done: BTreeSet<Uuid> = guidance
        .progress(&write.record.record_type, write.record.record_id)
        .await?
        .into_iter()
        .map(|progress| progress.item_id)
        .collect();
    let missing: Vec<FieldViolation> = required
        .iter()
        .filter(|item| !done.contains(&item.id))
        .map(|item| {
            FieldViolation::new(
                format!("{CHECKLIST_PATH_PREFIX}{}", item.key),
                item.label.clone(),
            )
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(AppError::invalid_fields_code(
        STAGE_CHECKLIST_INCOMPLETE,
        "the checklist of the current stage has required items that are not done",
        missing,
    ))
}

fn has_requirements(from: Option<&PipelineStage>, to: &PipelineStage) -> bool {
    from.is_some_and(|stage| !stage.exit_required_fields.is_empty())
        || !to.entry_required_fields.is_empty()
}

pub(crate) async fn check_stage_requirements(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    let Some(mapping) = args
        .registry
        .get(&args.record_type)
        .and_then(|definition| definition.capabilities.stage_mapping())
    else {
        return Ok(());
    };
    let stage_field = mapping.stage_field.as_str();
    let write = args.write();
    let (from_id, to_id) = match args.mode {
        WriteMode::Create => (None, uuid_of(field_value(&args.values, stage_field))),
        WriteMode::Update => match field_change(&write, stage_field) {
            Some(change) => (uuid_of(Some(&change.old)), uuid_of(Some(&change.new))),
            None => return Ok(()),
        },
    };
    let Some(to_id) = to_id else {
        return Ok(());
    };
    let workflow = PipelineStore::new(ctx.pool.clone());
    let Some(to) = workflow.find_stage(to_id).await? else {
        return Ok(());
    };
    let from = match from_id {
        Some(id) => workflow.find_stage(id).await?,
        None => None,
    };
    if !has_requirements(from.as_ref(), &to) {
        return Ok(());
    }
    let mut custom = Map::new();
    if needs_custom_values(from.as_ref(), &to) {
        if args.mode == WriteMode::Update {
            custom = CustomFieldReader::new(&MySqlCustomFieldPorts::new(
                ctx.pool.clone(),
                args.registry.clone(),
            ))
            .load_unrestricted(&args.record_type, &[args.record.record_id])
            .await?
            .remove(&args.record.record_id)
            .unwrap_or_default();
        }
        let prefix = format!("{CUSTOM_FIELDS_PREFIX}.");
        for change in &args.changes {
            if let Some(key) = change.field.strip_prefix(&prefix) {
                custom.insert(key.to_owned(), change.new.clone());
            }
        }
    }
    if ensure_stage_requirements(from.as_ref(), &to, &args.values, &custom, None).is_ok() {
        return Ok(());
    }
    let fields = match args.registry.get(&args.record_type) {
        Some(definition) => Some(
            FieldSet::for_scope(
                &MySqlFieldDefinitionRepository::new(ctx.pool.clone()),
                definition,
                ctx.scope,
            )
            .await?,
        ),
        None => None,
    };
    ensure_stage_requirements(from.as_ref(), &to, &args.values, &custom, fields.as_ref())
}
