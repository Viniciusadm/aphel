use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeMap;

use serde_json::{Map, Number, Value, json};
use uuid::Uuid;

use crate::model::{
    FULL_NAME_TARGET, HONEYPOT_FIELD, LeadFormField, LeadFormFieldKind, LeadFormSubmissionStatus,
    LeadFormTargetField,
};
use crate::store::{self, NewSubmission, StoredForm};
use crate::validate::{can, contact_fields};
use crate::{KEY, SUBMITTED, error_code, error_fields, scope_for_user};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::RecordCtx;
use aphel_sdk::leads::{Lead, LeadConversion, LeadConverting};
use aphel_sdk::platform::application::query::{
    Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, RecordRef, RecordType,
};
use aphel_sdk::platform::domain::access::Permission;

pub(crate) const MAX_BODY_BYTES: usize = 64 * 1024;
pub(crate) const INVALID_PAYLOAD: &str = "lead_form_invalid_payload";
pub(crate) const PAYLOAD_TOO_LARGE: &str = "lead_form_payload_too_large";
pub(crate) const INVALID_SUBMISSION: &str = "lead_form_invalid_submission";
pub(crate) const OWNER_UNAVAILABLE: &str = "lead_form_owner_unavailable";
pub(crate) const SUBMISSION_FAILED: &str = "lead_form_submission_failed";
pub(crate) const TAG_FAILED: &str = "lead_form_tag_failed";
pub(crate) const DEAL_FAILED: &str = "lead_form_deal_failed";
pub(crate) const DISTRIBUTION_FAILED: &str = "lead_form_distribution_failed";

const MAX_STORED_VALUE_CHARS: usize = 1000;
const MAX_ERROR_MESSAGE_CHARS: usize = 500;
const LIFECYCLE_STAGE_TARGET: &str = "lifecycle_stage";
const LEAD_STAGE: &str = "lead";

pub(crate) struct SubmissionContext {
    pub(crate) ip_address: Option<String>,
    pub(crate) origin: Option<String>,
}

pub(crate) type Values = BTreeMap<String, Value>;

pub(crate) fn parse_payload(
    content_type: &str,
    body: &[u8],
) -> Result<Map<String, Value>, AppError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(AppError::validation_code(
            PAYLOAD_TOO_LARGE,
            "o envio excede o tamanho máximo de 64 KB",
        ));
    }
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if media_type == "application/x-www-form-urlencoded" {
        let mut values = Map::new();
        for (key, value) in form_urlencoded::parse(body) {
            values.insert(key.into_owned(), Value::String(value.into_owned()));
        }
        return Ok(values);
    }
    if media_type.is_empty()
        || media_type == "application/json"
        || media_type == "text/plain"
        || media_type.ends_with("+json")
    {
        return match serde_json::from_slice::<Value>(body) {
            Ok(Value::Object(values)) => Ok(values),
            _ => Err(AppError::validation_code(
                INVALID_PAYLOAD,
                "o corpo precisa ser um objeto JSON",
            )),
        };
    }
    Err(AppError::validation_code(
        INVALID_PAYLOAD,
        "tipo de conteúdo não suportado",
    ))
}

pub(crate) fn honeypot(payload: &Map<String, Value>) -> Option<String> {
    match payload.get(HONEYPOT_FIELD) {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(truncate(value, 200)),
        Some(Value::Null) | Some(Value::String(_)) | None => None,
        Some(other) => Some(truncate(&other.to_string(), 200)),
    }
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn raw_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        _ => String::new(),
    }
}

fn checked(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_i64() == Some(1),
        Some(Value::String(text)) => matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "on" | "true" | "1" | "yes" | "sim"
        ),
        _ => false,
    }
}

fn sanitize(text: &str, kind: LeadFormFieldKind) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control() || (kind == LeadFormFieldKind::Textarea && *c == '\n'))
        .collect();
    cleaned.trim().to_owned()
}

fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    value.len() <= 254
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(char::is_whitespace)
}

fn is_phone(value: &str) -> bool {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    (8..=15).contains(&digits)
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | ' ' | '.'))
}

