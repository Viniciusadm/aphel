use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::{
    INVALID_PRODUCT, PRODUCT_CREATED, PRODUCT_DELETED, PRODUCT_UPDATED, Product, ProductData,
    ProductDetails, ProductInput, SaveProductCommand, apply_product_input, product_not_found,
    product_snapshot, require_catalog,
};
use crate::store::ProductStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, RecordDraft, RecordPipeline,
    RecordWrite, WriteMode, decimal_json,
};
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::{RecordQuery, RecordQueryEngine};
use aphel_kernel::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    InputValidator, RecordMutation, changes_payload, merge_violations, parse_record_id,
    publish_mutation,
};

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: RecordQuery,
) -> Result<Listing<ProductDetails>, AppError> {
    let scope = ctx.scope;
    let page = RecordQueryEngine::new(&ctx.query())
        .query(scope, RecordType::PRODUCT.as_str(), query)
        .await?;
    let products = ProductStore::of(ctx).find_many(&page.items).await?;
    let ordered = page
        .items
        .iter()
        .filter_map(|id| products.iter().find(|product| product.id == *id).cloned())
        .collect();
    Ok(Listing {
        items: with_custom_fields(ctx, ordered).await?,
        total: page.total,
    })
}

pub(crate) async fn get(ctx: &Ctx<'_>, product_id: &str) -> Result<ProductDetails, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_READ)?;
    let product = existing(ctx, product_id).await?;
    details(ctx, product).await
}

pub async fn create(
    ctx: &Ctx<'_>,
    command: SaveProductCommand,
) -> Result<ProductDetails, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_CREATE)?;
    let provided = command.input.provided_keys();
    let mut input = command.input;
    input.name.get_or_insert(None);
    input.currency.get_or_insert(None);
    let currency = ProductStore::of(ctx).organization_currency().await?;

    let mut validator = InputValidator::new();
    let base = ProductData {
        currency: currency.clone(),
        ..ProductData::default()
    };
    let data = apply_product_input(&base, input, &currency, &mut validator);
    check_code(ctx, &data, None, &mut validator).await?;
    check_field_access(
        ctx,
        &provided,
        &product_snapshot(&base),
        &product_snapshot(&data),
    )
    .await?;

    let now = ctx.now();
    let id = Uuid::now_v7();
    let record = RecordRef::new(RecordType::PRODUCT, id);
    let mut tx = ctx.begin().await?;
    let (data, custom_fields) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Create,
        (data, &currency),
        command.custom_fields,
        &mut validator,
    )
    .await?;
    let product = Product {
        id,
        data,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let values = product_snapshot(&product.data);
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(&mut tx, scope, &record, &custom_fields, WriteMode::Create)
        .await;
    let custom = merge_violations(validator, custom, INVALID_PRODUCT)?;
    ProductStore::of(ctx).insert(&mut tx, &product).await?;
    let mut changes = diff_values(&Value::Null, &values, &DiffOptions::default())?;
    changes.extend(custom);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Create,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: PRODUCT_CREATED,
            payload: json!({
                "name": product.data.name,
                "code": product.data.code,
                "price": decimal_json(product.data.price),
                "currency": product.data.currency,
            }),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    reload(ctx, product.id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    product_id: &str,
    command: SaveProductCommand,
) -> Result<ProductDetails, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let current = existing(ctx, product_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let id = current.id;
    let old = &current.data;

    let mut validator = InputValidator::new();
    let provided = command.input.provided_keys();
    let data = apply_product_input(old, command.input, &old.currency, &mut validator);
    if data.code != old.code {
        check_code(ctx, &data, Some(id), &mut validator).await?;
    }
    check_field_access(
        ctx,
        &provided,
        &product_snapshot(old),
        &product_snapshot(&data),
    )
    .await?;

    let record = RecordRef::new(RecordType::PRODUCT, id);
    let mut tx = ctx.begin().await?;
    aphel_kernel::records::lock_version(
        &mut tx,
        "products",
        current.id,
        command.expected_updated_at,
    )
    .await?;
    let (data, custom_fields) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Update,
        (data, &old.currency),
        command.custom_fields,
        &mut validator,
    )
    .await?;
    let values = product_snapshot(&data);
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&values)
        .apply(&mut tx, scope, &record, &custom_fields, WriteMode::Update)
        .await;
    let custom = merge_violations(validator, custom, INVALID_PRODUCT)?;
    let mut changes = diff_values(&product_snapshot(old), &values, &DiffOptions::default())?;
    changes.extend(custom);
    if changes.is_empty() {
        return details(ctx, current).await;
    }
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    let now = ctx.now();
    ProductStore::of(ctx)
        .update(&mut tx, id, &data, now)
        .await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: PRODUCT_UPDATED,
            payload: changes_payload(&changes),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    reload(ctx, id).await
}

