use crate::incoming::{
    ActionError, ActionFieldError, ActionKind, ActionOutcome, ActionStatus, MappingExpression,
    WebhookAction,
};
use aphel_kernel::{
    Ctx,
    application::shared::RecordType,
    errors::{AppError, CodedKind},
};
use serde_json::{Map, Value, json};
use uuid::Uuid;
pub(crate) const CREATED: &str = "created";
pub(crate) const UPDATED: &str = "updated";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    Mapping,
    Permission,
    Internal,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionFailure {
    pub(crate) kind: FailureKind,
    pub(crate) error: ActionError,
}
#[derive(Debug, Default)]
pub(crate) struct ExecutionContext {
    records: Vec<Value>,
}
pub(crate) fn failure_from_error(error: AppError) -> ActionFailure {
    let (kind, code, fields) = match &error {
        AppError::InvalidFields { fields, .. } => {
            (FailureKind::Mapping, "validation_error", fields.clone())
        }
        AppError::CodedFields { code, fields, .. } => (FailureKind::Mapping, *code, fields.clone()),
        AppError::Validation(_) => (FailureKind::Mapping, "validation_error", Vec::new()),
        AppError::NotFound(_) => (FailureKind::Mapping, "not_found", Vec::new()),
        AppError::Conflict(_) => (FailureKind::Mapping, "conflict", Vec::new()),
        AppError::Coded { kind, code, .. } | AppError::CodedDetails { kind, code, .. } => {
            match kind {
                CodedKind::Forbidden | CodedKind::Unauthorized => {
                    (FailureKind::Permission, *code, Vec::new())
                }
                _ => (FailureKind::Mapping, *code, Vec::new()),
            }
        }
        AppError::Forbidden(_) | AppError::Unauthorized(_) => {
            (FailureKind::Permission, "permission_denied", Vec::new())
        }
        AppError::RateLimited(_)
        | AppError::Configuration(_)
        | AppError::Persistence(_)
        | AppError::Internal(_) => {
            tracing::error!(error = %error, "incoming webhook action failed");
            return ActionFailure {
                kind: FailureKind::Internal,
                error: ActionError {
                    code: "internal_error".to_owned(),
                    message: "internal server error".to_owned(),
                    fields: Vec::new(),
                },
            };
        }
    };
    ActionFailure {
        kind,
        error: ActionError {
            code: code.to_owned(),
            message: error.client_message().to_owned(),
            fields: fields
                .into_iter()
                .map(|violation| ActionFieldError {
                    field: violation.field,
                    message: violation.message,
                })
                .collect(),
        },
    }
}

pub(crate) fn outcome(index: usize, action: &WebhookAction, status: ActionStatus) -> ActionOutcome {
    ActionOutcome {
        index,
        action_type: action.kind(),
        status,
        operation: None,
        record_type: None,
        record_id: None,
        error: None,
    }
}

pub(crate) async fn run_action(
    ctx: &Ctx<'_>,
    action: &WebhookAction,
    payload: &Value,
    context: &mut ExecutionContext,
    _ip: Option<String>,
) -> Result<(RecordType, Uuid, &'static str), ActionFailure> {
    let catalog = super::FieldCatalog::load(ctx)
        .await
        .map_err(failure_from_error)?;
    let fields = catalog.fields.get(&action.record_type).ok_or_else(|| {
        failure_from_error(AppError::Forbidden(
            "the target entity is unavailable".into(),
        ))
    })?;
    let input = json!({"payload": payload, "actions": context.records});
    let mut body = Map::new();
    let mut custom = Map::new();
    for (key, raw) in action.fields() {
        let field = fields
            .active()
            .find(|f| f.key() == key && f.is_editable())
            .ok_or_else(|| {
                failure_from_error(AppError::Forbidden(format!("field {key} is not editable")))
            })?;
        let expression = MappingExpression::parse(raw).map_err(|message| ActionFailure {
            kind: FailureKind::Mapping,
            error: ActionError {
                code: "mapping_failed".into(),
                message,
                fields: vec![ActionFieldError {
                    field: key.clone(),
                    message: "invalid mapping".into(),
                }],
            },
        })?;
        if let Some(value) = expression.evaluate(&input) {
            if field.is_system() {
                body.insert(key.clone(), value);
            } else {
                custom.insert(key.clone(), value);
            }
        }
    }
    let existing = if action.kind() == ActionKind::UpsertRecord {
        let key = action.match_field.as_deref().ok_or_else(|| {
            failure_from_error(AppError::Validation("upsert requires a match field".into()))
        })?;
        if !fields
            .active()
            .any(|f| f.key() == key && f.is_editable() && f.filterable() && f.definition.is_unique)
        {
            return Err(failure_from_error(AppError::Validation(
                "upsert requires a unique editable match field".into(),
            )));
        }
        let value = body
            .get(key)
            .or_else(|| custom.get(key))
            .filter(|v| !v.is_null() && !v.as_str().is_some_and(|s| s.trim().is_empty()))
            .ok_or_else(|| {
                failure_from_error(AppError::Validation("the match value is required".into()))
            })?;
        super::gateway::find_record(ctx, &action.record_type, key, value.clone())
            .await
            .map_err(failure_from_error)?
    } else {
        None
    };
    let assignment_rule = if existing.is_none() && !body.contains_key("owner_id") {
        action.assignment_rule_id.as_ref()
    } else {
        None
    };
    if assignment_rule.is_some()
        && !fields
            .active()
            .any(|f| f.key() == "owner_id" && f.is_editable())
    {
        return Err(failure_from_error(AppError::Forbidden(
            "the owner field is not editable".into(),
        )));
    }
    let routing_values = {
        let mut values = custom.clone();
        values.extend(body.clone());
        values
    };
    if !custom.is_empty() {
        body.insert("custom_fields".into(), Value::Object(custom));
    }
    let assigned = match assignment_rule {
        Some(rule) => super::gateway::create_with_owner(
            ctx,
            rule,
            &action.record_type,
            &routing_values,
            Value::Object(body.clone()),
        )
        .await
        .map_err(failure_from_error)?,
        None => None,
    };
    let (saved, operation) = if let Some(record) = assigned {
        (Ok(json!({"id": record.record_id})), CREATED)
    } else if let Some(id) = existing {
        (
            ctx.update_record(&action.record_type, id, Value::Object(body))
                .await,
            UPDATED,
        )
    } else {
        (
            ctx.create_record(&action.record_type, Value::Object(body))
                .await,
            CREATED,
        )
    };
    let saved = saved.map_err(failure_from_error)?;
    let id = saved
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| {
            failure_from_error(AppError::Internal("the saved record has no id".into()))
        })?;
    context
        .records
        .push(json!({"id": id, "record_type": action.record_type, "operation": operation}));
    Ok((
        RecordType::parse(action.record_type.clone())
            .map_err(|_| failure_from_error(AppError::Validation("invalid entity key".into())))?,
        id,
        operation,
    ))
}
