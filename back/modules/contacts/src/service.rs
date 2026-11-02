use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{
    CONTACT_ARCHIVED, CONTACT_COMPANY_LINKED, CONTACT_COMPANY_UNLINKED, CONTACT_CREATED,
    CONTACT_DELETED, CONTACT_UNARCHIVED, CONTACT_UPDATED, CompanyContactLink, Contact,
    ContactCompanyLink, ContactData, ContactDetails, ContactInput, ContactView, INVALID_CONTACT,
    LinkCompanyCommand, NewContactCompanyLink, SaveContactCommand, already_linked,
    apply_contact_input, contact_not_found, contact_snapshot, link_not_found,
};
use crate::store::ContactStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, RecordDraft, RecordPipeline,
    RecordTypeDefinition, RecordWrite, ReferenceChecker, WriteMode,
};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{RecordQuery, RecordQueryEngine};
use aphel_kernel::application::shared::{FieldChange, RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::domain::fields::FieldValueError;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{
    InputValidator, PendingCommit, RecordMutation, archive_change, changes_payload, check_owner,
    contributed_input, is_writable_contributed, merge_contributed, merge_violations,
    normalize_search_term, parse_record_id, permission_denied, publish_mutation, with_contributed,
    write_contributed,
};
use companies::domain::company_not_found;

pub fn contact_values(data: &ContactData, contributed: &Map<String, Value>) -> Value {
    with_contributed(contact_snapshot(data), contributed)
}

pub(crate) fn view_values(definition: &RecordTypeDefinition, view: &ContactView) -> Value {
    contact_values(
        &view.contact.data,
        &contributed_input(definition, &view.contributed),
    )
}

pub async fn list(
    ctx: &Ctx<'_>,
    mut query: RecordQuery,
) -> Result<Listing<ContactDetails>, AppError> {
    query.search = query.search.as_deref().map(normalize_search_term);
    let page = RecordQueryEngine::new(&ctx.query())
        .query(ctx.scope, RecordType::CONTACT.as_str(), query)
        .await?;
    let views = ContactStore::of(ctx).find_many(&page.items).await?;
    let mut ordered = Vec::with_capacity(page.items.len());
    for id in &page.items {
        if let Some(view) = views.iter().find(|view| view.contact.id == *id) {
            ordered.push(view.clone());
        }
    }
    Ok(Listing {
        items: with_custom_fields(ctx, ordered).await?,
        total: page.total,
    })
}

pub async fn get(ctx: &Ctx<'_>, contact_id: &str) -> Result<ContactDetails, AppError> {
    let view = visible(ctx, contact_id).await?;
    details(ctx, view).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: SaveContactCommand,
) -> Result<ContactDetails, AppError> {
    let mut tx = ctx.begin().await?;
    let created = create_in_tx(ctx, &mut tx, command).await?;
    tx.commit().await?;
    created.committed(ctx).await;
    reload(ctx, created.id()).await
}

pub struct CreatedContact {
    record: RecordRef,
    changes: Vec<FieldChange>,
    values: Value,
}

impl CreatedContact {
    pub fn id(&self) -> Uuid {
        self.record.record_id
    }

    pub async fn committed(&self, ctx: &Ctx<'_>) {
        ctx.fields()
            .pipeline()
            .committed(
                ctx.scope,
                &RecordWrite {
                    record: &self.record,
                    mode: WriteMode::Create,
                    changes: &self.changes,
                    values: &self.values,
                },
            )
            .await;
    }
}

pub async fn create_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    command: SaveContactCommand,
) -> Result<CreatedContact, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::CONTACTS_CREATE)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::CONTACT.as_str())?;
    let mut input = command.input;
    input.contributed = contributed_input(definition, &input.contributed);
    let provided: Vec<String> = input
        .provided_keys()
        .iter()
        .map(|key| (*key).to_owned())
        .collect();
    input.first_name.get_or_insert(None);
    input
        .owner_id
        .get_or_insert_with(|| Some(scope.user_id().as_str().to_owned()));

    let mut validator = InputValidator::new();
    let contributed =
        merge_contributed(definition, &Map::new(), &input.contributed, &mut validator);
    let data = apply_contact_input(&ContactData::default(), input, &mut validator);
    check_field_access(
        ctx,
        &provided,
        &contact_values(&ContactData::default(), &Map::new()),
        &contact_values(&data, &contributed),
    )
    .await?;

    let record = RecordRef::new(RecordType::CONTACT, Uuid::now_v7());
    let fields = ctx.fields();
    let (data, contributed, custom_fields) = saving(
        ctx,
        tx,
        definition,
        &record,
        WriteMode::Create,
        (data, contributed),
        command.custom_fields,
        &mut validator,
    )
    .await?;
    check_references(ctx, None, &data, &mut validator).await?;

    let now = ctx.now();
    let contact = Contact {
        id: record.record_id,
        data,
        engagement: Default::default(),
        created_by: Some(scope.user_id().clone()),
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    let values = contact_values(&contact.data, &contributed);
    let custom = CustomFieldWriter::new(&fields)
        .with_record_values(&values)
        .apply(tx, scope, &record, &custom_fields, WriteMode::Create)
        .await;
    let custom = merge_violations(validator, custom, INVALID_CONTACT)?;
    ContactStore::of(ctx).insert(tx, &contact).await?;
    write_contributed(tx, definition, contact.id, &contributed).await?;
    let mut changes = diff_values(&Value::Null, &values, &DiffOptions::default())?;
    changes.extend(custom);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Create,
        changes: &changes,
        values: &values,
    };
    let pipeline = fields.pipeline();
    pipeline.validate(tx, scope, &write).await?;
    pipeline.saved(tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: CONTACT_CREATED,
            payload: json!({
                "name": contact.full_name(),
                "company_id": contact.data.company_id.map(|id| id.to_string()),
            }),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    Ok(CreatedContact {
        record,
        changes,
        values,
    })
}

