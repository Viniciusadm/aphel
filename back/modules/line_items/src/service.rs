use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    HostRecord, INVALID_LINE_ITEM, LINE_ITEM_ADDED, LINE_ITEM_REMOVED,
    LINE_ITEM_SETTINGS_RECORD_TYPE, LINE_ITEM_SETTINGS_UPDATED, LINE_ITEM_UPDATED,
    LINE_ITEMS_LIMIT_REACHED, LINE_ITEMS_REORDERED, LineItemInput, LineItemLine, LineItemSettings,
    LineItemsSummary, PRODUCT_CURRENCY_MISMATCH, RECORD_HAS_NO_ITEMS, ReorderLineItemsCommand,
    SaveLineItemCommand, UpdateLineItemSettingsCommand, item_changes, item_not_found,
    item_snapshot, line, line_total, parse_decimal,
};
use crate::recurring::apply_item_recurrence;
use crate::store::LineItemStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::permissions::{
    FieldAccess, FieldAccessPolicy, check_access,
};
use aphel_kernel::application::fields::{CustomFieldPorts, ReferenceChecker};
use aphel_kernel::application::query::RecordQueryPorts;
use aphel_kernel::application::shared::{FieldChange, RecordRef, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{InputValidator, RecordMutation, publish_mutation};
use aphel_sdk::line_items::{
    AmountMode, DiscountKind, LineData, LineItem, MAX_ITEM_NAME_LENGTH, MAX_LINES_PER_RECORD,
    MAX_UNIT_LENGTH, MONEY_DECIMALS, QUANTITY_DECIMALS, items_total, normalize_quantity,
    parse_money,
};
use aphel_sdk::money::base_amount;

use aphel_sdk::line_items::catalog_item;
fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}
use aphel_sdk::line_items::{LineItemProjection, LineItemTraits, LineItemsChanged, TotalsChanged};

struct Stamp {
    record: RecordRef,
    now: DateTime<Utc>,
    ip: Option<String>,
}

fn convert_item(item: &mut LineItem, rate: Option<Decimal>) {
    item.exchange_rate = rate;
    item.amount_base = base_amount(Some(item.total), rate);
}

pub async fn summary(ctx: &Ctx<'_>, record_ref: &RecordRef) -> Result<LineItemsSummary, AppError> {
    let parent = host_for(ctx, record_ref, false).await?;
    let access = amount_access(ctx, record_ref).await?;
    if access == FieldAccess::Hidden {
        check_access([(access, "amount".to_owned())])?;
    }
    let settings = current_settings(ctx).await?;
    let items = LineItemStore::of(ctx).list(&parent.record).await?;
    let mode = if has_projection(record_ref) {
        settings.amount_mode
    } else {
        AmountMode::Manual
    };
    Ok(build_summary(&parent, items, mode))
}

pub async fn add(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    command: SaveLineItemCommand,
) -> Result<LineItemLine, AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    let mode = writable_mode(ctx, record_ref).await?;
    let data = resolve_input(ctx, None, command.input, &parent.currency).await?;
    let total = line_total(&data)?;
    let now = ctx.now();
    let stamp = Stamp {
        record: parent.record.clone(),
        now,
        ip: command.ip,
    };
    let store = LineItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, mut items) = lock(ctx, &store, &mut tx, &parent).await?;
    if items.len() >= MAX_LINES_PER_RECORD {
        return Err(AppError::validation_code(
            LINE_ITEMS_LIMIT_REACHED,
            format!("a record has at most {MAX_LINES_PER_RECORD} items"),
        ));
    }
    let mut item = LineItem {
        id: Uuid::now_v7(),
        record: parent.record.clone(),
        data,
        total,
        amount_base: None,
        exchange_rate: None,
        position: items
            .iter()
            .map(|item| item.position)
            .max()
            .map_or(0, |position| position + 1),
        created_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    convert_item(&mut item, locked.exchange_rate);
    store.insert(&mut tx, &item).await?;
    let snapshot = item_snapshot(&item);
    record(
        ctx,
        &mut tx,
        &stamp,
        LINE_ITEM_ADDED,
        json!({ "item_id": item.id.to_string(), "item": snapshot }),
        item_changes(&Value::Null, &snapshot)?,
    )
    .await?;
    items.push(item.clone());
    project(ctx, &mut tx, &stamp, &items, mode).await?;
    tx.commit().await?;
    Ok(line(item))
}

