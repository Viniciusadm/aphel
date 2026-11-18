use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::records::{self, actor_id, actor_kind, is_tolerated, is_validation};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::Permission;

use crate::errors::{self, TEMPLATE_INACTIVE, TEMPLATE_REQUIRED};
use crate::format;
use crate::markup::{self, Issue, MAX_BODY_CHARS};
use crate::pdf;
use crate::render::{self, RenderedBlock};
use crate::store::{self, DocumentRow, TemplateRow, TemplateValues};
use crate::variables::{self, Catalog, Items, RenderData};

pub(crate) const MANAGE_PERMISSION: &str = "proposals.manage";
pub(crate) const GENERATE_PERMISSION: &str = "proposals.generate";
pub(crate) const MANAGE: Permission = Permission::from_static(MANAGE_PERMISSION);
pub(crate) const GENERATE: Permission = Permission::from_static(GENERATE_PERMISSION);
pub(crate) const PROPOSAL_GENERATED: &str = "proposal.generated";
pub(crate) const TEMPLATE_CREATED: &str = "proposal_template.created";
pub(crate) const TEMPLATE_UPDATED: &str = "proposal_template.updated";
pub(crate) const TEMPLATE_DELETED: &str = "proposal_template.deleted";
const MAX_NAME_CHARS: usize = 160;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_KEY_CHARS: usize = 64;
const AUDIT_BODY_CHARS: usize = 2000;

pub(crate) fn is_template_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_CHARS
        && key.starts_with(|character: char| character.is_ascii_lowercase())
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

pub(crate) fn key_from_name(name: &str) -> String {
    let slug = format::slug(name).replace('-', "_");
    let key = if slug.starts_with(|character: char| character.is_ascii_lowercase()) {
        slug
    } else {
        format!("template_{slug}")
    };
    key.trim_end_matches('_')
        .chars()
        .take(MAX_KEY_CHARS - 4)
        .collect()
}

pub(crate) fn clean_description(description: Option<String>) -> Option<String> {
    description
        .map(|description| description.trim().to_owned())
        .filter(|description| !description.is_empty())
}

pub(crate) fn body_violations(issues: &[Issue]) -> Vec<FieldViolation> {
    issues
        .iter()
        .map(|issue| FieldViolation::new("body", issue.describe()))
        .collect()
}

pub(crate) fn value_violations(values: &TemplateValues) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    if !is_template_key(&values.key) {
        violations.push(FieldViolation::new(
            "key",
            "must start with a lowercase letter and contain only lowercase letters, digits and underscores (up to 64)",
        ));
    }
    let name = values.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        violations.push(FieldViolation::new(
            "name",
            "is required and must have up to 160 characters",
        ));
    }
    if values
        .description
        .as_ref()
        .is_some_and(|description| description.chars().count() > MAX_DESCRIPTION_CHARS)
    {
        violations.push(FieldViolation::new(
            "description",
            "must have up to 500 characters",
        ));
    }
    if values.body.trim().is_empty() || values.body.chars().count() > MAX_BODY_CHARS {
        violations.push(FieldViolation::new(
            "body",
            "is required and must have up to 100000 characters",
        ));
    }
    if values.is_default && !values.is_active {
        violations.push(FieldViolation::new(
            "is_default",
            "an inactive template cannot be the default",
        ));
    }
    violations
}

pub(crate) fn check(values: &TemplateValues, catalog: &Catalog) -> Result<(), AppError> {
    let mut violations = value_violations(values);
    if violations.iter().all(|violation| violation.field != "body") {
        let issues = catalog.validate(&markup::parse(&values.body));
        violations.extend(body_violations(&issues));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(errors::template_invalid(violations))
    }
}

fn truncated(body: &str) -> Value {
    if body.chars().count() > AUDIT_BODY_CHARS {
        json!(format!(
            "{}…",
            body.chars().take(AUDIT_BODY_CHARS).collect::<String>()
        ))
    } else {
        json!(body)
    }
}

