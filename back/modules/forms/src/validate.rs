use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeSet;

use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};

use crate::model::{
    FULL_NAME_TARGET, LeadFormDeal, LeadFormDefinition, LeadFormField, LeadFormFieldKind,
    LeadFormPipelineOption, LeadFormRequest, LeadFormStageOption, LeadFormTargetField, MAX_FIELDS,
    MAX_KEY_LENGTH, MAX_LABEL_LENGTH, MAX_MESSAGE_LENGTH, MAX_NAME_LENGTH, MAX_OPTION_LENGTH,
    MAX_OPTIONS, MAX_ORIGINS, MAX_PLACEHOLDER_LENGTH, MAX_SOURCE_LENGTH, MAX_SUBMIT_LABEL_LENGTH,
    MAX_TEMPLATE_LENGTH, MAX_URL_LENGTH,
};
use crate::scope_for_user;

pub(crate) const INVALID_FORM: &str = "lead_form_invalid";

const MAPPABLE_TYPES: [&str; 10] = [
    "text",
    "long_text",
    "email",
    "phone",
    "url",
    "select",
    "checkbox",
    "number",
    "currency",
    "date",
];

const UNMAPPABLE_KEYS: [&str; 4] = ["owner_id", "company_id", "created_at", "updated_at"];

pub(crate) struct ValidatedForm {
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) acting_user_id: Option<Uuid>,
    pub(crate) definition: LeadFormDefinition,
}

pub(crate) async fn contact_fields(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
) -> Result<Vec<LeadFormTargetField>, AppError> {
    Ok(ctx
        .field_set(scope, "people.person")
        .await?
        .active()
        .map(|field| LeadFormTargetField {
            key: field.key().to_owned(),
            label: field.definition.label.clone(),
            field_type: field.definition.type_name().to_owned(),
            custom: !field.is_system(),
        })
        .collect())
}

pub(crate) async fn target_fields(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
) -> Result<Vec<LeadFormTargetField>, AppError> {
    Ok(contact_fields(ctx, scope)
        .await?
        .into_iter()
        .filter(|field| {
            MAPPABLE_TYPES.contains(&field.field_type.as_str())
                && !UNMAPPABLE_KEYS.contains(&field.key.as_str())
        })
        .collect())
}

pub(crate) async fn workflow(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
) -> Result<Vec<LeadFormPipelineOption>, AppError> {
    let workflow = workflow::service::list_pipelines(&ctx.with_scope(scope)).await?;
    Ok(workflow
        .into_iter()
        .filter(|view| view.pipeline.is_active)
        .map(|view| LeadFormPipelineOption {
            id: view.pipeline.id.to_string(),
            name: view.pipeline.name,
            is_default: view.pipeline.is_default,
            stages: view
                .stages
                .into_iter()
                .filter(|stage| stage.stage.is_active)
                .map(|stage| LeadFormStageOption {
                    id: stage.stage.id.to_string(),
                    name: stage.stage.name,
                })
                .collect(),
        })
        .collect())
}

pub(crate) fn can(scope: &AccessScope, permission: &Permission) -> bool {
    scope.scope_for(permission).is_some()
}

fn length(value: &str) -> usize {
    value.chars().count()
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_field_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_LENGTH
        && value.starts_with(|c: char| c.is_ascii_lowercase())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub(crate) fn is_http_url(value: &str) -> bool {
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    rest.is_some_and(|rest| {
        !rest.is_empty()
            && !rest.starts_with('/')
            && !value
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '<' || c == '>')
    })
}

pub(crate) fn normalize_origin(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('/').to_ascii_lowercase();
    let (scheme, host) = value.split_once("://")?;
    let valid = matches!(scheme, "http" | "https")
        && !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    valid.then_some(value)
}

fn is_amount(value: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    let (integer, fraction) = match value.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (value, None),
    };
    digits(integer)
        && integer.len() <= 16
        && fraction.is_none_or(|fraction| digits(fraction) && fraction.len() <= 2)
}