#[allow(clippy::too_many_arguments)]
async fn saving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    definition: &RecordTypeDefinition,
    record: &RecordRef,
    mode: WriteMode,
    (data, contributed): (ContactData, Map<String, Value>),
    custom_fields: Map<String, Value>,
    validator: &mut InputValidator,
) -> Result<(ContactData, Map<String, Value>, Map<String, Value>), AppError> {
    let values = contact_values(&data, &contributed);
    let draft = ctx
        .pipeline()
        .saving(
            tx,
            ctx.scope,
            RecordDraft::new(record.clone(), mode, &values, custom_fields),
        )
        .await?;
    let changed = draft.changed_values(&values);
    if changed.is_empty() {
        return Ok((data, contributed, draft.custom_fields));
    }
    let contributed_changes: Map<String, Value> = changed
        .iter()
        .filter(|(key, _)| is_writable_contributed(definition, key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let contributed = merge_contributed(definition, &contributed, &contributed_changes, validator);
    let data = apply_contact_input(&data, ContactInput::from_values(&changed), validator);
    Ok((data, contributed, draft.custom_fields))
}

pub async fn update(
    ctx: &Ctx<'_>,
    contact_id: &str,
    command: SaveContactCommand,
) -> Result<ContactDetails, AppError> {
    let mut tx = ctx.begin().await?;
    let (id, pending) = update_in_tx(ctx, &mut tx, contact_id, command).await?;
    match pending {
        Some(pending) => {
            tx.commit().await?;
            pending.committed(ctx).await;
        }
        None => drop(tx),
    }
    reload(ctx, id).await
}

pub async fn update_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    contact_id: &str,
    command: SaveContactCommand,
) -> Result<(Uuid, Option<PendingCommit>), AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, contact_id, &Permission::CONTACTS_UPDATE).await?;
    ensure_version(command.expected_updated_at, current.contact.updated_at)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::CONTACT.as_str())?;
    let id = current.contact.id;
    let old = &current.contact.data;
    let old_contributed = contributed_input(definition, &current.contributed);

    let mut validator = InputValidator::new();
    let mut input = command.input;
    input.contributed = contributed_input(definition, &input.contributed);
    let provided: Vec<String> = input
        .provided_keys()
        .iter()
        .map(|key| (*key).to_owned())
        .collect();
    let contributed = merge_contributed(
        definition,
        &old_contributed,
        &input.contributed,
        &mut validator,
    );
    let data = apply_contact_input(old, input, &mut validator);
    let old_values = contact_values(old, &old_contributed);
    check_field_access(
        ctx,
        &provided,
        &old_values,
        &contact_values(&data, &contributed),
    )
    .await?;

    let record = RecordRef::new(RecordType::CONTACT, id);
    let fields = ctx.fields();
    let store = ContactStore::of(ctx);
    aphel_kernel::records::lock_version(
        tx,
        "contacts",
        current.contact.id,
        command.expected_updated_at,
    )
    .await?;
    let (data, contributed, custom_fields) = saving(
        ctx,
        tx,
        definition,
        &record,
        WriteMode::Update,
        (data, contributed),
        command.custom_fields,
        &mut validator,
    )
    .await?;
    check_references(ctx, Some(old), &data, &mut validator).await?;
    let values = contact_values(&data, &contributed);
    let custom = CustomFieldWriter::new(&fields)
        .with_record_values(&values)
        .apply(tx, scope, &record, &custom_fields, WriteMode::Update)
        .await;
    let custom = merge_violations(validator, custom, INVALID_CONTACT)?;
    let mut changes = diff_values(&old_values, &values, &DiffOptions::default())?;
    let system_changed = !changes.is_empty();
    changes.extend(custom);
    if changes.is_empty() {
        return Ok((id, None));
    }
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = fields.pipeline();
    pipeline.validate(tx, scope, &write).await?;

    let now = ctx.now();
    if system_changed {
        store.update(tx, id, &data, now).await?;
        write_contributed(tx, definition, id, &contributed).await?;
    }
    if let Some(company_id) = data.company_id
        && data.company_id != old.company_id
    {
        store.delete_link(tx, id, company_id).await?;
    }
    pipeline.saved(tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: CONTACT_UPDATED,
            payload: changes_payload(&changes),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    Ok((
        id,
        Some(PendingCommit {
            record,
            mode: WriteMode::Update,
            changes,
            values,
        }),
    ))
}