pub(crate) async fn replace(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    inputs: Vec<LineItemInput>,
    ip: Option<String>,
) -> Result<LineItemsSummary, AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    if inputs.len() > MAX_LINES_PER_RECORD {
        return Err(AppError::validation_code(
            LINE_ITEMS_LIMIT_REACHED,
            format!("a record has at most {MAX_LINES_PER_RECORD} items"),
        ));
    }
    let mode = writable_mode(ctx, record_ref).await?;
    let mut resolved = Vec::with_capacity(inputs.len());
    for input in inputs {
        let data = resolve_input(ctx, None, input, &parent.currency).await?;
        let total = line_total(&data)?;
        resolved.push((data, total));
    }
    let now = ctx.now();
    let stamp = Stamp {
        record: parent.record.clone(),
        now,
        ip,
    };
    let store = LineItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, current) = lock(ctx, &store, &mut tx, &parent).await?;
    for item in current {
        store.delete(&mut tx, &parent.record, item.id).await?;
        record(
            ctx,
            &mut tx,
            &stamp,
            LINE_ITEM_REMOVED,
            json!({ "item_id": item.id.to_string(), "name": item.data.name }),
            item_changes(&item_snapshot(&item), &Value::Null)?,
        )
        .await?;
    }
    let mut items = Vec::with_capacity(resolved.len());
    for (position, (data, total)) in (0_i32..).zip(resolved) {
        let mut item = LineItem {
            id: Uuid::now_v7(),
            record: parent.record.clone(),
            data,
            total,
            amount_base: None,
            exchange_rate: None,
            position,
            created_by: Some(ctx.scope.user_id().clone()),
            created_at: now,
            updated_at: now,
        };
        convert_item(&mut item, locked.exchange_rate);
        store.insert(&mut tx, &item).await?;
        let snapshot = item_snapshot(&item);
        record(
            ctx,
            &mut tx,
            &stamp,
            LINE_ITEM_ADDED,
            json!({ "item_id": item.id.to_string(), "item": snapshot }),
            item_changes(&Value::Null, &snapshot)?,
        )
        .await?;
        items.push(item);
    }
    project(ctx, &mut tx, &stamp, &items, mode).await?;
    let result = summary_locked(ctx, &store, &mut tx, record_ref).await?;
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    item_id: &str,
    command: SaveLineItemCommand,
) -> Result<LineItemLine, AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    let item_id = parse_id(item_id).ok_or_else(item_not_found)?;
    let store = LineItemStore::of(ctx);
    let current = store
        .list(&parent.record)
        .await?
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(item_not_found)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mode = writable_mode(ctx, record_ref).await?;
    let data = resolve_input(ctx, Some(&current.data), command.input, &parent.currency).await?;
    let total = line_total(&data)?;
    if data == current.data && total == current.total {
        return Ok(line(current));
    }
    let now = ctx.now();
    let stamp = Stamp {
        record: parent.record.clone(),
        now,
        ip: command.ip,
    };
    let mut tx = ctx.begin().await?;
    let (locked, mut items) = lock(ctx, &store, &mut tx, &parent).await?;
    let Some(stored) = items.iter_mut().find(|item| item.id == item_id) else {
        return Err(item_not_found());
    };
    ensure_version(Some(current.updated_at), stored.updated_at)?;
    let before = item_snapshot(stored);
    stored.data = data;
    stored.total = total;
    convert_item(stored, locked.exchange_rate);
    stored.updated_at = now;
    let item = stored.clone();
    store.update(&mut tx, &item).await?;
    let changes = item_changes(&before, &item_snapshot(&item))?;
    record(
        ctx,
        &mut tx,
        &stamp,
        LINE_ITEM_UPDATED,
        json!({
            "item_id": item.id.to_string(),
            "name": item.data.name,
            "changes": changes
                .iter()
                .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
                .collect::<Vec<_>>(),
        }),
        changes,
    )
    .await?;
    project(ctx, &mut tx, &stamp, &items, mode).await?;
    tx.commit().await?;
    Ok(line(item))
}