fn validate_field(
    index: usize,
    field: LeadFormField,
    targets: &[LeadFormTargetField],
    keys: &mut BTreeSet<String>,
    mapped: &mut BTreeSet<String>,
    violations: &mut Vec<FieldViolation>,
) -> LeadFormField {
    let path = |name: &str| format!("fields.{index}.{name}");
    let key = field.key.trim().to_owned();
    if !is_field_key(&key) {
        violations.push(FieldViolation::new(
            path("key"),
            "must be lowercase letters, digits or underscores starting with a letter (up to 40)",
        ));
    } else if !keys.insert(key.clone()) {
        violations.push(FieldViolation::new(path("key"), "is duplicated"));
    }
    let label = field.label.trim().to_owned();
    if label.is_empty() || length(&label) > MAX_LABEL_LENGTH {
        violations.push(FieldViolation::new(
            path("label"),
            "is required and must have at most 120 characters",
        ));
    }
    let placeholder = clean(field.placeholder);
    if placeholder
        .as_deref()
        .is_some_and(|value| length(value) > MAX_PLACEHOLDER_LENGTH)
    {
        violations.push(FieldViolation::new(
            path("placeholder"),
            "must have at most 120 characters",
        ));
    }
    let mut options = Vec::new();
    if field.kind == LeadFormFieldKind::Select {
        let mut values = BTreeSet::new();
        if field.options.is_empty() || field.options.len() > MAX_OPTIONS {
            violations.push(FieldViolation::new(
                path("options"),
                "a select needs between 1 and 50 options",
            ));
        }
        for (position, option) in field.options.into_iter().enumerate() {
            let value = option.value.trim().to_owned();
            let label = option.label.trim().to_owned();
            if value.is_empty()
                || label.is_empty()
                || length(&value) > MAX_OPTION_LENGTH
                || length(&label) > MAX_OPTION_LENGTH
            {
                violations.push(FieldViolation::new(
                    format!("fields.{index}.options.{position}"),
                    "value and label are required (up to 100 characters)",
                ));
            } else if !values.insert(value.clone()) {
                violations.push(FieldViolation::new(
                    format!("fields.{index}.options.{position}.value"),
                    "is duplicated",
                ));
            }
            options.push(crate::model::LeadFormOption { value, label });
        }
    }
    let target = clean(field.target);
    if let Some(target) = &target {
        match targets.iter().find(|candidate| &candidate.key == target) {
            None => violations.push(FieldViolation::new(
                path("target"),
                "is not a writable contact field",
            )),
            Some(candidate) => {
                let checkbox_target = candidate.field_type == "checkbox";
                let checkbox_field = field.kind == LeadFormFieldKind::Checkbox;
                if checkbox_target != checkbox_field {
                    violations.push(FieldViolation::new(
                        path("target"),
                        "checkbox fields can only be mapped to checkbox contact fields",
                    ));
                }
                if !mapped.insert(target.clone()) {
                    violations.push(FieldViolation::new(
                        path("target"),
                        "this contact field is already mapped",
                    ));
                }
                if (target == "first_name" || target == FULL_NAME_TARGET) && !field.required {
                    violations.push(FieldViolation::new(
                        path("required"),
                        "the field mapped to the contact name must be required",
                    ));
                }
            }
        }
    }
    LeadFormField {
        key,
        label,
        kind: field.kind,
        required: field.required,
        placeholder,
        options,
        target,
    }
}

async fn validate_deal(
    ctx: &Ctx<'_>,
    actor: &AccessScope,
    deal: LeadFormDeal,
    violations: &mut Vec<FieldViolation>,
) -> Result<LeadFormDeal, AppError> {
    let title_template = deal.title_template.trim().to_owned();
    if title_template.is_empty() || length(&title_template) > MAX_TEMPLATE_LENGTH {
        violations.push(FieldViolation::new(
            "deal.title_template",
            "is required and must have at most 200 characters",
        ));
    }
    let amount = clean(deal.amount).map(|amount| amount.replace(',', "."));
    if amount.as_deref().is_some_and(|amount| !is_amount(amount)) {
        violations.push(FieldViolation::new(
            "deal.amount",
            "must be a decimal number with up to 2 decimal places",
        ));
    }
    let workflow = workflow(ctx, actor).await.unwrap_or_default();
    match workflow
        .iter()
        .find(|pipeline| pipeline.id == deal.pipeline_id)
    {
        None => violations.push(FieldViolation::new(
            "deal.pipeline_id",
            "is not an active pipeline visible to the acting user",
        )),
        Some(pipeline) => {
            if !pipeline
                .stages
                .iter()
                .any(|stage| stage.id == deal.stage_id)
            {
                violations.push(FieldViolation::new(
                    "deal.stage_id",
                    "is not an active stage of the pipeline",
                ));
            }
        }
    }
    Ok(LeadFormDeal {
        pipeline_id: deal.pipeline_id,
        stage_id: deal.stage_id,
        title_template,
        amount,
    })
}

