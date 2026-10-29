use serde_json::{Map, Value};
use uuid::Uuid;

use crate::incoming::gateway;
use crate::incoming::{
    ActionError, ActionFieldError, ActionKind, ActionOutcome, ActionStatus, ActivityAction,
    ContactAction, ContactMatch, DealAction, FieldCatalog, MappingExpression, WebhookAction,
    scalar_text,
};
use activities::domain::{CreateActivityCommand, RecordLinkInput};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::records::AddressInput;
use contacts::domain::{ContactInput, SaveContactCommand};
use deals::domain::DealInput;

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
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) deal_id: Option<Uuid>,
}

struct ResolvedFields {
    system: Map<String, Value>,
    custom: Map<String, Value>,
}

impl ResolvedFields {
    fn text(&self, key: &str) -> Option<String> {
        self.system
            .get(key)
            .and_then(scalar_text)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    }

    fn patch(&self, key: &str) -> Option<Option<String>> {
        self.text(key).map(Some)
    }

    fn merged(&self) -> Map<String, Value> {
        let mut values = self.custom.clone();
        values.extend(self.system.clone());
        values
    }
}

fn mapping_failure(fields: Vec<ActionFieldError>) -> ActionFailure {
    ActionFailure {
        kind: FailureKind::Mapping,
        error: ActionError {
            code: "mapping_failed".to_owned(),
            message: "the payload could not be mapped to the CRM fields".to_owned(),
            fields,
        },
    }
}

fn resolve(
    kind: ActionKind,
    mapping: &Map<String, Value>,
    payload: &Value,
) -> Result<ResolvedFields, ActionFailure> {
    let mut resolved = ResolvedFields {
        system: Map::new(),
        custom: Map::new(),
    };
    let mut errors = Vec::new();
    for (key, raw) in mapping {
        let expression = match MappingExpression::parse(raw) {
            Ok(expression) => expression,
            Err(message) => {
                errors.push(ActionFieldError {
                    field: key.clone(),
                    message,
                });
                continue;
            }
        };
        let Some(value) = expression.evaluate(payload) else {
            continue;
        };
        if FieldCatalog::is_system(kind, key) {
            if matches!(value, Value::Object(_)) {
                errors.push(ActionFieldError {
                    field: key.clone(),
                    message: "the mapped value must be a text, number or boolean".to_owned(),
                });
                continue;
            }
            resolved.system.insert(key.clone(), value);
        } else if !value.is_null() {
            resolved.custom.insert(key.clone(), value);
        }
    }
    if errors.is_empty() {
        Ok(resolved)
    } else {
        Err(mapping_failure(errors))
    }
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
    ip: Option<String>,
) -> Result<(RecordType, Uuid, &'static str), ActionFailure> {
    let fields = resolve(action.kind(), action.fields(), payload)?;
    match action {
        WebhookAction::UpsertContact(config) => {
            upsert_contact(ctx, config, fields, context, ip).await
        }
        WebhookAction::CreateDeal(config) => create_deal(ctx, config, fields, context, ip).await,
        WebhookAction::CreateActivity(config) => {
            create_activity(ctx, config, fields, context, ip).await
        }
    }
}

async fn upsert_contact(
    ctx: &Ctx<'_>,
    config: &ContactAction,
    fields: ResolvedFields,
    context: &mut ExecutionContext,
    ip: Option<String>,
) -> Result<(RecordType, Uuid, &'static str), ActionFailure> {
    let existing = existing_contact(ctx, config, &fields)
        .await
        .map_err(failure_from_error)?;
    let mut command = SaveContactCommand {
        input: ContactInput {
            first_name: fields.patch("first_name"),
            last_name: fields.patch("last_name"),
            email: fields.patch("email"),
            phone: fields.patch("phone"),
            mobile: fields.patch("mobile"),
            job_title: fields.patch("job_title"),
            company_id: fields.patch("company_id"),
            owner_id: fields.patch("owner_id"),
            address: AddressInput {
                street: fields.patch("address_street"),
                number: fields.patch("address_number"),
                complement: fields.patch("address_complement"),
                district: fields.patch("address_district"),
                city: fields.patch("address_city"),
                state: fields.patch("address_state"),
                postal_code: fields.patch("address_postal_code"),
                country: fields.patch("address_country"),
            },
            source: fields.patch("source"),
            contributed: fields.system.clone(),
            notes: fields.patch("notes"),
        },
        custom_fields: fields.custom.clone(),
        ip,
        expected_updated_at: None,
    };
    if existing.is_none()
        && command.input.owner_id.is_none()
        && let Some(rule_id) = &config.distribution_rule_id
    {
        command.input.owner_id =
            gateway::distribute_owner(ctx, rule_id, &RecordType::CONTACT, &fields.merged())
                .await
                .map_err(failure_from_error)?
                .map(Some);
    }
    let (id, operation) = match existing {
        Some(contact_id) => (
            gateway::update_contact(ctx, contact_id, command)
                .await
                .map_err(failure_from_error)?,
            UPDATED,
        ),
        None => (
            gateway::create_contact(ctx, command)
                .await
                .map_err(failure_from_error)?,
            CREATED,
        ),
    };
    context.contact_id = Some(id);
    Ok((RecordType::CONTACT, id, operation))
}