pub(crate) async fn remove(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    item_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    let item_id = parse_id(item_id).ok_or_else(item_not_found)?;
    let mode = writable_mode(ctx, record_ref).await?;
    let stamp = Stamp {
        record: parent.record.clone(),
        now: ctx.now(),
        ip,
    };
    let store = LineItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (_locked, mut items) = lock(ctx, &store, &mut tx, &parent).await?;
    let Some(index) = items.iter().position(|item| item.id == item_id) else {
        return Err(item_not_found());
    };
    let item = items.remove(index);
    store.delete(&mut tx, &parent.record, item.id).await?;
    record(
        ctx,
        &mut tx,
        &stamp,
        LINE_ITEM_REMOVED,
        json!({ "item_id": item.id.to_string(), "name": item.data.name }),
        item_changes(&item_snapshot(&item), &Value::Null)?,
    )
    .await?;
    project(ctx, &mut tx, &stamp, &items, mode).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn reorder(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    command: ReorderLineItemsCommand,
) -> Result<LineItemsSummary, AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    let requested: Vec<Option<Uuid>> = command.item_ids.iter().map(|id| parse_id(id)).collect();
    let now = ctx.now();
    let stamp = Stamp {
        record: parent.record.clone(),
        now,
        ip: command.ip,
    };
    let store = LineItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (_, items) = lock(ctx, &store, &mut tx, &parent).await?;
    let existing: HashSet<Uuid> = items.iter().map(|item| item.id).collect();
    let order: Vec<Uuid> = requested.iter().flatten().copied().collect();
    let unique: HashSet<Uuid> = order.iter().copied().collect();
    if order.len() != requested.len() || unique.len() != order.len() || unique != existing {
        return Err(AppError::invalid_fields(
            INVALID_LINE_ITEM,
            vec![FieldViolation::new(
                "item_ids",
                "must list every item of the record exactly once",
            )],
        ));
    }
    let before: Vec<String> = items.iter().map(|item| item.id.to_string()).collect();
    let after: Vec<String> = order.iter().map(Uuid::to_string).collect();
    if before != after {
        let positions: Vec<(Uuid, i32)> = order
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, i32::try_from(index).unwrap_or(i32::MAX)))
            .collect();
        store
            .set_positions(&mut tx, &parent.record, &positions, now)
            .await?;
        record(
            ctx,
            &mut tx,
            &stamp,
            LINE_ITEMS_REORDERED,
            json!({ "item_ids": after }),
            vec![FieldChange::new("item_order", json!(before), json!(after))],
        )
        .await?;
    }
    let result = summary_locked(ctx, &store, &mut tx, record_ref).await?;
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn apply_total(
    ctx: &Ctx<'_>,
    record_ref: &RecordRef,
    ip: Option<String>,
) -> Result<LineItemsSummary, AppError> {
    let parent = host_for(ctx, record_ref, true).await?;
    ensure_amount_writable(ctx, record_ref).await?;
    if !has_projection(record_ref) {
        return Err(AppError::Validation(
            "the record owner does not support applying item totals".to_owned(),
        ));
    }
    let stamp = Stamp {
        record: parent.record.clone(),
        now: ctx.now(),
        ip,
    };
    let store = LineItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (_locked, items) = lock(ctx, &store, &mut tx, &parent).await?;
    if items.is_empty() {
        return Err(AppError::validation_code(
            RECORD_HAS_NO_ITEMS,
            "the record has no items to sum",
        ));
    }
    project(ctx, &mut tx, &stamp, &items, AmountMode::Items).await?;
    let result = summary_locked(ctx, &store, &mut tx, record_ref).await?;
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<LineItemSettings, AppError> {
    if ctx.scope.require(&Permission::SETTINGS_MANAGE).is_err() {
        ctx.scope
            .require(&aphel_kernel::domain::access::Permission::from_static(
                "line_items.read",
            ))?;
    }
    current_settings(ctx).await
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    command: UpdateLineItemSettingsCommand,
) -> Result<LineItemSettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut tx = ctx.begin().await?;
    let current = LineItemStore::of(ctx).settings_locked(&mut tx).await?;
    if let Some(updated_at) = current.updated_at {
        ensure_version(command.expected_updated_at, updated_at)?;
    }
    let mut settings = current.clone();
    if let Some(mode) = command.amount_mode {
        settings.amount_mode = mode;
    }
    if settings.amount_mode == current.amount_mode && current.updated_at.is_some() {
        return Ok(current);
    }
    let now = ctx.now();
    settings.updated_at = Some(now);
    LineItemStore::of(ctx)
        .save_settings(&mut tx, scope.user_id(), &settings, now)
        .await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(LINE_ITEM_SETTINGS_RECORD_TYPE, Uuid::nil()),
            event_type: LINE_ITEM_SETTINGS_UPDATED,
            payload: json!({ "amount_mode": settings.amount_mode.as_str() }),
            changes: vec![FieldChange::new(
                "amount_mode",
                json!(current.amount_mode.as_str()),
                json!(settings.amount_mode.as_str()),
            )],
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(settings)
}