async fn check_field_access(
    ctx: &Ctx<'_>,
    provided: &[String],
    old: &Value,
    new: &Value,
) -> Result<(), AppError> {
    let provided: Vec<&str> = provided.iter().map(String::as_str).collect();
    check_record_writes(
        ctx.fields().definitions(),
        ctx.scope,
        &RecordType::CONTACT,
        &provided,
        old,
        new,
    )
    .await
}

pub(crate) async fn archive(
    ctx: &Ctx<'_>,
    contact_id: &str,
    ip: Option<String>,
) -> Result<ContactDetails, AppError> {
    set_archived(ctx, contact_id, true, ip).await
}

pub(crate) async fn unarchive(
    ctx: &Ctx<'_>,
    contact_id: &str,
    ip: Option<String>,
) -> Result<ContactDetails, AppError> {
    set_archived(ctx, contact_id, false, ip).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    contact_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, contact_id, &Permission::CONTACTS_DELETE).await?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::CONTACT.as_str())?;
    let id = current.contact.id;
    let now = ctx.now();
    let record = RecordRef::new(RecordType::CONTACT, id);
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    pipeline.deleting(&mut tx, scope, &record).await?;
    ContactStore::of(ctx).soft_delete(&mut tx, id, now).await?;
    pipeline.deleted(&mut tx, scope, &record).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record,
            event_type: CONTACT_DELETED,
            payload: json!({ "name": current.contact.full_name() }),
            changes: diff_values(
                &view_values(definition, &current),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn list_companies(
    ctx: &Ctx<'_>,
    contact_id: &str,
    window: ListWindow,
) -> Result<Listing<ContactCompanyLink>, AppError> {
    let contact = visible(ctx, contact_id).await?;
    let Ok(visibility) = ctx.scope.record_filter(&Permission::COMPANIES_READ) else {
        return Ok(Listing {
            items: Vec::new(),
            total: 0,
        });
    };
    ContactStore::of(ctx)
        .list_companies(ctx.scope, &visibility, contact.contact.id, window)
        .await
}

pub(crate) async fn link_company(
    ctx: &Ctx<'_>,
    contact_id: &str,
    command: LinkCompanyCommand,
) -> Result<ContactCompanyLink, AppError> {
    let scope = ctx.scope;
    let contact = changeable(ctx, contact_id, &Permission::CONTACTS_UPDATE).await?;
    let mut violations = Vec::new();
    let company_id = Uuid::parse_str(command.company_id.trim()).ok();
    if company_id.is_none() {
        violations.push(FieldViolation::new("company_id", "must be a record id"));
    }
    let role = command.role;
    if let Some(id) = company_id
        && !company_visible(ctx, id).await?
    {
        violations.push(FieldViolation::new(
            "company_id",
            FieldValueError::ReferenceNotFound.message(),
        ));
    }
    let Some(company_id) = company_id.filter(|_| violations.is_empty()) else {
        violations.sort_by(|left, right| left.field.cmp(&right.field));
        return Err(AppError::invalid_fields("invalid link", violations));
    };
    let contact_id = contact.contact.id;
    let store = ContactStore::of(ctx);
    if contact.contact.data.company_id == Some(company_id)
        || store
            .find_link(scope, contact_id, company_id)
            .await?
            .is_some()
    {
        return Err(already_linked());
    }

    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    store
        .insert_link(
            &mut tx,
            &NewContactCompanyLink {
                id: Uuid::now_v7(),
                contact_id,
                company_id,
                role,
                created_at: now,
            },
        )
        .await?;
    let role_value = role.map(|role| role.as_str());
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::CONTACT, contact_id),
            event_type: CONTACT_COMPANY_LINKED,
            payload: json!({ "company_id": company_id.to_string(), "role": role_value }),
            changes: vec![FieldChange::new(
                format!("companies.{company_id}"),
                Value::Null,
                json!({ "role": role_value }),
            )],
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    store
        .find_link(scope, contact_id, company_id)
        .await?
        .ok_or_else(link_not_found)
}

