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
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::fields::{MySqlCustomFieldPorts, MySqlFieldDefinitionRepository};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::ChangedRecord;

const OPEN_STATUS: &str = "open";
const WON_STATUS: &str = "won";
const STAGE_FIELD: &str = "stage_id";
const STATUS_FIELD: &str = "status";

fn uuid_of(value: Option<&Value>) -> Option<Uuid> {
    value
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
}

fn text_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

async fn is_advance(
    pipelines: &PipelineStore,
    from: Uuid,
    to: Uuid,
    old_status: &str,
    new_status: &str,
) -> Result<bool, AppError> {
    if new_status == WON_STATUS {
        return Ok(old_status != WON_STATUS);
    }
    if old_status != OPEN_STATUS || new_status != OPEN_STATUS || from == to {
        return Ok(false);
    }
    let (Some(from), Some(to)) = (
        pipelines.find_stage(from).await?,
        pipelines.find_stage(to).await?,
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
    if write.record.record_type != RecordType::DEAL || write.mode != WriteMode::Update {
        return Ok(());
    }
    let stage_change = write.changed(STAGE_FIELD);
    let status_change = write.changed(STATUS_FIELD);
    if stage_change.is_none() && status_change.is_none() {
        return Ok(());
    }
    let Some(to) = uuid_of(write.values.get(STAGE_FIELD)) else {
        return Ok(());
    };
    let from = match stage_change {
        Some(change) => match uuid_of(Some(&change.old)) {
            Some(from) => from,
            None => return Ok(()),
        },
        None => to,
    };
    let Some(new_status) = text_of(write.values.get(STATUS_FIELD)) else {
        return Ok(());
    };
    let old_status = match status_change {
        Some(change) => text_of(Some(&change.old)).unwrap_or(new_status),
        None => new_status,
    };
    let pipelines = PipelineStore::new(ctx.pool.clone());
    if !is_advance(&pipelines, from, to, old_status, new_status).await? {
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
        .progress(write.record.record_id)
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
    if args.record_type != RecordType::DEAL {
        return Ok(());
    }
    let write = args.write();
    let (from_id, to_id) = match args.mode {
        WriteMode::Create => (None, uuid_of(args.values.get(STAGE_FIELD))),
        WriteMode::Update => match write.changed(STAGE_FIELD) {
            Some(change) => (uuid_of(Some(&change.old)), uuid_of(Some(&change.new))),
            None => return Ok(()),
        },
    };
    let Some(to_id) = to_id else {
        return Ok(());
    };
    let pipelines = PipelineStore::new(ctx.pool.clone());
    let Some(to) = pipelines.find_stage(to_id).await? else {
        return Ok(());
    };
    let from = match from_id {
        Some(id) => pipelines.find_stage(id).await?,
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
            .load_unrestricted(&RecordType::DEAL, &[args.record.record_id])
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
    let fields = match args.registry.get(&RecordType::DEAL) {
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
