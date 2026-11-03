use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEAL_HAS_NO_ITEMS, DEAL_ITEM_ADDED, DEAL_ITEM_REMOVED, DEAL_ITEM_SETTINGS_RECORD_TYPE,
    DEAL_ITEM_SETTINGS_UPDATED, DEAL_ITEM_UPDATED, DEAL_ITEMS_LIMIT_REACHED, DEAL_ITEMS_REORDERED,
    DealItemInput, DealItemLine, DealItemSettings, DealItemsSummary, INVALID_DEAL_ITEM, LockedDeal,
    PRODUCT_CURRENCY_MISMATCH, ReorderDealItemsCommand, SaveDealItemCommand,
    UpdateDealItemSettingsCommand, item_changes, item_not_found, item_snapshot, line, line_total,
    optional_decimal, parse_decimal,
};
use crate::recurring::{apply_item_recurrence, apply_revenue};
use crate::store::DealItemStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::decimal_json;
use aphel_kernel::application::fields::permissions::{
    FieldAccess, FieldAccessPolicy, check_access,
};
use aphel_kernel::application::query::RecordQueryPorts;
use aphel_kernel::application::shared::{FieldChange, RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{InputValidator, RecordMutation, changes_payload, publish_mutation};
use aphel_sdk::money::base_amount;
use deals::domain::{
    DEAL_AMOUNT_CHANGED, DEAL_UPDATED, DealRecord, deal_not_found, revenue_changes,
};
use products::domain::{
    DealAmountMode, DealItem, DealItemData, DiscountKind, MAX_ITEM_NAME_LENGTH, MAX_ITEMS_PER_DEAL,
    MAX_PRODUCT_UNIT_LENGTH, MONEY_DECIMALS, QUANTITY_DECIMALS, items_total, normalize_quantity,
    parse_money,
};
use products::store::ProductStore;
use workflow::domain::parse_id;

struct Stamp {
    deal_id: Uuid,
    now: DateTime<Utc>,
    ip: Option<String>,
}

fn convert_item(item: &mut DealItem, rate: Option<Decimal>) {
    item.exchange_rate = rate;
    item.amount_base = base_amount(Some(item.total), rate);
}

pub async fn summary(ctx: &Ctx<'_>, deal_id: &str) -> Result<DealItemsSummary, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_READ).await?;
    let access = amount_access(ctx).await?;
    if access == FieldAccess::Hidden {
        check_access([(access, "amount".to_owned())])?;
    }
    let settings = current_settings(ctx).await?;
    let items = DealItemStore::of(ctx).list(deal.deal.id).await?;
    Ok(build_summary(&deal, items, settings.amount_mode))
}

pub async fn add(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: SaveDealItemCommand,
) -> Result<DealItemLine, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let mode = writable_mode(ctx).await?;
    let data = resolve_input(ctx, None, command.input, &deal.deal.currency).await?;
    let total = line_total(&data)?;
    let now = ctx.now();
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now,
        ip: command.ip,
    };
    let store = DealItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, mut items) = lock(&store, &mut tx, deal.deal.id).await?;
    if items.len() >= MAX_ITEMS_PER_DEAL {
        return Err(AppError::validation_code(
            DEAL_ITEMS_LIMIT_REACHED,
            format!("a deal has at most {MAX_ITEMS_PER_DEAL} items"),
        ));
    }
    let mut item = DealItem {
        id: Uuid::now_v7(),
        deal_id: deal.deal.id,
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
    convert_item(&mut item, deal.deal.revenue.exchange_rate);
    store.insert(&mut tx, &item).await?;
    let snapshot = item_snapshot(&item);
    record(
        ctx,
        &mut tx,
        &stamp,
        DEAL_ITEM_ADDED,
        json!({ "item_id": item.id.to_string(), "item": snapshot }),
        item_changes(&Value::Null, &snapshot)?,
    )
    .await?;
    items.push(item.clone());
    if mode == DealAmountMode::Items {
        sync_amount(ctx, &store, &mut tx, &stamp, &locked, &items).await?;
    }
    sync_revenue(ctx, &store, &mut tx, &stamp, &items).await?;
    tx.commit().await?;
    Ok(line(item))
}