pub(crate) async fn unlink_company(
    ctx: &Ctx<'_>,
    contact_id: &str,
    company_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let contact = changeable(ctx, contact_id, &Permission::CONTACTS_UPDATE).await?;
    let contact_id = contact.contact.id;
    let company_id = parse_record_id(company_id, link_not_found)?;
    let store = ContactStore::of(ctx);
    let link = store
        .find_link(scope, contact_id, company_id)
        .await?
        .ok_or_else(link_not_found)?;

    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !store.delete_link(&mut tx, contact_id, company_id).await? {
        return Err(link_not_found());
    }
    let role_value = link.role.map(|role| role.as_str());
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::CONTACT, contact_id),
            event_type: CONTACT_COMPANY_UNLINKED,
            payload: json!({ "company_id": company_id.to_string(), "role": role_value }),
            changes: vec![FieldChange::new(
                format!("companies.{company_id}"),
                json!({ "role": role_value }),
                Value::Null,
            )],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn list_company_contacts(
    ctx: &Ctx<'_>,
    company_id: &str,
    window: ListWindow,
) -> Result<Listing<CompanyContactLink>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::COMPANIES_READ)?;
    let company_id = parse_record_id(company_id, company_not_found)?;
    if !company_visible(ctx, company_id).await? {
        return Err(company_not_found());
    }
    let visibility = scope.record_filter(&Permission::CONTACTS_READ)?;
    ContactStore::of(ctx)
        .list_company_contacts(scope, &visibility, company_id, window)
        .await
}