fn build_summary(parent: &HostRecord, items: Vec<LineItem>, mode: AmountMode) -> LineItemsSummary {
    let total = items_total(&items);
    let lines: Vec<LineItemLine> = items.into_iter().map(line).collect();
    LineItemsSummary {
        record: parent.record.clone(),
        currency: parent.currency.clone(),
        subtotal: lines.iter().map(|line| line.gross).sum(),
        discount_total: lines.iter().map(|line| line.discount).sum(),
        total,
        record_amount: parent.amount,
        amount_mode: mode,
        amount_managed: mode == AmountMode::Items
            && !lines.is_empty()
            && has_projection(&parent.record),
        can_apply_total: has_projection(&parent.record),
        items: lines,
    }
}

async fn resolve_input(
    ctx: &Ctx<'_>,
    base: Option<&LineData>,
    input: LineItemInput,
    currency: &str,
) -> Result<LineData, AppError> {
    let mut validator = InputValidator::new();
    let mut data = base.cloned().unwrap_or_else(|| LineData {
        quantity: Decimal::ONE,
        ..LineData::default()
    });
    let mut product = None;
    if let Some(value) = &input.product_id {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.product_id = None,
            Some(text) => match parse_id(text) {
                None => validator.violation("product_id", "must be a valid id"),
                Some(id) if Some(id) == data.product_id => {}
                Some(id) => match catalog_item(ctx, id).await? {
                    None => validator.violation("product_id", "product was not found"),
                    Some(found) if !found.active => {
                        validator.violation("product_id", "product is inactive")
                    }
                    Some(found) if found.currency != currency => {
                        return Err(AppError::invalid_fields_code(
                            PRODUCT_CURRENCY_MISMATCH,
                            "the product is priced in another currency",
                            vec![FieldViolation::new(
                                "product_id",
                                format!(
                                    "product is priced in {} but the record uses {currency}",
                                    found.currency
                                ),
                            )],
                        ));
                    }
                    Some(found) => {
                        data.product_id = Some(found.id);
                        product = Some(found);
                    }
                },
            },
        }
    }
    if let Some(product) = &product {
        if input.name.is_none() {
            data.name = product.name.clone();
        }
        if input.unit.is_none() {
            data.unit = product.unit.clone();
        }
        if input.unit_price.is_none() {
            data.unit_price = product.price;
        }
        if input.recurrence.is_none() {
            data.recurrence = product.recurrence;
        }
        if input.term_months.is_none() {
            data.term_months = product.term_months;
        }
    }
    if let Some(value) = input.name {
        data.name = validator.required_text("name", value, MAX_ITEM_NAME_LENGTH);
    } else if data.name.trim().is_empty() {
        validator.violation("name", "is required");
    }
    if let Some(value) = input.unit {
        data.unit = validator.text("unit", value, MAX_UNIT_LENGTH);
    }
    if let Some(value) = input.quantity {
        match value.as_deref().and_then(parse_decimal).and_then(normalize_quantity) {
            Some(quantity) => data.quantity = quantity,
            None => validator.violation(
                "quantity",
                format!(
                    "must be a number greater than zero with at most {QUANTITY_DECIMALS} decimal places"
                ),
            ),
        }
    }
    if let Some(value) = input.unit_price {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.unit_price = Decimal::ZERO,
            Some(text) => match parse_money(text) {
                Some(price) => data.unit_price = price,
                None => validator.violation(
                    "unit_price",
                    format!(
                        "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                    ),
                ),
            },
        }
    }
    if let Some(value) = input.discount_type {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.discount_kind = DiscountKind::None,
            Some(text) => match DiscountKind::parse(text) {
                Some(kind) => data.discount_kind = kind,
                None => validator.violation("discount_type", "must be none, percent or amount"),
            },
        }
    }
    if let Some(value) = input.discount_value {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.discount_value = Decimal::ZERO,
            Some(text) => match parse_money(text) {
                Some(discount) => data.discount_value = discount,
                None => validator.violation(
                    "discount_value",
                    format!(
                        "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                    ),
                ),
            },
        }
    }
    if let Some(value) = input.tax_percent {
        match value.as_deref().and_then(parse_decimal).filter(|value| {
            *value >= Decimal::ZERO && *value <= Decimal::ONE_HUNDRED && value.scale() <= 4
        }) {
            Some(value) => data.tax_percent = value,
            None => validator.violation(
                "tax_percent",
                "must be between 0 and 100 with at most four decimals",
            ),
        }
    }
    if data.discount_kind == DiscountKind::None {
        data.discount_value = Decimal::ZERO;
    }
    apply_item_recurrence(
        &mut data,
        input.recurrence,
        input.term_months,
        &mut validator,
    );
    validator.finish(INVALID_LINE_ITEM)?;
    Ok(data)
}