pub(crate) fn validate_values(
    fields: &[LeadFormField],
    payload: &Map<String, Value>,
) -> Result<Values, Vec<FieldViolation>> {
    let mut values = Values::new();
    let mut violations = Vec::new();
    for field in fields {
        let raw = payload.get(&field.key);
        if field.kind == LeadFormFieldKind::Checkbox {
            let flag = checked(raw);
            if field.required && !flag {
                violations.push(FieldViolation::new(&field.key, "marque esta opção"));
            }
            values.insert(field.key.clone(), Value::Bool(flag));
            continue;
        }
        let text = sanitize(&raw_text(raw), field.kind);
        if text.is_empty() {
            if field.required {
                violations.push(FieldViolation::new(&field.key, "campo obrigatório"));
            }
            continue;
        }
        let violation = if text.chars().count() > field.kind.max_length() {
            Some(format!(
                "use no máximo {} caracteres",
                field.kind.max_length()
            ))
        } else {
            match field.kind {
                LeadFormFieldKind::Email if !is_email(&text) => {
                    Some("informe um e-mail válido".to_owned())
                }
                LeadFormFieldKind::Phone if !is_phone(&text) => {
                    Some("informe um telefone válido".to_owned())
                }
                LeadFormFieldKind::Select
                    if !field.options.iter().any(|option| option.value == text) =>
                {
                    Some("escolha uma das opções".to_owned())
                }
                _ => None,
            }
        };
        match violation {
            Some(message) => violations.push(FieldViolation::new(&field.key, message)),
            None => {
                values.insert(field.key.clone(), Value::String(text));
            }
        }
    }
    if violations.is_empty() {
        Ok(values)
    } else {
        Err(violations)
    }
}

pub(crate) fn stored_data(fields: &[LeadFormField], payload: &Map<String, Value>) -> Value {
    let mut data = Map::new();
    for field in fields {
        if let Some(value) = payload.get(&field.key) {
            let value = match value {
                Value::String(text) => Value::String(truncate(text, MAX_STORED_VALUE_CHARS)),
                Value::Bool(_) | Value::Number(_) | Value::Null => value.clone(),
                other => Value::String(truncate(&other.to_string(), MAX_STORED_VALUE_CHARS)),
            };
            data.insert(field.key.clone(), value);
        }
    }
    Value::Object(data)
}

fn as_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(true) => "sim".to_owned(),
        Value::Bool(false) => "não".to_owned(),
        other => other.to_string(),
    }
}

fn converted(value: &Value, field_type: &str) -> Option<Value> {
    match field_type {
        "checkbox" => Some(Value::Bool(value.as_bool().unwrap_or(false))),
        "number" | "currency" => {
            let text = as_text(value).replace(',', ".");
            text.trim()
                .parse::<f64>()
                .ok()
                .and_then(Number::from_f64)
                .map(Value::Number)
        }
        _ => Some(Value::String(as_text(value))),
    }
}

fn contact_values(
    form: &StoredForm,
    values: &Values,
    targets: &[LeadFormTargetField],
) -> Map<String, Value> {
    let mut contact = Map::new();
    let mut full_name = None;
    for field in &form.definition.fields {
        let (Some(target), Some(value)) = (&field.target, values.get(&field.key)) else {
            continue;
        };
        if target == FULL_NAME_TARGET {
            full_name = Some(as_text(value));
            continue;
        }
        let Some(info) = targets.iter().find(|info| &info.key == target) else {
            continue;
        };
        if let Some(value) = converted(value, &info.field_type) {
            contact.insert(target.clone(), value);
        }
    }
    if let Some(full_name) = full_name {
        let mut parts = full_name.trim().splitn(2, char::is_whitespace);
        let first = parts.next().unwrap_or_default().trim().to_owned();
        let last = parts.next().unwrap_or_default().trim().to_owned();
        if !first.is_empty() && !contact.contains_key("first_name") {
            contact.insert("first_name".to_owned(), Value::String(first));
        }
        if !last.is_empty() && !contact.contains_key("last_name") {
            contact.insert("last_name".to_owned(), Value::String(last));
        }
    }
    if let Some(source) = &form.definition.source
        && !contact.contains_key("source")
        && targets.iter().any(|info| info.key == "source")
    {
        contact.insert("source".to_owned(), Value::String(source.clone()));
    }
    if !contact.contains_key(LIFECYCLE_STAGE_TARGET)
        && targets
            .iter()
            .any(|info| info.key == LIFECYCLE_STAGE_TARGET)
    {
        contact.insert(
            LIFECYCLE_STAGE_TARGET.to_owned(),
            Value::String(LEAD_STAGE.to_owned()),
        );
    }
    contact
}

