use aphel_contract_crm::CrmPermissions;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::compose::html::sanitize_html;
use crate::service::records;
use crate::store::sending::{OutboundStore, TemplateStore};
use crate::templates::{
    CreateTemplateCommand, EMAIL_TEMPLATE_CREATED, EMAIL_TEMPLATE_DELETED,
    EMAIL_TEMPLATE_KEY_TAKEN, EMAIL_TEMPLATE_RECORD, EMAIL_TEMPLATE_UPDATED, EmailTemplateView,
    MAX_TEMPLATE_BODY_BYTES, MAX_TEMPLATE_NAME_LENGTH, MAX_TEMPLATE_SUBJECT_LENGTH,
    NewEmailTemplate, PreviewCommand, RECORD_PREFIX, RenderedEmail, TemplateFilter,
    TemplateListScope, TemplateScope, TemplateVariable, USER_PREFIX, UpdateTemplateCommand,
    VariableField, VariableValues, is_email_record_type, related_record_types, related_records,
    render_email, template_not_found,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::configuration::{is_config_key, key_from_name};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_changes};
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

const MAX_KEY_ATTEMPTS: usize = 50;

#[derive(Debug, Serialize)]
struct TemplateSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    scope: &'static str,
    record_type: Option<&'a str>,
    subject: &'a str,
    body_html: &'a str,
}

impl<'a> From<&'a EmailTemplateView> for TemplateSnapshot<'a> {
    fn from(template: &'a EmailTemplateView) -> Self {
        Self {
            key: &template.key,
            name: &template.name,
            scope: template.scope.as_str(),
            record_type: template.record_type.as_deref(),
            subject: &template.subject,
            body_html: &template.body_html,
        }
    }
}

fn normalize_name(value: &str, violations: &mut Vec<FieldViolation>) -> String {
    let name = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        violations.push(FieldViolation::new("name", "must not be empty"));
    } else if name.chars().count() > MAX_TEMPLATE_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have at most {MAX_TEMPLATE_NAME_LENGTH} characters"),
        ));
    }
    name
}

fn normalize_subject(value: &str, violations: &mut Vec<FieldViolation>) -> String {
    let subject = value.replace(['\r', '\n'], " ").trim().to_owned();
    if subject.chars().count() > MAX_TEMPLATE_SUBJECT_LENGTH {
        violations.push(FieldViolation::new(
            "subject",
            format!("must have at most {MAX_TEMPLATE_SUBJECT_LENGTH} characters"),
        ));
    }
    subject
}

fn normalize_body(value: &str, violations: &mut Vec<FieldViolation>) -> String {
    if value.len() > MAX_TEMPLATE_BODY_BYTES {
        violations.push(FieldViolation::new(
            "body_html",
            format!("must have at most {MAX_TEMPLATE_BODY_BYTES} bytes"),
        ));
    }
    sanitize_html(value)
}