pub(crate) async fn replace(
    ctx: &Ctx<'_>,
    deal_id: &str,
    inputs: Vec<DealItemInput>,
    ip: Option<String>,
) -> Result<DealItemsSummary, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    if inputs.len() > MAX_ITEMS_PER_DEAL {
        return Err(AppError::validation_code(
            DEAL_ITEMS_LIMIT_REACHED,
            format!("a deal has at most {MAX_ITEMS_PER_DEAL} items"),
        ));
    }
    let mode = writable_mode(ctx).await?;
    let mut resolved = Vec::with_capacity(inputs.len());
    for input in inputs {
        let data = resolve_input(ctx, None, input, &deal.deal.currency).await?;
        let total = line_total(&data)?;
        resolved.push((data, total));
    }
    let now = ctx.now();
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now,
        ip,
    };
    let store = DealItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, current) = lock(&store, &mut tx, deal.deal.id).await?;
    for item in current {
        store.delete(&mut tx, deal.deal.id, item.id).await?;
        record(
            ctx,
            &mut tx,
            &stamp,
            DEAL_ITEM_REMOVED,
            json!({ "item_id": item.id.to_string(), "name": item.data.name }),
            item_changes(&item_snapshot(&item), &Value::Null)?,
        )
        .await?;
    }
    let mut items = Vec::with_capacity(resolved.len());
    for (position, (data, total)) in (0_i32..).zip(resolved) {
        let mut item = DealItem {
            id: Uuid::now_v7(),
            deal_id: deal.deal.id,
            data,
            total,
            amount_base: None,
            exchange_rate: None,
            position,
            created_by: Some(ctx.scope.user_id().clone()),
            created_at: now,
            updated_at: now,
        };
        convert_item(&mut item, deal.deal.revenue.exchange_rate);
        store.insert(&mut tx, &item).await?;
        let snapshot = item_snapshot(&item);
        record(
            ctx,
            &mut tx,
            &stamp,
            DEAL_ITEM_ADDED,
            json!({ "item_id": item.id.to_string(), "item": snapshot }),
            item_changes(&Value::Null, &snapshot)?,
        )
        .await?;
        items.push(item);
    }
    if mode == DealAmountMode::Items {
        sync_amount(ctx, &store, &mut tx, &stamp, &locked, &items).await?;
    }
    sync_revenue(ctx, &store, &mut tx, &stamp, &items).await?;
    tx.commit().await?;
    summary(ctx, deal_id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    deal_id: &str,
    item_id: &str,
    command: SaveDealItemCommand,
) -> Result<DealItemLine, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let item_id = parse_id(item_id).ok_or_else(item_not_found)?;
    let store = DealItemStore::of(ctx);
    let current = store
        .list(deal.deal.id)
        .await?
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(item_not_found)?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mode = writable_mode(ctx).await?;
    let data = resolve_input(ctx, Some(&current.data), command.input, &deal.deal.currency).await?;
    let total = line_total(&data)?;
    if data == current.data && total == current.total {
        return Ok(line(current));
    }
    let now = ctx.now();
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now,
        ip: command.ip,
    };
    let mut tx = ctx.begin().await?;
    let (locked, mut items) = lock(&store, &mut tx, deal.deal.id).await?;
    let Some(stored) = items.iter_mut().find(|item| item.id == item_id) else {
        return Err(item_not_found());
    };
    ensure_version(Some(current.updated_at), stored.updated_at)?;
    let before = item_snapshot(stored);
    stored.data = data;
    stored.total = total;
    convert_item(stored, deal.deal.revenue.exchange_rate);
    stored.updated_at = now;
    let item = stored.clone();
    store.update(&mut tx, &item).await?;
    let changes = item_changes(&before, &item_snapshot(&item))?;
    record(
        ctx,
        &mut tx,
        &stamp,
        DEAL_ITEM_UPDATED,
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
    if mode == DealAmountMode::Items {
        sync_amount(ctx, &store, &mut tx, &stamp, &locked, &items).await?;
    }
    sync_revenue(ctx, &store, &mut tx, &stamp, &items).await?;
    tx.commit().await?;
    Ok(line(item))
}

