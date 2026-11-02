use aphel_contract_tagging as tagging;
use std::sync::Arc;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::BulkAction;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordCapabilities;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordCtx, RecordTypeHandler};
use sequences::EnrollCommand;
use sequences::domain::EnrollmentSource;

fn unsupported_action(action: &BulkAction, record_type: &str) -> AppError {
    AppError::validation_code(
        "bulk_action_not_supported",
        format!(
            "{} cannot be applied to {record_type} records",
            action.kind()
        ),
    )
}

fn invalid_value(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid value",
        vec![FieldViolation::new(field.to_owned(), message.to_owned())],
    )
}

pub(crate) fn value_text(field: &str, value: &Value) -> Result<Option<String>, AppError> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) if text.trim().is_empty() => Ok(None),
        Value::String(text) => Ok(Some(text.trim().to_owned())),
        Value::Number(number) => Ok(Some(number.to_string())),
        Value::Bool(flag) => Ok(Some(flag.to_string())),
        _ => Err(invalid_value(field, "must be a text, number or null")),
    }
}

pub(crate) fn bulk_change(
    capabilities: &RecordCapabilities,
    field: &str,
    value: &Value,
    is_custom: bool,
) -> Result<Value, AppError> {
    let mut body = Map::new();
    if is_custom {
        let mut custom = Map::new();
        custom.insert(field.to_owned(), value.clone());
        body.insert("custom_fields".to_owned(), Value::Object(custom));
        return Ok(Value::Object(body));
    }
    if !capabilities
        .bulk_fields
        .iter()
        .any(|allowed| allowed == field)
    {
        return Err(invalid_value(field, "cannot be changed in bulk"));
    }
    let text = value_text(field, value)?;
    body.insert(field.to_owned(), text.map_or(Value::Null, Value::String));
    Ok(Value::Object(body))
}

fn target(
    ctx: &Ctx<'_>,
    action: &BulkAction,
    record_type: &str,
) -> Result<(Arc<dyn RecordTypeHandler>, RecordCapabilities), AppError> {
    let registry = ctx.registry();
    let capabilities = registry
        .capabilities(record_type)
        .filter(|capabilities| capabilities.allows_bulk())
        .cloned()
        .ok_or_else(|| unsupported_action(action, record_type))?;
    Ok((registry.handler(record_type)?, capabilities))
}

async fn update(
    ctx: &Ctx<'_>,
    record_ctx: &RecordCtx<'_>,
    action: &BulkAction,
    record_type: &str,
    record_id: Uuid,
    (field, value, is_custom): (&str, &Value, bool),
) -> Result<(), AppError> {
    let (handler, capabilities) = target(ctx, action, record_type)?;
    let body = bulk_change(&capabilities, field, value, is_custom)?;
    handler
        .update(record_ctx, record_id, body)
        .await
        .map(|_| ())
}

pub(crate) async fn apply(
    ctx: &Ctx<'_>,
    record_ctx: &RecordCtx<'_>,
    record_type: &str,
    record_id: Uuid,
    action: &BulkAction,
) -> Result<(), AppError> {
    let id = record_id.to_string();
    match action {
        BulkAction::AssignOwner { owner_id } => {
            update(
                ctx,
                record_ctx,
                action,
                record_type,
                record_id,
                ("owner_id", &Value::String(owner_id.clone()), false),
            )
            .await
        }
        BulkAction::SetField {
            field,
            value,
            custom,
        } => {
            update(
                ctx,
                record_ctx,
                action,
                record_type,
                record_id,
                (field, value, *custom),
            )
            .await
        }
        BulkAction::AddTag { tag_id } => ctx
            .call(
                tagging::APPLY,
                tagging::TagRecord {
                    record_type: record_type.to_owned(),
                    record_id: id.clone(),
                    tag_id: tag_id.clone(),
                },
            )
            .await
            .map(|_| ()),
        BulkAction::RemoveTag { tag_id } => ctx
            .call(
                tagging::REMOVE,
                tagging::TagRecord {
                    record_type: record_type.to_owned(),
                    record_id: id.clone(),
                    tag_id: tag_id.clone(),
                },
            )
            .await
            .map(|_| ()),
        BulkAction::MoveStage { stage_id } => {
            let (handler, capabilities) = target(ctx, action, record_type)?;
            if !capabilities.supports_stages {
                return Err(unsupported_action(action, record_type));
            }
            let mut body = Map::new();
            body.insert("stage_id".to_owned(), Value::String(stage_id.clone()));
            handler
                .update(record_ctx, record_id, Value::Object(body))
                .await
                .map(|_| ())
        }
        BulkAction::EnrollInSequence { sequence_id } => {
            let (_, capabilities) = target(ctx, action, record_type)?;
            if !capabilities.supports_sequences {
                return Err(unsupported_action(action, record_type));
            }
            sequences::service::enroll(
                ctx,
                EnrollCommand {
                    sequence_id: sequence_id.clone(),
                    contact_id: id,
                    mailbox_id: None,
                    source: EnrollmentSource::Bulk,
                    enroll_as: None,
                    ip: None,
                },
            )
            .await
            .map(|_| ())
        }
        BulkAction::Archive => {
            let (handler, _) = target(ctx, action, record_type)?;
            handler.archive(record_ctx, record_id).await
        }
        BulkAction::Unarchive => {
            let (handler, _) = target(ctx, action, record_type)?;
            handler.restore(record_ctx, record_id).await
        }
        BulkAction::Delete => {
            let (handler, _) = target(ctx, action, record_type)?;
            handler.delete(record_ctx, record_id).await
        }
    }
}