fn normalize_record_type(
    value: Option<&str>,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    if !is_email_record_type(value) {
        violations.push(FieldViolation::new(
            "record_type",
            "must be contact, company, deal or a custom object",
        ));
        return None;
    }
    Some(value.to_owned())
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    list_scope: Option<&str>,
    record_type: Option<&str>,
) -> Result<Vec<EmailTemplateView>, AppError> {
    let scope = ctx.scope;
    require_template_reader(scope)?;
    let list_scope = match list_scope.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => TemplateListScope::parse(value).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid template filter",
                vec![FieldViolation::new(
                    "scope",
                    "must be all, organization or personal",
                )],
            )
        })?,
        None => TemplateListScope::All,
    };
    let record_type = record_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    TemplateStore::of(ctx)
        .list(&TemplateFilter {
            scope: list_scope,
            viewer: scope.user_id().clone(),
            record_type,
        })
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<EmailTemplateView, AppError> {
    require_template_reader(ctx.scope)?;
    find_visible_template(ctx, id).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: CreateTemplateCommand,
) -> Result<EmailTemplateView, AppError> {
    let scope = ctx.scope;
    let template_scope = match command.scope.as_deref().map(str::trim) {
        None | Some("") => TemplateScope::Personal,
        Some(value) => TemplateScope::parse(value).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid e-mail template",
                vec![FieldViolation::new(
                    "scope",
                    "must be organization or personal",
                )],
            )
        })?,
    };
    require_template_writer(scope, template_scope)?;
    let mut violations = Vec::new();
    let name = normalize_name(&command.name, &mut violations);
    let subject = normalize_subject(&command.subject, &mut violations);
    let body_html = normalize_body(&command.body_html, &mut violations);
    let record_type = normalize_record_type(command.record_type.as_deref(), &mut violations);
    let explicit_key = command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    if let Some(key) = explicit_key
        && !is_config_key(key)
    {
        violations.push(FieldViolation::new(
            "key",
            "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid e-mail template",
            violations,
        ));
    }
    let owner = match template_scope {
        TemplateScope::Organization => None,
        TemplateScope::Personal => Some(scope.user_id().clone()),
    };
    let templates = TemplateStore::of(ctx);
    let key = match explicit_key {
        Some(key) => {
            if templates.key_taken(owner.as_ref(), key, None).await? {
                return Err(key_taken(key));
            }
            key.to_owned()
        }
        None => free_key(ctx, owner.as_ref(), &name).await?,
    };
    let template = NewEmailTemplate {
        id: Uuid::now_v7(),
        key,
        name,
        scope: template_scope,
        owner_id: owner,
        record_type,
        subject,
        body_html,
        created_by: Some(scope.user_id().clone()),
    };
    let now = ctx.now();
    let view = EmailTemplateView {
        id: template.id,
        key: template.key.clone(),
        name: template.name.clone(),
        scope: template.scope,
        owner_id: template.owner_id.clone(),
        record_type: template.record_type.clone(),
        subject: template.subject.clone(),
        body_html: template.body_html.clone(),
        created_at: now,
        updated_at: now,
    };
    let changes = diff_changes(
        &Value::Null,
        &TemplateSnapshot::from(&view),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    templates.insert(&mut tx, &template, now).await?;
    publish(
        ctx,
        &mut tx,
        &view,
        EMAIL_TEMPLATE_CREATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    require(ctx, template.id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateTemplateCommand,
) -> Result<EmailTemplateView, AppError> {
    let scope = ctx.scope;
    let current = find_visible_template(ctx, id).await?;
    require_template_writer(scope, current.scope)?;
    let mut violations = Vec::new();
    let mut updated = current.clone();
    if let Some(name) = &command.name {
        updated.name = normalize_name(name, &mut violations);
    }
    if let Some(subject) = &command.subject {
        updated.subject = normalize_subject(subject, &mut violations);
    }
    if let Some(body) = &command.body_html {
        updated.body_html = normalize_body(body, &mut violations);
    }
    if let Some(record_type) = &command.record_type {
        updated.record_type = normalize_record_type(record_type.as_deref(), &mut violations);
    }
    if let Some(key) = command.key.as_deref().map(str::trim) {
        if is_config_key(key) {
            updated.key = key.to_owned();
        } else {
            violations.push(FieldViolation::new(
                "key",
                "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
            ));
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid e-mail template",
            violations,
        ));
    }
    let templates = TemplateStore::of(ctx);
    if updated.key != current.key
        && templates
            .key_taken(current.owner_id.as_ref(), &updated.key, Some(current.id))
            .await?
    {
        return Err(key_taken(&updated.key));
    }
    let changes = diff_changes(
        &TemplateSnapshot::from(&current),
        &TemplateSnapshot::from(&updated),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    templates
        .update(&mut tx, &updated, scope.user_id(), now)
        .await?;
    publish(
        ctx,
        &mut tx,
        &updated,
        EMAIL_TEMPLATE_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    tx.commit().await?;
    require(ctx, current.id).await
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let current = find_visible_template(ctx, id).await?;
    require_template_writer(ctx.scope, current.scope)?;
    let changes = diff_changes(
        &TemplateSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    if !TemplateStore::of(ctx).delete(&mut tx, current.id).await? {
        return Err(template_not_found());
    }
    publish(ctx, &mut tx, &current, EMAIL_TEMPLATE_DELETED, changes, ip).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn variables(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
) -> Result<Vec<TemplateVariable>, AppError> {
    require_template_reader(ctx.scope)?;
    let record_types = match record_type.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if is_email_record_type(value) => related_record_types(value),
        Some(_) => {
            return Err(AppError::invalid_fields(
                "invalid record type",
                vec![FieldViolation::new(
                    "record_type",
                    "must be contact, company, deal or a custom object",
                )],
            ));
        }
        None => vec![
            aphel_contract_crm::keys::CONTACT.to_owned(),
            aphel_contract_crm::keys::COMPANY.to_owned(),
            aphel_contract_crm::keys::DEAL.to_owned(),
        ],
    };
    let mut variables = Vec::new();
    for record_type in record_types {
        let fields = match records::readable_fields(ctx, &record_type).await {
            Ok(fields) => fields,
            Err(error) => {
                tracing::debug!(%record_type, %error, "skipping e-mail template variables");
                continue;
            }
        };
        variables.extend(fields.into_iter().map(|field| TemplateVariable {
            key: format!("{record_type}.{}", field.key),
            label: field.label,
            record_type: record_type.clone(),
        }));
    }
    variables.push(TemplateVariable {
        key: format!("{USER_PREFIX}.display_name"),
        label: "Nome do remetente".to_owned(),
        record_type: USER_PREFIX.to_owned(),
    });
    variables.push(TemplateVariable {
        key: format!("{USER_PREFIX}.email"),
        label: "E-mail do remetente".to_owned(),
        record_type: USER_PREFIX.to_owned(),
    });
    Ok(variables)
}

pub async fn preview(ctx: &Ctx<'_>, command: PreviewCommand) -> Result<RenderedEmail, AppError> {
    let scope = ctx.scope;
    require_template_reader(scope)?;
    let template = match command.template_id.as_deref() {
        Some(id) => Some(find_visible_template(ctx, id).await?),
        None => None,
    };
    let subject = command
        .subject
        .or_else(|| template.as_ref().map(|template| template.subject.clone()))
        .unwrap_or_default();
    let body = command
        .body_html
        .or_else(|| template.as_ref().map(|template| template.body_html.clone()))
        .unwrap_or_default();
    let record = match (command.record_type.as_deref(), command.record_id.as_deref()) {
        (Some(record_type), Some(record_id)) => {
            let record = RecordRef::parse(record_type, record_id)?;
            records::require_readable(ctx, &record).await?;
            Some(record)
        }
        (None, None) => None,
        _ => {
            return Err(AppError::Validation(
                "record must have record_type and record_id".to_owned(),
            ));
        }
    };
    let values = load_variable_values(ctx, scope.user_id(), record.as_ref()).await?;
    Ok(render_email(&subject, &body, &values))
}

pub(crate) async fn load_variable_values(
    ctx: &Ctx<'_>,
    sender: &UserId,
    record: Option<&RecordRef>,
) -> Result<VariableValues, AppError> {
    let mut values = VariableValues::new();
    if let Some(profile) = OutboundStore::of(ctx).sender_profile(sender).await? {
        values.insert(format!("{USER_PREFIX}.display_name"), profile.display_name);
        values.insert(format!("{USER_PREFIX}.email"), profile.email);
    }
    let Some(record) = record else {
        return Ok(values);
    };
    let Some(data) = records::load_record(ctx, record).await? else {
        return Ok(values);
    };
    let record_type = record.record_type.as_str();
    let fields = readable_fields(ctx, record_type).await;
    values.extend_record(record_type, &data, &fields);
    values.extend_record(RECORD_PREFIX, &data, &fields);
    for related in related_records(record_type) {
        let Some(id) = data.get(related.column.as_str()).and_then(Value::as_str) else {
            continue;
        };
        let Ok(related_record) = RecordRef::parse(related.record_type.as_str(), id) else {
            continue;
        };
        if let Some(related_data) = records::load_record(ctx, &related_record).await? {
            let fields = readable_fields(ctx, related.record_type.as_str()).await;
            values.extend_record(related.record_type.as_str(), &related_data, &fields);
        }
    }
    Ok(values)
}

async fn readable_fields(ctx: &Ctx<'_>, record_type: &str) -> Vec<VariableField> {
    records::readable_fields(ctx, record_type)
        .await
        .unwrap_or_default()
}

pub(crate) async fn find_visible_template(
    ctx: &Ctx<'_>,
    id: &str,
) -> Result<EmailTemplateView, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| template_not_found())?;
    TemplateStore::of(ctx)
        .find(id)
        .await?
        .filter(|template| template.is_visible_to(ctx.scope.user_id()))
        .ok_or_else(template_not_found)
}

pub(crate) fn require_template_reader(scope: &AccessScope) -> Result<(), AppError> {
    if scope.require(&Permission::SETTINGS_MANAGE).is_ok() {
        return Ok(());
    }
    scope.require(&Permission::EMAILS_SEND).map(|_| ())
}

fn require_template_writer(
    scope: &AccessScope,
    template_scope: TemplateScope,
) -> Result<(), AppError> {
    match template_scope {
        TemplateScope::Organization => scope.require(&Permission::SETTINGS_MANAGE).map(|_| ()),
        TemplateScope::Personal => scope.require(&Permission::EMAILS_SEND).map(|_| ()),
    }
}

fn key_taken(key: &str) -> AppError {
    AppError::conflict_code(
        EMAIL_TEMPLATE_KEY_TAKEN,
        format!("there is already an e-mail template with the key {key}"),
    )
}

async fn free_key(ctx: &Ctx<'_>, owner: Option<&UserId>, name: &str) -> Result<String, AppError> {
    let templates = TemplateStore::of(ctx);
    let base: String = key_from_name(name).chars().take(56).collect();
    for attempt in 0..MAX_KEY_ATTEMPTS {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            format!("{base}_{}", attempt + 1)
        };
        if !templates.key_taken(owner, &candidate, None).await? {
            return Ok(candidate);
        }
    }
    Ok(format!(
        "{base}_{}",
        &Uuid::now_v7().simple().to_string()[..6]
    ))
}

async fn require(ctx: &Ctx<'_>, id: Uuid) -> Result<EmailTemplateView, AppError> {
    TemplateStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(template_not_found)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    template: &EmailTemplateView,
    action: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let now = ctx.now();
    let record = RecordRef::new(EMAIL_TEMPLATE_RECORD, template.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), action, now).with_payload(json!({
                "template_id": template.id,
                "key": template.key,
                "scope": template.scope.as_str(),
                "owner_id": template.owner_id.as_ref().map(UserId::as_str),
            })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, action, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}