async fn existing_contact(
    ctx: &Ctx<'_>,
    config: &ContactAction,
    fields: &ResolvedFields,
) -> Result<Option<Uuid>, AppError> {
    match config.match_by {
        Some(ContactMatch::Email) => match fields.text("email") {
            Some(email) => gateway::find_contact_by_email(ctx, &email.to_lowercase()).await,
            None => Ok(None),
        },
        Some(ContactMatch::Phone) => {
            let numbers: Vec<String> = ["phone", "mobile"]
                .into_iter()
                .filter_map(|key| fields.text(key))
                .collect();
            gateway::find_contact_by_phone(ctx, &numbers).await
        }
        Some(ContactMatch::Field) => {
            let Some(key) = config.match_field.as_deref().map(str::trim) else {
                return Ok(None);
            };
            match fields.custom.get(key).filter(|value| !value.is_null()) {
                Some(value) => gateway::find_contact_by_field(ctx, key, value.clone()).await,
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

fn integer(fields: &ResolvedFields, key: &str) -> Result<Option<i64>, ActionFailure> {
    let Some(text) = fields.text(key) else {
        return Ok(None);
    };
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && value.fract() == 0.0)
        .map(|value| Some(value as i64))
        .ok_or_else(|| {
            mapping_failure(vec![ActionFieldError {
                field: key.to_owned(),
                message: "must be an integer".to_owned(),
            }])
        })
}

async fn create_deal(
    ctx: &Ctx<'_>,
    config: &DealAction,
    fields: ResolvedFields,
    context: &mut ExecutionContext,
    ip: Option<String>,
) -> Result<(RecordType, Uuid, &'static str), ActionFailure> {
    let probability = integer(&fields, "probability")?;
    let owner_id = match (fields.patch("owner_id"), &config.distribution_rule_id) {
        (None, Some(rule_id)) => {
            gateway::distribute_owner(ctx, rule_id, &RecordType::DEAL, &fields.merged())
                .await
                .map_err(failure_from_error)?
                .map(Some)
        }
        (owner_id, _) => owner_id,
    };
    let contact_id = fields
        .patch("contact_id")
        .or_else(|| context.contact_id.map(|id| Some(id.to_string())));
    let input = DealInput {
        title: fields.text("title"),
        amount: fields.patch("amount"),
        currency: fields.text("currency").map(|code| code.to_uppercase()),
        pipeline_id: Some(config.pipeline_id.clone()),
        stage_id: Some(config.stage_id.clone()),
        status: None,
        probability: probability.map(Some),
        expected_close_date: fields.patch("expected_close_date"),
        contact_id,
        company_id: fields.patch("company_id"),
        owner_id,
        source: fields.patch("source"),
        lost_reason_id: None,
        lost_reason_note: None,
        contract_start: None,
        contract_end: None,
        custom_fields: fields.custom,
        expected_updated_at: None,
        ip,
    };
    let id = gateway::create_deal(ctx, input)
        .await
        .map_err(failure_from_error)?;
    context.deal_id = Some(id);
    Ok((RecordType::DEAL, id, CREATED))
}

async fn create_activity(
    ctx: &Ctx<'_>,
    config: &ActivityAction,
    fields: ResolvedFields,
    context: &mut ExecutionContext,
    ip: Option<String>,
) -> Result<(RecordType, Uuid, &'static str), ActionFailure> {
    let duration_minutes = integer(&fields, "duration_minutes")?;
    let (record, links) = match (context.deal_id, context.contact_id) {
        (Some(deal_id), contact_id) => (
            Some((RecordType::DEAL, deal_id)),
            contact_id
                .map(|id| RecordLinkInput {
                    record_type: RecordType::CONTACT.as_str().to_owned(),
                    record_id: id.to_string(),
                })
                .into_iter()
                .collect(),
        ),
        (None, Some(contact_id)) => (Some((RecordType::CONTACT, contact_id)), Vec::new()),
        (None, None) => (None, Vec::new()),
    };
    let command = CreateActivityCommand {
        title: fields.text("title").unwrap_or_default(),
        activity_type_id: config.activity_type_id.clone(),
        status: None,
        priority: fields.text("priority"),
        due_at: fields.text("due_at"),
        completed_at: None,
        owner_id: fields.text("owner_id"),
        record_type: record.as_ref().map(|(kind, _)| kind.as_str().to_owned()),
        record_id: record.map(|(_, id)| id.to_string()),
        links,
        duration_minutes,
        description: fields.text("description"),
        location: None,
        custom_fields: fields.custom,
        ip,
    };
    let id = gateway::create_activity(ctx, command)
        .await
        .map_err(failure_from_error)?;
    Ok((RecordType::ACTIVITY, id, CREATED))
}