async fn lock(
    ctx: &Ctx<'_>,
    store: &LineItemStore,
    tx: &mut MySqlTx,
    expected: &HostRecord,
) -> Result<(HostRecord, Vec<LineItem>), AppError> {
    let record = &expected.record;
    let snapshot = aphel_kernel::records::snapshot::capture(tx, ctx.pool(), ctx.registry(), record)
        .await?
        .ok_or_else(item_not_found)?;
    authorize(ctx, record, true).await?;
    let parent = host_snapshot(ctx, record, &snapshot)?;
    if parent.currency != expected.currency {
        return Err(AppError::Conflict(
            "the record currency changed; reload the items".to_owned(),
        ));
    }
    let items = store.list_locked(tx, record).await?;
    Ok((parent, items))
}
async fn project(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    stamp: &Stamp,
    items: &[LineItem],
    mode: AmountMode,
) -> Result<(), AppError> {
    if aphel_sdk::line_items::normalize_money(items_total(items)).is_none() {
        return Err(AppError::Validation(
            "the record total exceeds the monetary limit".to_owned(),
        ));
    }
    let totals = TotalsChanged {
        record: stamp.record.clone(),
        items: items.to_vec(),
        mode,
        at: stamp.now,
        ip: stamp.ip.clone(),
    };
    for projection in ctx.app().contributions::<dyn LineItemProjection>() {
        if projection.record_type() == stamp.record.record_type {
            projection.apply(ctx, tx, &totals).await?;
        }
    }
    ctx.run_hook(LineItemsChanged, tx, crate::KEY, &totals)
        .await
}

async fn record(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    stamp: &Stamp,
    event_type: &'static str,
    payload: Value,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        stamp.now,
        RecordMutation {
            record: stamp.record.clone(),
            event_type,
            payload,
            changes,
            ip: stamp.ip.clone(),
        },
    )
    .await
}

async fn current_settings(ctx: &Ctx<'_>) -> Result<LineItemSettings, AppError> {
    Ok(LineItemStore::of(ctx).settings().await?.unwrap_or_default())
}

async fn writable_mode(ctx: &Ctx<'_>, record_ref: &RecordRef) -> Result<AmountMode, AppError> {
    let mode = if has_projection(record_ref) {
        current_settings(ctx).await?.amount_mode
    } else {
        AmountMode::Manual
    };
    if mode == AmountMode::Items {
        ensure_amount_writable(ctx, record_ref).await?;
    }
    Ok(mode)
}