async fn saving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    mode: WriteMode,
    (data, currency): (ProductData, &str),
    custom_fields: Map<String, Value>,
    validator: &mut InputValidator,
) -> Result<(ProductData, Map<String, Value>), AppError> {
    let values = product_snapshot(&data);
    let draft = ctx
        .fields()
        .pipeline()
        .saving(
            tx,
            ctx.scope,
            RecordDraft::new(record.clone(), mode, &values, custom_fields),
        )
        .await?;
    let changed = draft.changed_values(&values);
    if changed.is_empty() {
        return Ok((data, draft.custom_fields));
    }
    let data = apply_product_input(
        &data,
        ProductInput::from_values(&changed),
        currency,
        validator,
    );
    Ok((data, draft.custom_fields))
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    product_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_DELETE)?;
    let current = existing(ctx, product_id).await?;
    let now = ctx.now();
    let record = RecordRef::new(RecordType::PRODUCT, current.id);
    let pipeline = ctx.pipeline();
    let mut tx = ctx.begin().await?;
    pipeline.deleting(&mut tx, scope, &record).await?;
    ProductStore::of(ctx)
        .soft_delete(&mut tx, current.id, now)
        .await?;
    pipeline.deleted(&mut tx, scope, &record).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record,
            event_type: PRODUCT_DELETED,
            payload: json!({ "name": current.data.name }),
            changes: diff_values(
                &product_snapshot(&current.data),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

async fn existing(ctx: &Ctx<'_>, product_id: &str) -> Result<Product, AppError> {
    let id = parse_record_id(product_id, product_not_found)?;
    ProductStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(product_not_found)
}

async fn check_code(
    ctx: &Ctx<'_>,
    data: &ProductData,
    except: Option<Uuid>,
    validator: &mut InputValidator,
) -> Result<(), AppError> {
    if let Some(code) = &data.code
        && ProductStore::of(ctx).code_taken(code, except).await?
    {
        validator.violation("code", "is already used by another product");
    }
    Ok(())
}

async fn check_field_access(
    ctx: &Ctx<'_>,
    provided: &[&str],
    old: &Value,
    new: &Value,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    check_record_writes(
        ctx.fields().definitions(),
        scope,
        &RecordType::PRODUCT,
        provided,
        old,
        new,
    )
    .await
}

async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<ProductDetails, AppError> {
    let product = ProductStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(product_not_found)?;
    details(ctx, product).await
}

async fn details(ctx: &Ctx<'_>, product: Product) -> Result<ProductDetails, AppError> {
    Ok(with_custom_fields(ctx, vec![product]).await?.remove(0))
}

async fn with_custom_fields(
    ctx: &Ctx<'_>,
    products: Vec<Product>,
) -> Result<Vec<ProductDetails>, AppError> {
    let scope = ctx.scope;
    let ids: Vec<Uuid> = products.iter().map(|product| product.id).collect();
    let mut values = CustomFieldReader::new(&ctx.fields())
        .load(scope, &RecordType::PRODUCT, &ids)
        .await?;
    Ok(products
        .into_iter()
        .map(|product| ProductDetails {
            custom_fields: values.remove(&product.id).unwrap_or_default(),
            product,
        })
        .collect())
}