pub(crate) async fn remove(
    ctx: &Ctx<'_>,
    deal_id: &str,
    item_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let item_id = parse_id(item_id).ok_or_else(item_not_found)?;
    let mode = writable_mode(ctx).await?;
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now: ctx.now(),
        ip,
    };
    let store = DealItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, mut items) = lock(&store, &mut tx, deal.deal.id).await?;
    let Some(index) = items.iter().position(|item| item.id == item_id) else {
        return Err(item_not_found());
    };
    let item = items.remove(index);
    store.delete(&mut tx, deal.deal.id, item.id).await?;
    record(
        ctx,
        &mut tx,
        &stamp,
        DEAL_ITEM_REMOVED,
        json!({ "item_id": item.id.to_string(), "name": item.data.name }),
        item_changes(&item_snapshot(&item), &Value::Null)?,
    )
    .await?;
    if mode == DealAmountMode::Items {
        sync_amount(ctx, &store, &mut tx, &stamp, &locked, &items).await?;
    }
    sync_revenue(ctx, &store, &mut tx, &stamp, &items).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn reorder(
    ctx: &Ctx<'_>,
    deal_id: &str,
    command: ReorderDealItemsCommand,
) -> Result<DealItemsSummary, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    let requested: Vec<Option<Uuid>> = command.item_ids.iter().map(|id| parse_id(id)).collect();
    let now = ctx.now();
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now,
        ip: command.ip,
    };
    let store = DealItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (_, items) = lock(&store, &mut tx, deal.deal.id).await?;
    let existing: HashSet<Uuid> = items.iter().map(|item| item.id).collect();
    let order: Vec<Uuid> = requested.iter().flatten().copied().collect();
    let unique: HashSet<Uuid> = order.iter().copied().collect();
    if order.len() != requested.len() || unique.len() != order.len() || unique != existing {
        return Err(AppError::invalid_fields(
            INVALID_DEAL_ITEM,
            vec![FieldViolation::new(
                "item_ids",
                "must list every item of the deal exactly once",
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
            .set_positions(&mut tx, deal.deal.id, &positions, now)
            .await?;
        record(
            ctx,
            &mut tx,
            &stamp,
            DEAL_ITEMS_REORDERED,
            json!({ "item_ids": after }),
            vec![FieldChange::new("item_order", json!(before), json!(after))],
        )
        .await?;
        tx.commit().await?;
    }
    summary(ctx, deal_id).await
}

pub(crate) async fn apply_total(
    ctx: &Ctx<'_>,
    deal_id: &str,
    ip: Option<String>,
) -> Result<DealItemsSummary, AppError> {
    let deal = deal_for(ctx, deal_id, &Permission::DEALS_UPDATE).await?;
    ensure_amount_writable(ctx).await?;
    let stamp = Stamp {
        deal_id: deal.deal.id,
        now: ctx.now(),
        ip,
    };
    let store = DealItemStore::of(ctx);
    let mut tx = ctx.begin().await?;
    let (locked, items) = lock(&store, &mut tx, deal.deal.id).await?;
    if items.is_empty() {
        return Err(AppError::validation_code(
            DEAL_HAS_NO_ITEMS,
            "the deal has no items to sum",
        ));
    }
    if sync_amount(ctx, &store, &mut tx, &stamp, &locked, &items).await? {
        tx.commit().await?;
    }
    summary(ctx, deal_id).await
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<DealItemSettings, AppError> {
    if ctx.scope.require(&Permission::SETTINGS_MANAGE).is_err() {
        ctx.scope.require(&Permission::DEALS_READ)?;
    }
    current_settings(ctx).await
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    command: UpdateDealItemSettingsCommand,
) -> Result<DealItemSettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = current_settings(ctx).await?;
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
    let mut tx = ctx.begin().await?;
    DealItemStore::of(ctx)
        .save_settings(&mut tx, scope.user_id(), &settings, now)
        .await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(DEAL_ITEM_SETTINGS_RECORD_TYPE, Uuid::nil()),
            event_type: DEAL_ITEM_SETTINGS_UPDATED,
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

fn build_summary(
    deal: &DealRecord,
    items: Vec<DealItem>,
    mode: DealAmountMode,
) -> DealItemsSummary {
    let total = items_total(&items);
    let lines: Vec<DealItemLine> = items.into_iter().map(line).collect();
    DealItemsSummary {
        deal_id: deal.deal.id,
        currency: deal.deal.currency.clone(),
        subtotal: lines.iter().map(|line| line.gross).sum(),
        discount_total: lines.iter().map(|line| line.discount).sum(),
        total,
        deal_amount: deal.deal.amount,
        amount_mode: mode,
        amount_managed: mode == DealAmountMode::Items && !lines.is_empty(),
        items: lines,
    }
}

async fn resolve_input(
    ctx: &Ctx<'_>,
    base: Option<&DealItemData>,
    input: DealItemInput,
    currency: &str,
) -> Result<DealItemData, AppError> {
    let mut validator = InputValidator::new();
    let mut data = base.cloned().unwrap_or_else(|| DealItemData {
        quantity: Decimal::ONE,
        ..DealItemData::default()
    });
    let mut product = None;
    if let Some(value) = &input.product_id {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.product_id = None,
            Some(text) => match parse_id(text) {
                None => validator.violation("product_id", "must be a valid id"),
                Some(id) if Some(id) == data.product_id => {}
                Some(id) => match ProductStore::of(ctx).find(id).await? {
                    None => validator.violation("product_id", "product was not found"),
                    Some(found) if !found.data.is_active => {
                        validator.violation("product_id", "product is inactive")
                    }
                    Some(found) if found.data.currency != currency => {
                        return Err(AppError::invalid_fields_code(
                            PRODUCT_CURRENCY_MISMATCH,
                            "the product is priced in another currency",
                            vec![FieldViolation::new(
                                "product_id",
                                format!(
                                    "product is priced in {} but the deal uses {currency}",
                                    found.data.currency
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
            data.name = product.data.name.clone();
        }
        if input.unit.is_none() {
            data.unit = product.data.unit.clone();
        }
        if input.unit_price.is_none() {
            data.unit_price = product.data.price;
        }
        if input.recurrence.is_none() {
            data.recurrence = product.data.recurrence;
        }
        if input.term_months.is_none() {
            data.term_months = product.data.term_months;
        }
    }
    if let Some(value) = input.name {
        data.name = validator.required_text("name", value, MAX_ITEM_NAME_LENGTH);
    } else if data.name.trim().is_empty() {
        validator.violation("name", "is required");
    }
    if let Some(value) = input.unit {
        data.unit = validator.text("unit", value, MAX_PRODUCT_UNIT_LENGTH);
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
    if data.discount_kind == DiscountKind::None {
        data.discount_value = Decimal::ZERO;
    }
    apply_item_recurrence(
        &mut data,
        input.recurrence,
        input.term_months,
        &mut validator,
    );
    validator.finish(INVALID_DEAL_ITEM)?;
    Ok(data)
}

async fn lock(
    store: &DealItemStore,
    tx: &mut MySqlTx,
    deal_id: Uuid,
) -> Result<(LockedDeal, Vec<DealItem>), AppError> {
    let locked = store
        .lock_deal(tx, deal_id)
        .await?
        .ok_or_else(deal_not_found)?;
    let items = store.list_locked(tx, deal_id).await?;
    Ok((locked, items))
}

async fn sync_amount(
    ctx: &Ctx<'_>,
    store: &DealItemStore,
    tx: &mut MySqlTx,
    stamp: &Stamp,
    locked: &LockedDeal,
    items: &[DealItem],
) -> Result<bool, AppError> {
    let total = items_total(items);
    if locked.amount == Some(total) {
        return Ok(false);
    }
    store
        .set_deal_amount(tx, stamp.deal_id, total, stamp.now)
        .await?;
    let old = optional_decimal(locked.amount);
    let new = decimal_json(total);
    record(
        ctx,
        tx,
        stamp,
        DEAL_AMOUNT_CHANGED,
        json!({
            "old": old,
            "new": new,
            "currency": locked.currency,
            "source": "items",
        }),
        vec![FieldChange::new("amount", old.clone(), new.clone())],
    )
    .await?;
    Ok(true)
}

async fn sync_revenue(
    ctx: &Ctx<'_>,
    store: &DealItemStore,
    tx: &mut MySqlTx,
    stamp: &Stamp,
    items: &[DealItem],
) -> Result<bool, AppError> {
    let Some(state) = store.lock_revenue(tx, stamp.deal_id).await? else {
        return Ok(false);
    };
    let mut revenue = state.revenue.clone();
    apply_revenue(&mut revenue, items, state.won_on);
    let changes = revenue_changes(&state.revenue, &revenue);
    if changes.is_empty() {
        return Ok(false);
    }
    store
        .save_revenue(tx, stamp.deal_id, &revenue, stamp.now)
        .await?;
    let mut payload = changes_payload(&changes);
    payload["source"] = json!("items");
    record(ctx, tx, stamp, DEAL_UPDATED, payload, changes).await?;
    Ok(true)
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
            record: RecordRef::new(RecordType::DEAL, stamp.deal_id),
            event_type,
            payload,
            changes,
            ip: stamp.ip.clone(),
        },
    )
    .await
}

async fn current_settings(ctx: &Ctx<'_>) -> Result<DealItemSettings, AppError> {
    Ok(DealItemStore::of(ctx).settings().await?.unwrap_or_default())
}

async fn writable_mode(ctx: &Ctx<'_>) -> Result<DealAmountMode, AppError> {
    let mode = current_settings(ctx).await?.amount_mode;
    if mode == DealAmountMode::Items {
        ensure_amount_writable(ctx).await?;
    }
    Ok(mode)
}

async fn amount_access(ctx: &Ctx<'_>) -> Result<FieldAccess, AppError> {
    Ok(
        FieldAccessPolicy::load(ctx.query().definitions(), ctx.scope, &RecordType::DEAL)
            .await?
            .key_access("amount"),
    )
}

async fn ensure_amount_writable(ctx: &Ctx<'_>) -> Result<(), AppError> {
    let access = amount_access(ctx).await?;
    check_access([(access, "amount".to_owned())])
}

async fn deal_for(
    ctx: &Ctx<'_>,
    deal_id: &str,
    permission: &Permission,
) -> Result<DealRecord, AppError> {
    let visibility = ctx.scope.record_filter(permission)?;
    deals::service::require(ctx, &visibility, deal_id).await
}