async fn amount_access(ctx: &Ctx<'_>, record: &RecordRef) -> Result<FieldAccess, AppError> {
    let definition = ctx.registry().require(record.record_type.as_str())?;
    let mapping = definition
        .capabilities
        .line_item_mapping()?
        .ok_or_else(item_not_found)?;
    let policy =
        FieldAccessPolicy::load(ctx.query().definitions(), ctx.scope, &record.record_type).await?;
    Ok(mapping
        .amount_field
        .map_or(FieldAccess::Editable, |field| policy.key_access(&field)))
}
async fn ensure_amount_writable(ctx: &Ctx<'_>, record: &RecordRef) -> Result<(), AppError> {
    check_access([(amount_access(ctx, record).await?, "amount".to_owned())])
}
async fn authorize(ctx: &Ctx<'_>, record: &RecordRef, write: bool) -> Result<(), AppError> {
    let definition = ctx.registry().require(record.record_type.as_str())?;
    if definition.capabilities.line_item_mapping()?.is_none() {
        return Err(item_not_found());
    }
    let permission = if write {
        definition
            .action_permission("update")
            .ok_or_else(item_not_found)?
    } else {
        definition.read_permission.clone()
    };
    let visibility = ctx.scope.record_filter(&permission)?;
    let found = ctx
        .fields()
        .references()
        .existing_records(ctx.scope, definition, &visibility, &[record.record_id])
        .await?;
    if !found.contains(&record.record_id) {
        return Err(item_not_found());
    }
    Ok(())
}
fn host_snapshot(
    ctx: &Ctx<'_>,
    record: &RecordRef,
    snapshot: &Value,
) -> Result<HostRecord, AppError> {
    let definition = ctx.registry().require(record.record_type.as_str())?;
    let mapping = definition
        .capabilities
        .line_item_mapping()?
        .ok_or_else(item_not_found)?;
    let value = |key: &str| {
        snapshot
            .get("values")
            .and_then(|values| values.get(key))
            .or_else(|| {
                snapshot
                    .get("custom_fields")
                    .and_then(|values| values.get(key))
            })
    };
    let decimal = |key: &str| {
        value(key).and_then(|value| match value {
            Value::String(text) => parse_decimal(text),
            Value::Number(number) => parse_decimal(&number.to_string()),
            _ => None,
        })
    };
    let currency = value(&mapping.currency_field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("the record must have a currency".to_owned()))?
        .to_owned();
    Ok(HostRecord {
        record: record.clone(),
        currency,
        amount: mapping.amount_field.as_deref().and_then(decimal),
        exchange_rate: mapping.exchange_rate_field.as_deref().and_then(decimal),
    })
}
pub(crate) async fn host_for(
    ctx: &Ctx<'_>,
    record: &RecordRef,
    write: bool,
) -> Result<HostRecord, AppError> {
    authorize(ctx, record, write).await?;
    let definition = ctx.registry().require(record.record_type.as_str())?;
    let mapping = definition
        .capabilities
        .line_item_mapping()?
        .ok_or_else(item_not_found)?;
    let policy =
        FieldAccessPolicy::load(ctx.query().definitions(), ctx.scope, &record.record_type).await?;
    for field in std::iter::once(&mapping.currency_field)
        .chain(mapping.amount_field.iter())
        .chain(mapping.exchange_rate_field.iter())
    {
        if policy.key_access(field) == FieldAccess::Hidden {
            check_access([(FieldAccess::Hidden, field.clone())])?;
        }
    }
    let mut tx = ctx.begin().await?;
    let snapshot =
        aphel_kernel::records::snapshot::capture(&mut tx, ctx.pool(), ctx.registry(), record)
            .await?
            .ok_or_else(item_not_found)?;
    authorize(ctx, record, write).await?;
    host_snapshot(ctx, record, &snapshot)
}

fn has_projection(record: &RecordRef) -> bool {
    aphel_kernel::modules::app()
        .contributions::<dyn LineItemProjection>()
        .iter()
        .any(|source| source.record_type() == record.record_type)
}

async fn summary_locked(
    ctx: &Ctx<'_>,
    store: &LineItemStore,
    tx: &mut MySqlTx,
    record: &RecordRef,
) -> Result<LineItemsSummary, AppError> {
    let snapshot = aphel_kernel::records::snapshot::capture(tx, ctx.pool(), ctx.registry(), record)
        .await?
        .ok_or_else(item_not_found)?;
    let parent = host_snapshot(ctx, record, &snapshot)?;
    let items = store.list_locked(tx, record).await?;
    let mode = if has_projection(record) {
        current_settings(ctx).await?.amount_mode
    } else {
        AmountMode::Manual
    };
    Ok(build_summary(&parent, items, mode))
}