pub(crate) fn template_changes(
    old: Option<&TemplateValues>,
    new: Option<&TemplateValues>,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let pick = |values: Option<&TemplateValues>, field: &str| -> Value {
        values.map_or(Value::Null, |values| match field {
            "key" => json!(values.key),
            "name" => json!(values.name),
            "description" => json!(values.description),
            "body" => truncated(&values.body),
            "is_active" => json!(values.is_active),
            _ => json!(values.is_default),
        })
    };
    for field in [
        "key",
        "name",
        "description",
        "body",
        "is_active",
        "is_default",
    ] {
        let before = pick(old, field);
        let after = pick(new, field);
        if before != after {
            changes.push(FieldChange::new(field, before, after));
        }
    }
    changes
}

async fn record_template_event(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    event_type: &str,
    payload: Value,
    mut changes: Vec<FieldChange>,
    id_change: FieldChange,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let now = Utc::now();
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now).with_payload(payload),
        )
        .await?;
    changes.push(id_change);
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now).with_changes(changes),
        )
        .await
}

fn template_payload(id: Uuid, values: &TemplateValues) -> Value {
    json!({
        "template_id": id,
        "key": values.key,
        "name": values.name,
        "is_active": values.is_active,
        "is_default": values.is_default,
    })
}

pub(crate) async fn unique_key(ctx: &Ctx<'_>, base: &str) -> Result<String, AppError> {
    let pool = ctx.pool();
    let base = if is_template_key(base) {
        base.to_owned()
    } else {
        "template".to_owned()
    };
    let mut candidate = base.clone();
    let mut suffix = 2;
    while store::find_template_by_key(pool, &candidate)
        .await?
        .is_some()
    {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

pub(crate) async fn insert_template(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    values: &TemplateValues,
) -> Result<Uuid, AppError> {
    let id = store::insert_template(tx, actor_id(ctx.scope), values).await?;
    record_template_event(
        ctx,
        tx,
        TEMPLATE_CREATED,
        template_payload(id, values),
        template_changes(None, Some(values)),
        FieldChange::new("template_id", Value::Null, json!(id)),
    )
    .await?;
    Ok(id)
}

pub(crate) async fn write_template_update(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    current: &TemplateRow,
    values: &TemplateValues,
) -> Result<(), AppError> {
    store::update_template(tx, actor_id(ctx.scope), current.id, values).await?;
    record_template_event(
        ctx,
        tx,
        TEMPLATE_UPDATED,
        template_payload(current.id, values),
        template_changes(Some(&TemplateValues::from(current)), Some(values)),
        FieldChange::new("template_id", json!(current.id), json!(current.id)),
    )
    .await
}

pub(crate) async fn create_template(
    ctx: &Ctx<'_>,
    values: TemplateValues,
) -> Result<TemplateRow, AppError> {
    ctx.scope.require(&MANAGE)?;
    let catalog = variables::catalog(ctx).await?;
    check(&values, &catalog)?;
    let mut tx = ctx.begin().await?;
    let id = insert_template(ctx, &mut tx, &values).await?;
    tx.commit().await?;
    load_template(ctx, id).await
}

pub(crate) async fn load_template(ctx: &Ctx<'_>, id: Uuid) -> Result<TemplateRow, AppError> {
    store::find_template(ctx.pool(), id)
        .await?
        .ok_or_else(errors::template_not_found)
}

pub(crate) struct TemplatePatch {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) body: Option<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_default: Option<bool>,
}

pub(crate) async fn update_template(
    ctx: &Ctx<'_>,
    id: Uuid,
    patch: TemplatePatch,
) -> Result<TemplateRow, AppError> {
    ctx.scope.require(&MANAGE)?;
    let current = load_template(ctx, id).await?;
    let mut values = TemplateValues::from(&current);
    if let Some(name) = patch.name {
        values.name = name.trim().to_owned();
    }
    if let Some(description) = patch.description {
        values.description = clean_description(description);
    }
    if let Some(body) = patch.body {
        values.body = body;
    }
    if let Some(is_active) = patch.is_active {
        values.is_active = is_active;
        if !is_active && patch.is_default.is_none() {
            values.is_default = false;
        }
    }
    if let Some(is_default) = patch.is_default {
        values.is_default = is_default;
    }
    if values == TemplateValues::from(&current) {
        return Ok(current);
    }
    let catalog = variables::catalog(ctx).await?;
    check(&values, &catalog)?;
    let mut tx = ctx.begin().await?;
    write_template_update(ctx, &mut tx, &current, &values).await?;
    tx.commit().await?;
    load_template(ctx, id).await
}

pub(crate) async fn delete_template(ctx: &Ctx<'_>, id: Uuid) -> Result<(), AppError> {
    ctx.scope.require(&MANAGE)?;
    let current = load_template(ctx, id).await?;
    let values = TemplateValues::from(&current);
    let mut tx = ctx.begin().await?;
    store::delete_template(&mut tx, id).await?;
    record_template_event(
        ctx,
        &mut tx,
        TEMPLATE_DELETED,
        template_payload(id, &values),
        template_changes(Some(&values), None),
        FieldChange::new("template_id", json!(id), Value::Null),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn related_record(
    ctx: &Ctx<'_>,
    record_type: &str,
    id: Option<&str>,
) -> Result<Value, AppError> {
    let Some(id) = id.and_then(|id| Uuid::parse_str(id).ok()) else {
        return Ok(Value::Null);
    };
    match records::load(ctx, record_type, id).await {
        Ok(record) => Ok(record.unwrap_or(Value::Null)),
        Err(error) if is_tolerated(&error) => Ok(Value::Null),
        Err(error) => Err(error),
    }
}

pub(crate) async fn load_deal(ctx: &Ctx<'_>, deal_id: Uuid) -> Result<Value, AppError> {
    match records::load(ctx, aphel_contract_crm::keys::DEAL, deal_id).await {
        Ok(Some(deal)) => Ok(deal),
        Ok(None) => Err(errors::deal_not_found()),
        Err(error) if is_tolerated(&error) => Err(errors::deal_not_found()),
        Err(error) => Err(error),
    }
}

pub(crate) async fn load_quote(
    ctx: &Ctx<'_>,
    quote_id: Uuid,
    deal_id: Option<Uuid>,
) -> Result<(Uuid, Value), AppError> {
    let quote = match records::quote(ctx, quote_id).await {
        Ok(quote) => quote,
        Err(error) if is_tolerated(&error) => return Err(errors::quote_not_found()),
        Err(error) => return Err(error),
    };
    let quote_deal = quote
        .get("deal_id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(errors::quote_not_found)?;
    if deal_id.is_some_and(|deal_id| deal_id != quote_deal) {
        return Err(errors::quote_not_found());
    }
    Ok((quote_deal, quote))
}

pub(crate) async fn render_data(
    ctx: &Ctx<'_>,
    deal_id: Option<Uuid>,
    quote: Option<Value>,
    template_name: &str,
    number: Option<u32>,
) -> Result<RenderData, AppError> {
    let organization = records::organization(ctx).await?;
    let (user_name, user_email) = records::user_identity(ctx).await;
    let mut data = RenderData {
        deal: Value::Null,
        contact: Value::Null,
        company: Value::Null,
        organization_name: organization.name,
        timezone: organization.timezone,
        locale: organization.locale,
        currency: organization.currency,
        user_name,
        user_email,
        items: None,
        quote: Value::Null,
        number,
        template_name: template_name.to_owned(),
        today: format::today(organization.timezone),
        sample: deal_id.is_none(),
    };
    let Some(deal_id) = deal_id else {
        return Ok(data);
    };
    let deal = load_deal(ctx, deal_id).await?;
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    data.contact = related_record(
        ctx,
        aphel_contract_crm::keys::CONTACT,
        text(&deal, "contact_id").as_deref(),
    )
    .await?;
    let company_id = text(&deal, "company_id").or_else(|| text(&data.contact, "company_id"));
    data.company = related_record(
        ctx,
        aphel_contract_crm::keys::COMPANY,
        company_id.as_deref(),
    )
    .await?;
    data.items = match &quote {
        Some(quote) => Some(Items::from_quote(quote)),
        None => match records::line_items(ctx, deal_id).await {
            Ok(items) => Some(Items::from_lines(&items)),
            Err(error) if is_tolerated(&error) || is_validation(&error) => None,
            Err(error) => return Err(error),
        },
    };
    data.quote = quote.unwrap_or(Value::Null);
    data.deal = deal;
    Ok(data)
}

pub(crate) struct Preview {
    pub(crate) blocks: Vec<RenderedBlock>,
    pub(crate) issues: Vec<Issue>,
}

pub(crate) async fn preview(
    ctx: &Ctx<'_>,
    body: &str,
    template_name: &str,
    deal_id: Option<Uuid>,
    quote_id: Option<Uuid>,
) -> Result<Preview, AppError> {
    let catalog = variables::catalog(ctx).await?;
    let template = markup::parse(body);
    let issues = catalog.validate(&template);
    let (deal_id, quote) = match quote_id {
        Some(quote_id) => {
            let (deal_id, quote) = load_quote(ctx, quote_id, deal_id).await?;
            (Some(deal_id), Some(quote))
        }
        None => (deal_id, None),
    };
    let data = render_data(ctx, deal_id, quote, template_name, None).await?;
    Ok(Preview {
        blocks: render::render(&template, &data, &catalog),
        issues,
    })
}

pub(crate) enum TemplateChoice {
    Id(Uuid),
    Key(String),
    Default,
}

pub(crate) async fn choose_template(
    ctx: &Ctx<'_>,
    choice: &TemplateChoice,
) -> Result<TemplateRow, AppError> {
    let pool = ctx.pool();
    let template = match choice {
        TemplateChoice::Id(id) => store::find_template(pool, *id)
            .await?
            .ok_or_else(errors::template_not_found)?,
        TemplateChoice::Key(key) => store::find_template_by_key(pool, key)
            .await?
            .ok_or_else(errors::template_not_found)?,
        TemplateChoice::Default => store::default_template(pool).await?.ok_or_else(|| {
            AppError::validation_code(
                TEMPLATE_REQUIRED,
                "choose a template: there is no active template",
            )
        })?,
    };
    if !template.is_active {
        return Err(AppError::validation_code(
            TEMPLATE_INACTIVE,
            "the proposal template is inactive",
        ));
    }
    Ok(template)
}

fn file_name(number: u32, deal: &Value) -> String {
    let title = deal
        .get("title")
        .and_then(Value::as_str)
        .map(format::slug)
        .unwrap_or_default();
    let number = format::proposal_number(number);
    if title.is_empty() {
        format!("proposta-{number}.pdf")
    } else {
        format!("proposta-{number}-{title}.pdf")
    }
}

pub(crate) async fn generate(
    ctx: &Ctx<'_>,
    deal_id: Uuid,
    choice: TemplateChoice,
    quote_id: Option<Uuid>,
) -> Result<DocumentRow, AppError> {
    ctx.scope.require(&GENERATE)?;
    ctx.scope.require(&Permission::DEALS_UPDATE)?;
    let template = choose_template(ctx, &choice).await?;
    load_deal(ctx, deal_id).await?;
    let quote = match quote_id {
        Some(quote_id) => Some(load_quote(ctx, quote_id, Some(deal_id)).await?.1),
        None => None,
    };
    let catalog = variables::catalog(ctx).await?;
    let mut tx = ctx.begin().await?;
    let number = store::next_number(&mut tx).await?;
    let data = render_data(ctx, Some(deal_id), quote, &template.name, Some(number)).await?;
    let blocks = render::render(&markup::parse(&template.body), &data, &catalog);
    let content = pdf::document(
        pdf::Header {
            organization: data.organization_name.clone(),
            title: format!(
                "{} {}",
                data.locale.labels().proposal_title,
                format::proposal_number(number)
            ),
            subtitle: format::date(data.today, data.locale),
            locale: data.locale,
        },
        &blocks,
    )?;
    let attach_to = RecordRef::new(RecordType::DEAL, deal_id);
    let files = ctx.files().await?;
    let file = files
        .prepare_upload(
            ctx.scope,
            &aphel_sdk::platform::application::files::UploadFileCommand {
                file_name: file_name(number, &data.deal),
                content: bytes::Bytes::from(content),
                attach_to: Some(attach_to.clone()),
                ip: None,
            },
        )
        .await?;
    let document = DocumentRow {
        id: Uuid::now_v7(),
        deal_id,
        template_id: Some(template.id),
        template_name: template.name.clone(),
        number,
        file_id: file.id,
        file_name: file.original_name.clone(),
        file_size: file.size,
        generated_by: Some(actor_id(ctx.scope)),
        generated_by_name: data.user_name.clone(),
        created_at: Utc::now(),
    };
    let saved = async {
        files
            .record_upload_in_tx(&mut tx, ctx.scope, &file, Some(attach_to), None)
            .await?;
        record_document(ctx, &mut tx, &document, &template, quote_id).await
    }
    .await;
    if let Err(error) = saved {
        tx.rollback().await?;
        if let Err(cleanup) = ctx.storage()?.delete(&file.storage_key).await {
            tracing::warn!(%cleanup, file_id = %file.id, "failed to remove unused proposal file");
        }
        return Err(error);
    }
    tx.commit().await?;
    Ok(document)
}

async fn record_document(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    document: &DocumentRow,
    template: &TemplateRow,
    quote_id: Option<Uuid>,
) -> Result<(), AppError> {
    let deal_id = document.deal_id;
    let number = document.number;
    let deal_ref = RecordRef::new(RecordType::DEAL, deal_id);
    let actor = Actor::from_scope(ctx.scope);
    let now = Utc::now();
    store::insert_document(tx, document, &actor_kind(ctx.scope)).await?;
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), PROPOSAL_GENERATED, now)
                .with_record(deal_ref.clone())
                .with_payload(json!({
                    "document_id": document.id,
                    "deal_id": deal_id,
                    "number": number,
                    "template_id": template.id,
                    "template_key": template.key,
                    "template_name": template.name,
                    "file_id": document.file_id,
                    "file_name": document.file_name,
                    "quote_id": quote_id,
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, PROPOSAL_GENERATED, now)
                .with_record(deal_ref)
                .with_changes(vec![
                    FieldChange::new("proposal_number", Value::Null, json!(number)),
                    FieldChange::new("template", Value::Null, json!(template.name)),
                    FieldChange::new("file_id", Value::Null, json!(document.file_id)),
                ]),
        )
        .await?;
    Ok(())
}

pub(crate) async fn seed_starter_template(ctx: &Ctx<'_>) -> Result<(), AppError> {
    if store::count_templates(ctx.pool()).await? > 0 {
        return Ok(());
    }
    let values = TemplateValues {
        key: "proposta_comercial".to_owned(),
        name: "Proposta comercial".to_owned(),
        description: Some("Modelo inicial criado ao habilitar a extensão".to_owned()),
        body: STARTER_BODY.to_owned(),
        is_active: true,
        is_default: true,
    };
    let mut tx = ctx.begin().await?;
    insert_template(ctx, &mut tx, &values).await?;
    tx.commit().await?;
    Ok(())
}

const STARTER_BODY: &str = "# Proposta comercial\n\n{{organization.name}}, {{today}}\n\nÀ {{company.name}}\nA/C {{contact.name}}\n\n## Objeto\n\nApresentamos a proposta **{{deal.title}}**, preparada por {{owner.name}}.\n\n## Itens\n\n{{items_table}}\n\n## Condições\n\n- Valor total: **{{items.total}}**\n- Validade da proposta: 15 dias a partir de {{today}}\n\n---\n\nAtenciosamente,\n{{user.name}}\n{{organization.name}}\n";