pub(crate) async fn validate_request(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    request: LeadFormRequest,
    creator: Uuid,
) -> Result<ValidatedForm, AppError> {
    let mut violations = Vec::new();
    let name = request.name.trim().to_owned();
    if name.is_empty() || length(&name) > MAX_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            "is required and must have at most 120 characters",
        ));
    }
    let actor_id = request.acting_user_id.unwrap_or(creator);
    let actor = scope_for_user(ctx, actor_id).await?;
    let definition = request.definition;
    match &actor {
        None => violations.push(FieldViolation::new(
            "acting_user_id",
            "the acting user is not an active member of the organization",
        )),
        Some(actor) => {
            if !can(actor, &Permission::CONTACTS_CREATE) {
                violations.push(FieldViolation::new(
                    "acting_user_id",
                    "the acting user cannot create people",
                ));
            }
            if definition.deal.is_some() && !can(actor, &Permission::DEALS_CREATE) {
                violations.push(FieldViolation::new(
                    "acting_user_id",
                    "the acting user cannot create deals",
                ));
            }
        }
    }
    let targets = target_fields(ctx, scope).await?;
    if definition.fields.is_empty() || definition.fields.len() > MAX_FIELDS {
        violations.push(FieldViolation::new(
            "fields",
            "a form needs between 1 and 30 fields",
        ));
    }
    let mut keys = BTreeSet::new();
    let mut mapped = BTreeSet::new();
    let fields: Vec<LeadFormField> = definition
        .fields
        .into_iter()
        .enumerate()
        .map(|(index, field)| {
            validate_field(
                index,
                field,
                &targets,
                &mut keys,
                &mut mapped,
                &mut violations,
            )
        })
        .collect();
    if !mapped.contains("first_name") && !mapped.contains(FULL_NAME_TARGET) {
        violations.push(FieldViolation::new(
            "fields",
            "map a required field to the contact first name or full name",
        ));
    }
    let deal = match (definition.deal, &actor) {
        (Some(deal), Some(actor)) => Some(validate_deal(ctx, actor, deal, &mut violations).await?),
        (deal, _) => deal,
    };
    let tag_id = clean(definition.tag_id);
    if let (Some(tag_id), Some(actor)) = (&tag_id, &actor) {
        let valid = Uuid::parse_str(tag_id).is_ok()
            && tagging::service::get(&ctx.with_scope(actor), tag_id)
                .await
                .is_ok();
        if !valid {
            violations.push(FieldViolation::new("tag_id", "tag not found"));
        }
    }
    let owner_id = definition.owner_id;
    if let Some(owner_id) = owner_id
        && scope_for_user(ctx, owner_id).await?.is_none()
    {
        violations.push(FieldViolation::new(
            "owner_id",
            "the owner is not an active member of the organization",
        ));
    }
    let distribution_rule_id = clean(definition.distribution_rule_id);
    if let Some(rule_id) = &distribution_rule_id {
        let usable = ctx.app().is_enabled(assignment::KEY)
            && assignment::service::check(ctx, rule_id)
                .await?
                .is_some_and(|rule| {
                    rule.is_active && rule.record_type == RecordType::CONTACT.as_str()
                });
        if !usable {
            violations.push(FieldViolation::new(
                "distribution_rule_id",
                "must be an active lead distribution rule for people",
            ));
        }
    }
    let source = clean(definition.source);
    if source
        .as_deref()
        .is_some_and(|source| length(source) > MAX_SOURCE_LENGTH)
    {
        violations.push(FieldViolation::new(
            "source",
            "must have at most 120 characters",
        ));
    }
    let success_message = clean(definition.success_message);
    if success_message
        .as_deref()
        .is_some_and(|message| length(message) > MAX_MESSAGE_LENGTH)
    {
        violations.push(FieldViolation::new(
            "success_message",
            "must have at most 500 characters",
        ));
    }
    let submit_label = clean(definition.submit_label);
    if submit_label
        .as_deref()
        .is_some_and(|label| length(label) > MAX_SUBMIT_LABEL_LENGTH)
    {
        violations.push(FieldViolation::new(
            "submit_label",
            "must have at most 60 characters",
        ));
    }
    let redirect_url = clean(definition.redirect_url);
    if redirect_url
        .as_deref()
        .is_some_and(|url| length(url) > MAX_URL_LENGTH || !is_http_url(url))
    {
        violations.push(FieldViolation::new(
            "redirect_url",
            "must be an http(s) URL with at most 500 characters",
        ));
    }
    if definition.allowed_origins.len() > MAX_ORIGINS {
        violations.push(FieldViolation::new("allowed_origins", "at most 20 origins"));
    }
    let mut allowed_origins = Vec::new();
    for (index, origin) in definition.allowed_origins.iter().enumerate() {
        match normalize_origin(origin) {
            Some(origin) if !allowed_origins.contains(&origin) => allowed_origins.push(origin),
            Some(_) => {}
            None => violations.push(FieldViolation::new(
                format!("allowed_origins.{index}"),
                "must look like https://www.example.com (scheme and host, no path)",
            )),
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            INVALID_FORM,
            "invalid lead form",
            violations,
        ));
    }
    Ok(ValidatedForm {
        name,
        is_active: request.is_active,
        acting_user_id: request.acting_user_id,
        definition: LeadFormDefinition {
            fields,
            deal,
            tag_id,
            owner_id,
            distribution_rule_id,
            source,
            success_message,
            redirect_url,
            allowed_origins,
            dedupe_by_email: definition.dedupe_by_email,
            submit_label,
        },
    })
}