async fn set_archived(
    ctx: &Ctx<'_>,
    contact_id: &str,
    archived: bool,
    ip: Option<String>,
) -> Result<ContactDetails, AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, contact_id, &Permission::CONTACTS_ARCHIVE).await?;
    if current.contact.is_archived() == archived {
        return details(ctx, current).await;
    }
    let id = current.contact.id;
    let now = ctx.now();
    let archived_at = archived.then_some(now);
    let mut tx = ctx.begin().await?;
    ContactStore::of(ctx)
        .set_archived(&mut tx, id, archived_at)
        .await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::CONTACT, id),
            event_type: if archived {
                CONTACT_ARCHIVED
            } else {
                CONTACT_UNARCHIVED
            },
            payload: json!({ "name": current.contact.full_name() }),
            changes: vec![archive_change(current.contact.archived_at, archived_at)],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    reload(ctx, id).await
}

async fn check_references(
    ctx: &Ctx<'_>,
    old: Option<&ContactData>,
    data: &ContactData,
    validator: &mut InputValidator,
) -> Result<(), AppError> {
    if old.is_none_or(|old| old.owner_id != data.owner_id) {
        check_owner(&ctx.fields(), data.owner_id.as_ref(), validator).await?;
    }
    if let Some(company_id) = data.company_id
        && old.is_none_or(|old| old.company_id != data.company_id)
        && !company_visible(ctx, company_id).await?
    {
        validator.violation("company_id", FieldValueError::ReferenceNotFound.message());
    }
    Ok(())
}

async fn company_visible(ctx: &Ctx<'_>, company_id: Uuid) -> Result<bool, AppError> {
    let Ok(visibility) = ctx.scope.record_filter(&Permission::COMPANIES_READ) else {
        return Ok(false);
    };
    let fields = ctx.fields();
    let definition = fields.registry().require(RecordType::COMPANY.as_str())?;
    Ok(!fields
        .references()
        .existing_records(ctx.scope, definition, &visibility, &[company_id])
        .await?
        .is_empty())
}

pub async fn visible(ctx: &Ctx<'_>, contact_id: &str) -> Result<ContactView, AppError> {
    let visibility = ctx.scope.record_filter(&Permission::CONTACTS_READ)?;
    let id = parse_record_id(contact_id, contact_not_found)?;
    ContactStore::of(ctx)
        .find(ctx.scope, &visibility, id)
        .await?
        .ok_or_else(contact_not_found)
}

pub async fn changeable(
    ctx: &Ctx<'_>,
    contact_id: &str,
    permission: &Permission,
) -> Result<ContactView, AppError> {
    let current = visible(ctx, contact_id).await?;
    let allowed = ctx.scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && ContactStore::of(ctx)
            .find(ctx.scope, &allowed, current.contact.id)
            .await?
            .is_none()
    {
        return Err(permission_denied());
    }
    Ok(current)
}

pub(crate) async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<ContactDetails, AppError> {
    let view = ContactStore::of(ctx)
        .find(ctx.scope, &RecordScopeFilter::Organization, id)
        .await?
        .ok_or_else(contact_not_found)?;
    details(ctx, view).await
}

async fn details(ctx: &Ctx<'_>, view: ContactView) -> Result<ContactDetails, AppError> {
    Ok(with_custom_fields(ctx, vec![view]).await?.remove(0))
}

async fn with_custom_fields(
    ctx: &Ctx<'_>,
    views: Vec<ContactView>,
) -> Result<Vec<ContactDetails>, AppError> {
    let ids: Vec<Uuid> = views.iter().map(|view| view.contact.id).collect();
    let mut values = CustomFieldReader::new(&ctx.fields())
        .load(ctx.scope, &RecordType::CONTACT, &ids)
        .await?;
    Ok(views
        .into_iter()
        .map(|view| ContactDetails {
            custom_fields: values.remove(&view.contact.id).unwrap_or_default(),
            view,
        })
        .collect())
}