fn record_body(values: Map<String, Value>, targets: &[LeadFormTargetField]) -> Value {
    let mut body = Map::new();
    let mut custom = Map::new();
    for (key, value) in values {
        if targets
            .iter()
            .any(|target| target.key == key && target.custom)
        {
            custom.insert(key, value);
        } else {
            body.insert(key, value);
        }
    }
    if !custom.is_empty() {
        body.insert("custom_fields".to_owned(), Value::Object(custom));
    }
    Value::Object(body)
}

fn saved_id(record: &Value, record_type: &str) -> Result<Uuid, AppError> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal(format!("the created {record_type} has no id")))
}

async fn create_record(ctx: &Ctx<'_>, record_type: &str, body: Value) -> Result<Uuid, AppError> {
    let state = ctx.state();
    let created = ctx
        .registry()
        .handler(record_type)?
        .create(&RecordCtx::new(ctx.scope, &state), body)
        .await?;
    saved_id(&created, record_type)
}

pub(crate) fn render_template(template: &str, form: &StoredForm, values: &Values) -> String {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            output.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let key = after[..end].trim();
        if key == "form" {
            output.push_str(&form.name);
        } else if let Some(value) = values.get(key) {
            output.push_str(&as_text(value));
        }
        rest = &after[end + 2..];
    }
    output.push_str(rest);
    let title = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        truncate(&form.name, 255)
    } else {
        truncate(&title, 255)
    }
}

fn email_value(form: &StoredForm, values: &Values) -> Option<String> {
    form.definition
        .fields
        .iter()
        .find(|field| field.target.as_deref() == Some("email"))
        .and_then(|field| values.get(&field.key))
        .map(as_text)
        .filter(|email| !email.is_empty())
}

async fn existing_contact(ctx: &Ctx<'_>, email: &str) -> Result<Option<Uuid>, AppError> {
    let page = RecordQueryEngine::new(&ctx.query())
        .query(
            ctx.scope,
            RecordType::CONTACT.as_str(),
            RecordQuery {
                filter: Some(Filter::condition("email", FilterOperator::Eq, json!(email))),
                window: aphel_sdk::platform::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..RecordQuery::default()
            },
        )
        .await?;
    Ok(page.items.into_iter().next())
}

fn submission_error(form: &StoredForm, error: &AppError) -> AppError {
    let fields: Vec<FieldViolation> = error_fields(error)
        .iter()
        .filter_map(|violation| {
            form.definition
                .fields
                .iter()
                .find(|field| {
                    field.target.as_deref().is_some_and(|target| {
                        violation.field == target
                            || violation.field.ends_with(&format!(".{target}"))
                    })
                })
                .map(|field| FieldViolation::new(&field.key, "valor não aceito"))
        })
        .collect();
    if fields.is_empty() {
        AppError::unavailable_code(
            SUBMISSION_FAILED,
            "não foi possível registrar o envio agora; tente novamente mais tarde",
        )
    } else {
        AppError::invalid_fields_code(INVALID_SUBMISSION, "revise os campos destacados", fields)
    }
}

fn error_text(error: &AppError) -> String {
    truncate(&error.to_string(), MAX_ERROR_MESSAGE_CHARS)
}

pub(crate) async fn record_spam(
    ctx: &Ctx<'_>,
    form: &StoredForm,
    payload: &Map<String, Value>,
    trap: String,
    context: &SubmissionContext,
) -> Result<(), AppError> {
    let mut data = stored_data(&form.definition.fields, payload);
    if let Value::Object(map) = &mut data {
        map.insert(HONEYPOT_FIELD.to_owned(), Value::String(trap));
    }
    store::insert_submission(
        ctx.pool(),
        &NewSubmission {
            id: Uuid::now_v7(),
            form_id: form.id,
            status: LeadFormSubmissionStatus::Spam,
            contact_id: None,
            deal_id: None,
            contact_created: false,
            data,
            ip_address: context.ip_address.clone(),
            origin: context.origin.clone(),
            error_code: None,
            error_message: None,
        },
    )
    .await
}

