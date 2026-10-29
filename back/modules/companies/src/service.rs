use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{
    COMPANY_ARCHIVED, COMPANY_CREATED, COMPANY_DELETED, COMPANY_UNARCHIVED, COMPANY_UPDATED,
    Company, CompanyData, CompanyDetails, CompanyInput, CompanyView, INVALID_COMPANY,
    SaveCompanyCommand, apply_company_input, company_not_found, company_snapshot,
};
use crate::hierarchy::{HierarchyStore, Reparent, check_parent, reparent_children};
use crate::store::CompanyStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, RecordDraft, RecordPipeline,
    RecordTypeDefinition, RecordWrite, WriteMode,
};
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::{RecordQuery, RecordQueryEngine};
use aphel_kernel::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    InputValidator, PendingCommit, RecordMutation, archive_change, changes_payload, check_owner,
    contributed_input, is_writable_contributed, merge_contributed, merge_violations,
    normalize_search_term, parse_record_id, permission_denied, publish_mutation, with_contributed,
    write_contributed,
};

pub(crate) fn company_values(data: &CompanyData, contributed: &Map<String, Value>) -> Value {
    with_contributed(company_snapshot(data), contributed)
}

pub(crate) fn view_values(definition: &RecordTypeDefinition, view: &CompanyView) -> Value {
    company_values(
        &view.company.data,
        &contributed_input(definition, &view.contributed),
    )
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    mut query: RecordQuery,
) -> Result<Listing<CompanyDetails>, AppError> {
    query.search = query.search.as_deref().map(normalize_search_term);
    let page = RecordQueryEngine::new(&ctx.query())
        .query(ctx.scope, RecordType::COMPANY.as_str(), query)
        .await?;
    let views = CompanyStore::of(ctx).find_many(&page.items).await?;
    let mut ordered = Vec::with_capacity(page.items.len());
    for id in &page.items {
        if let Some(view) = views.iter().find(|view| view.company.id == *id) {
            ordered.push(view.clone());
        }
    }
    Ok(Listing {
        items: with_custom_fields(ctx, ordered).await?,
        total: page.total,
    })
}

pub(crate) async fn get(ctx: &Ctx<'_>, company_id: &str) -> Result<CompanyDetails, AppError> {
    let view = visible(ctx, company_id).await?;
    details(ctx, view).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: SaveCompanyCommand,
) -> Result<CompanyDetails, AppError> {
    let mut tx = ctx.begin().await?;
    let pending = create_in_tx(ctx, &mut tx, command).await?;
    tx.commit().await?;
    pending.committed(ctx).await;
    reload(ctx, pending.record.record_id).await
}