async fn record_failure(
    ctx: &Ctx<'_>,
    form: &StoredForm,
    data: Value,
    context: &SubmissionContext,
    code: &str,
    message: String,
) -> Result<(), AppError> {
    store::insert_submission(
        ctx.pool(),
        &NewSubmission {
            id: Uuid::now_v7(),
            form_id: form.id,
            status: LeadFormSubmissionStatus::Failed,
            contact_id: None,
            deal_id: None,
            contact_created: false,
            data,
            ip_address: context.ip_address.clone(),
            origin: context.origin.clone(),
            error_code: Some(code.to_owned()),
            error_message: Some(truncate(&message, MAX_ERROR_MESSAGE_CHARS)),
        },
    )
    .await
}

async fn create_deal(
    ctx: &Ctx<'_>,
    form: &StoredForm,
    values: &Values,
    contact_id: Uuid,
    owner_id: Option<&str>,
) -> Result<Option<Uuid>, AppError> {
    let Some(deal) = &form.definition.deal else {
        return Ok(None);
    };
    let mut deal_values = Map::new();
    deal_values.insert(
        "title".to_owned(),
        Value::String(render_template(&deal.title_template, form, values)),
    );
    deal_values.insert("pipeline_id".to_owned(), json!(deal.pipeline_id));
    deal_values.insert("stage_id".to_owned(), json!(deal.stage_id));
    deal_values.insert("contact_id".to_owned(), json!(contact_id.to_string()));
    if let Some(owner_id) = owner_id {
        deal_values.insert("owner_id".to_owned(), json!(owner_id));
    }
    if let Some(amount) = &deal.amount {
        deal_values.insert("amount".to_owned(), json!(amount));
    }
    if let Some(source) = &form.definition.source {
        deal_values.insert("source".to_owned(), json!(source));
    }
    let lead = Lead::from_record(
        ctx.load_record(RecordType::CONTACT.as_str(), contact_id)
            .await?
            .ok_or_else(|| AppError::NotFound("the created person is unavailable".into()))?,
    )?;
    let has_company = lead.company_id().is_some();
    let mut tx = ctx.begin().await?;
    let conversion = ctx
        .apply_hook(
            LeadConverting,
            &mut tx,
            KEY,
            LeadConversion {
                lead,
                deal: deal_values,
                company: None,
            },
        )
        .await?;
    tx.commit().await?;
    let mut deal_values = conversion.deal;
    if let Some(company) = conversion.company
        && !has_company
    {
        let company_id =
            create_record(ctx, RecordType::COMPANY.as_str(), Value::Object(company)).await?;
        deal_values.insert("company_id".to_owned(), json!(company_id.to_string()));
    }
    create_record(ctx, RecordType::DEAL.as_str(), Value::Object(deal_values))
        .await
        .map(Some)
}

async fn resolve_owner(
    ctx: &Ctx<'_>,
    form: &StoredForm,
    values: &Map<String, Value>,
    submission_id: Uuid,
    warnings: &mut Vec<(&'static str, String)>,
) -> Option<String> {
    let fixed = form.definition.owner_id.map(|id| id.to_string());
    let Some(rule_id) = &form.definition.distribution_rule_id else {
        return fixed;
    };
    let selector = match aphel_sdk::assignment::selector(ctx) {
        Ok(Some(selector)) => selector,
        Ok(None) => {
            warnings.push((
                DISTRIBUTION_FAILED,
                "the lead distribution module is disabled".to_owned(),
            ));
            return fixed;
        }
        Err(error) => {
            warnings.push((DISTRIBUTION_FAILED, error_text(&error)));
            return fixed;
        }
    };
    match selector
        .select(
            ctx,
            rule_id,
            RecordType::CONTACT.as_str(),
            values,
            "lead_form",
            format!("{KEY}:{submission_id}"),
        )
        .await
    {
        Ok(outcome) => match outcome {
            Some(user_id) => Some(user_id.as_str().to_owned()),
            None => {
                warnings.push((
                    DISTRIBUTION_FAILED,
                    "no member of the lead distribution rule was available".to_owned(),
                ));
                fixed
            }
        },
        Err(error) => {
            warnings.push((DISTRIBUTION_FAILED, error_text(&error)));
            fixed
        }
    }
}

pub(crate) async fn accept(
    ctx: &Ctx<'_>,
    form: &StoredForm,
    payload: &Map<String, Value>,
    values: &Values,
    context: &SubmissionContext,
) -> Result<(), AppError> {
    let data = stored_data(&form.definition.fields, payload);
    let actor = scope_for_user(ctx, form.acting_user())
        .await?
        .filter(|scope| can(scope, &Permission::CONTACTS_CREATE))
        .filter(|scope| form.definition.deal.is_none() || can(scope, &Permission::DEALS_CREATE));
    let Some(actor) = actor else {
        record_failure(
            ctx,
            form,
            data,
            context,
            OWNER_UNAVAILABLE,
            format!(
                "the acting user {} is no longer a member or lost the permission to create people or deals",
                form.acting_user()
            ),
        )
        .await?;
        return Err(AppError::unavailable_code(
            OWNER_UNAVAILABLE,
            "este formulário não está aceitando envios no momento",
        ));
    };
    let ctx = ctx.with_scope(&actor);
    let targets = contact_fields(&ctx, &actor).await?;
    let mut contact_id = None;
    if form.definition.dedupe_by_email
        && let Some(email) = email_value(form, values)
    {
        contact_id = existing_contact(&ctx, &email).await.ok().flatten();
    }
    let contact_created = contact_id.is_none();
    let submission_id = Uuid::now_v7();
    let mut warnings = Vec::new();
    let mut contact_map = contact_values(form, values, &targets);
    let owner_id = if contact_created || form.definition.deal.is_some() {
        resolve_owner(&ctx, form, &contact_map, submission_id, &mut warnings).await
    } else {
        None
    };
    if let Some(owner_id) = &owner_id {
        contact_map.insert("owner_id".to_owned(), json!(owner_id));
    }
    let contact_id = match contact_id {
        Some(id) => id,
        None => {
            let contact = record_body(contact_map, &targets);
            match create_record(&ctx, RecordType::CONTACT.as_str(), contact).await {
                Ok(id) => id,
                Err(error) => {
                    record_failure(
                        &ctx,
                        form,
                        data,
                        context,
                        error_code(&error),
                        error_text(&error),
                    )
                    .await?;
                    return Err(submission_error(form, &error));
                }
            }
        }
    };
    if let Some(tag_id) = &form.definition.tag_id
        && let Err(error) = ctx
            .call(
                aphel_sdk::tagging::APPLY,
                aphel_sdk::tagging::TagRecord {
                    record_type: RecordType::CONTACT.as_str().to_owned(),
                    record_id: contact_id.to_string(),
                    tag_id: tag_id.clone(),
                },
            )
            .await
    {
        warnings.push((TAG_FAILED, error_text(&error)));
    }
    let deal_id = match create_deal(&ctx, form, values, contact_id, owner_id.as_deref()).await {
        Ok(deal_id) => deal_id,
        Err(error) => {
            warnings.push((DEAL_FAILED, error_text(&error)));
            None
        }
    };
    let (error_code, error_message) = match warnings.first() {
        Some((code, _)) => (
            Some((*code).to_owned()),
            Some(truncate(
                &warnings
                    .iter()
                    .map(|(_, message)| message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
                MAX_ERROR_MESSAGE_CHARS,
            )),
        ),
        None => (None, None),
    };
    let contact = RecordRef::new(RecordType::CONTACT, contact_id);
    let actor = Actor::from_scope(&actor);
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    store::insert_submission(
        &mut *tx,
        &NewSubmission {
            id: submission_id,
            form_id: form.id,
            status: LeadFormSubmissionStatus::Accepted,
            contact_id: Some(contact_id),
            deal_id,
            contact_created,
            data,
            ip_address: context.ip_address.clone(),
            origin: context.origin.clone(),
            error_code,
            error_message,
        },
    )
    .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), SUBMITTED, now)
                .with_record(contact.clone())
                .with_payload(json!({
                    "form_id": form.id,
                    "form_name": form.name,
                    "submission_id": submission_id,
                    "contact_id": contact_id,
                    "contact_created": contact_created,
                    "deal_id": deal_id,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, SUBMITTED, now)
                .with_record(contact)
                .with_changes(vec![
                    FieldChange::new(
                        "lead_form",
                        Value::Null,
                        json!({ "id": form.id, "name": form.name }),
                    ),
                    FieldChange::new("submission_id", Value::Null, json!(submission_id)),
                ]),
        )
        .await?;
    Ok(tx.commit().await?)
}