pub async fn create_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    command: SaveCompanyCommand,
) -> Result<PendingCommit, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::COMPANIES_CREATE)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::COMPANY.as_str())?;
    let mut input = command.input;
    input.contributed = contributed_input(definition, &input.contributed);
    let provided: Vec<String> = input
        .provided_keys()
        .iter()
        .map(|key| (*key).to_owned())
        .collect();
    input.name.get_or_insert(None);
    input
        .owner_id
        .get_or_insert_with(|| Some(scope.user_id().as_str().to_owned()));

    let mut validator = InputValidator::new();
    let contributed =
        merge_contributed(definition, &Map::new(), &input.contributed, &mut validator);
    let data = apply_company_input(&CompanyData::default(), input, &mut validator);
    check_field_access(
        ctx,
        &provided,
        &company_values(&CompanyData::default(), &Map::new()),
        &company_values(&data, &contributed),
    )
    .await?;

    let record = RecordRef::new(RecordType::COMPANY, Uuid::now_v7());
    let fields = ctx.fields();
    let store = CompanyStore::of(ctx);
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
    check_owner(&fields, data.owner_id.as_ref(), &mut validator).await?;

    let now = ctx.now();
    let company = Company {
        id: record.record_id,
        data,
        engagement: Default::default(),
        created_by: Some(scope.user_id().clone()),
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    check_parent(
        &HierarchyStore::of(ctx),
        &store,
        tx,
        scope,
        company.id,
        company.data.parent_company_id,
    )
    .await?;
    let values = company_values(&company.data, &contributed);
    let custom = CustomFieldWriter::new(&fields)
        .with_record_values(&values)
        .apply(tx, scope, &record, &custom_fields, WriteMode::Create)
        .await;
    let custom = merge_violations(validator, custom, INVALID_COMPANY)?;
    store.insert(tx, &company).await?;
    write_contributed(tx, definition, company.id, &contributed).await?;
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
            event_type: COMPANY_CREATED,
            payload: json!({ "name": company.data.name }),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    Ok(PendingCommit {
        record,
        mode: WriteMode::Create,
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
    (data, contributed): (CompanyData, Map<String, Value>),
    custom_fields: Map<String, Value>,
    validator: &mut InputValidator,
) -> Result<(CompanyData, Map<String, Value>, Map<String, Value>), AppError> {
    let values = company_values(&data, &contributed);
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
    let data = apply_company_input(&data, CompanyInput::from_values(&changed), validator);
    Ok((data, contributed, draft.custom_fields))
}

pub async fn update(
    ctx: &Ctx<'_>,
    company_id: &str,
    command: SaveCompanyCommand,
) -> Result<CompanyDetails, AppError> {
    let mut tx = ctx.begin().await?;
    let (id, pending) = update_in_tx(ctx, &mut tx, company_id, command).await?;
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
    company_id: &str,
    command: SaveCompanyCommand,
) -> Result<(Uuid, Option<PendingCommit>), AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, company_id, &Permission::COMPANIES_UPDATE).await?;
    ensure_version(command.expected_updated_at, current.company.updated_at)?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::COMPANY.as_str())?;
    let id = current.company.id;
    let old = &current.company.data;
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
    let data = apply_company_input(old, input, &mut validator);
    let old_values = company_values(old, &old_contributed);
    check_field_access(
        ctx,
        &provided,
        &old_values,
        &company_values(&data, &contributed),
    )
    .await?;

    let record = RecordRef::new(RecordType::COMPANY, id);
    let fields = ctx.fields();
    let store = CompanyStore::of(ctx);
    aphel_kernel::records::lock_version(
        tx,
        "companies",
        current.company.id,
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
    if data.owner_id != old.owner_id {
        check_owner(&fields, data.owner_id.as_ref(), &mut validator).await?;
    }
    if data.parent_company_id != old.parent_company_id {
        check_parent(
            &HierarchyStore::of(ctx),
            &store,
            tx,
            scope,
            id,
            data.parent_company_id,
        )
        .await?;
    }
    let values = company_values(&data, &contributed);
    let custom = CustomFieldWriter::new(&fields)
        .with_record_values(&values)
        .apply(tx, scope, &record, &custom_fields, WriteMode::Update)
        .await;
    let custom = merge_violations(validator, custom, INVALID_COMPANY)?;
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
    pipeline.saved(tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: COMPANY_UPDATED,
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

pub(crate) async fn archive(
    ctx: &Ctx<'_>,
    company_id: &str,
    ip: Option<String>,
) -> Result<CompanyDetails, AppError> {
    set_archived(ctx, company_id, true, ip).await
}

pub(crate) async fn unarchive(
    ctx: &Ctx<'_>,
    company_id: &str,
    ip: Option<String>,
) -> Result<CompanyDetails, AppError> {
    set_archived(ctx, company_id, false, ip).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    company_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, company_id, &Permission::COMPANIES_DELETE).await?;
    let registry = ctx.registry().clone();
    let definition = registry.require(RecordType::COMPANY.as_str())?;
    let id = current.company.id;
    let now = ctx.now();
    let record = RecordRef::new(RecordType::COMPANY, id);
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    pipeline.deleting(&mut tx, scope, &record).await?;
    CompanyStore::of(ctx).soft_delete(&mut tx, id, now).await?;
    pipeline.deleted(&mut tx, scope, &record).await?;
    let hierarchy = HierarchyStore::of(ctx);
    let detached = reparent_children(
        &hierarchy,
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        Reparent {
            from_parent: id,
            to_parent: current.company.data.parent_company_id,
            except: None,
            now,
            ip: ip.clone(),
        },
    )
    .await?;
    hierarchy.mark_detached(&mut tx, &detached, id).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record,
            event_type: COMPANY_DELETED,
            payload: json!({ "name": current.company.data.name }),
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

async fn set_archived(
    ctx: &Ctx<'_>,
    company_id: &str,
    archived: bool,
    ip: Option<String>,
) -> Result<CompanyDetails, AppError> {
    let scope = ctx.scope;
    let current = changeable(ctx, company_id, &Permission::COMPANIES_ARCHIVE).await?;
    if current.company.is_archived() == archived {
        return details(ctx, current).await;
    }
    let id = current.company.id;
    let now = ctx.now();
    let archived_at = archived.then_some(now);
    let mut tx = ctx.begin().await?;
    CompanyStore::of(ctx)
        .set_archived(&mut tx, id, archived_at)
        .await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::COMPANY, id),
            event_type: if archived {
                COMPANY_ARCHIVED
            } else {
                COMPANY_UNARCHIVED
            },
            payload: json!({ "name": current.company.data.name }),
            changes: vec![archive_change(current.company.archived_at, archived_at)],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    reload(ctx, id).await
}

pub(crate) async fn visible(ctx: &Ctx<'_>, company_id: &str) -> Result<CompanyView, AppError> {
    let visibility = ctx.scope.record_filter(&Permission::COMPANIES_READ)?;
    let id = parse_record_id(company_id, company_not_found)?;
    CompanyStore::of(ctx)
        .find(ctx.scope, &visibility, id)
        .await?
        .ok_or_else(company_not_found)
}

pub(crate) async fn changeable(
    ctx: &Ctx<'_>,
    company_id: &str,
    permission: &Permission,
) -> Result<CompanyView, AppError> {
    let current = visible(ctx, company_id).await?;
    let allowed = ctx.scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && CompanyStore::of(ctx)
            .find(ctx.scope, &allowed, current.company.id)
            .await?
            .is_none()
    {
        return Err(permission_denied());
    }
    Ok(current)
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
        &RecordType::COMPANY,
        &provided,
        old,
        new,
    )
    .await
}

pub(crate) async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<CompanyDetails, AppError> {
    let view = CompanyStore::of(ctx)
        .find(ctx.scope, &RecordScopeFilter::Organization, id)
        .await?
        .ok_or_else(company_not_found)?;
    details(ctx, view).await
}

async fn details(ctx: &Ctx<'_>, view: CompanyView) -> Result<CompanyDetails, AppError> {
    Ok(with_custom_fields(ctx, vec![view]).await?.remove(0))
}

async fn with_custom_fields(
    ctx: &Ctx<'_>,
    views: Vec<CompanyView>,
) -> Result<Vec<CompanyDetails>, AppError> {
    let ids: Vec<Uuid> = views.iter().map(|view| view.company.id).collect();
    let mut values = CustomFieldReader::new(&ctx.fields())
        .load(ctx.scope, &RecordType::COMPANY, &ids)
        .await?;
    Ok(views
        .into_iter()
        .map(|view| CompanyDetails {
            custom_fields: values.remove(&view.company.id).unwrap_or_default(),
            view,
        })
        .collect())
}
