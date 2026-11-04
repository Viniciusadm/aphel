use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DISCOUNT_LIMITS_UPDATED, DISCOUNT_PERCENT_DECIMALS, MAX_PRICE_LIST_COMPANIES,
    MAX_PRICE_LIST_DESCRIPTION_LENGTH, MAX_PRICE_LIST_ITEMS, MAX_PRICE_LIST_NAME_LENGTH,
    MAX_PRICE_LISTS, NewPriceListItem, PRICE_LIST_CREATED, PRICE_LIST_DELETED,
    PRICE_LIST_ITEMS_REPLACED, PRICE_LIST_RECORD_TYPE, PRICE_LIST_UPDATED, PriceList,
    PriceListAppliesTo, PriceListFilter, PriceListItem, ReplacePriceListItemsCommand,
    RoleDiscountLimit, SavePriceListCommand, SetDiscountLimitsCommand, price_list_key_taken,
    price_list_not_found,
};
use crate::store::{DiscountLimitStore, PriceListStore};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_changes};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{InputValidator, MAX_LONG_TEXT_LENGTH, Patch, changes_payload};
use aphel_sdk::access::require_organization_record_scope as require_catalog;
use aphel_sdk::configuration::{is_config_key, key_from_name};
use aphel_sdk::line_items::{MONEY_DECIMALS, QUANTITY_DECIMALS, normalize_quantity, parse_money};
use aphel_sdk::money::normalize_currency_code as normalize_currency;
const INVALID_PRICE_LIST: &str = "invalid price list";
const INVALID_ITEMS: &str = "invalid price list items";
const INVALID_LIMITS: &str = "invalid discount limits";

#[derive(Debug, Serialize)]
struct PriceListSnapshot<'a> {
    key: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    currency: &'a str,
    applies_to: &'static str,
    segment_id: Option<String>,
    company_ids: Vec<String>,
    valid_from: Option<String>,
    valid_to: Option<String>,
    is_active: bool,
    is_default: bool,
}

impl<'a> From<&'a PriceList> for PriceListSnapshot<'a> {
    fn from(list: &'a PriceList) -> Self {
        let mut company_ids: Vec<String> = list.company_ids.iter().map(Uuid::to_string).collect();
        company_ids.sort();
        Self {
            key: &list.key,
            name: &list.name,
            description: list.description.as_deref(),
            currency: &list.currency,
            applies_to: list.applies_to.as_str(),
            segment_id: list.segment_id.map(|id| id.to_string()),
            company_ids,
            valid_from: list.valid_from.map(date_text),
            valid_to: list.valid_to.map(date_text),
            is_active: list.is_active,
            is_default: list.is_default,
        }
    }
}

fn date_text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_date(
    field: &str,
    value: Option<String>,
    validator: &mut InputValidator,
) -> Option<NaiveDate> {
    let text = value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())?;
    match NaiveDate::parse_from_str(&text, "%Y-%m-%d") {
        Ok(date) => Some(date),
        Err(_) => {
            validator.violation(field, "must be a date in the YYYY-MM-DD format");
            None
        }
    }
}

pub(crate) fn parse_percent(text: &str) -> Option<Decimal> {
    let text = text.trim();
    let value = Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())?;
    let rounded = value.round_dp_with_strategy(
        DISCOUNT_PERCENT_DECIMALS,
        RoundingStrategy::MidpointAwayFromZero,
    );
    (rounded == value && rounded >= Decimal::ZERO && rounded <= Decimal::ONE_HUNDRED)
        .then(|| rounded.normalize())
}

pub(crate) fn parse_min_quantity(value: Option<&str>) -> Option<Decimal> {
    match value.map(str::trim) {
        None | Some("") => Some(Decimal::ONE),
        Some(text) => Decimal::from_str(text)
            .ok()
            .or_else(|| Decimal::from_scientific(text).ok())
            .and_then(normalize_quantity),
    }
}

fn item_entry(product_id: Uuid, min_quantity: Decimal, unit_price: Decimal) -> String {
    format!(
        "{product_id}:{}@{}",
        min_quantity.normalize(),
        unit_price.normalize()
    )
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    filter: PriceListFilter,
    window: ListWindow,
) -> Result<Listing<PriceList>, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_READ)?;
    let mut filter = filter;
    filter.currency = filter
        .currency
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| text.to_ascii_uppercase());
    filter.search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned);
    PriceListStore::of(ctx).list(&filter, window).await
}

pub(crate) async fn get(ctx: &Ctx<'_>, id: &str) -> Result<PriceList, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_READ)?;
    existing(ctx, id).await
}

pub(crate) async fn items(ctx: &Ctx<'_>, id: &str) -> Result<Vec<PriceListItem>, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_READ)?;
    let list = existing(ctx, id).await?;
    PriceListStore::of(ctx).items(list.id).await
}

pub async fn create(ctx: &Ctx<'_>, command: SavePriceListCommand) -> Result<PriceList, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let repository = PriceListStore::of(ctx);
    let currency = repository.organization_currency().await?;
    let now = ctx.now();
    let base = PriceList {
        id: Uuid::now_v7(),
        key: String::new(),
        name: String::new(),
        description: None,
        currency,
        applies_to: PriceListAppliesTo::All,
        segment_id: None,
        company_ids: Vec::new(),
        valid_from: None,
        valid_to: None,
        is_active: true,
        is_default: false,
        item_count: 0,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let mut validator = InputValidator::new();
    if repository.count().await? >= MAX_PRICE_LISTS {
        validator.violation(
            "name",
            format!("an organization can have at most {MAX_PRICE_LISTS} price lists"),
        );
    }
    let ip = command.ip.clone();
    let mut command = command;
    command.name.get_or_insert_default();
    let mut list = apply(ctx, &base, command, &mut validator).await?;
    if list.key.is_empty() {
        list.key = generated_key(ctx, &list.name).await?;
    }
    validator.finish(INVALID_PRICE_LIST)?;
    let changes = diff_changes(
        &Value::Null,
        &PriceListSnapshot::from(&list),
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    if list.is_default {
        repository
            .clear_default(&mut tx, &list.currency, list.id)
            .await?;
    }
    repository.insert(&mut tx, &list).await?;
    repository
        .set_companies(&mut tx, list.id, &list.company_ids)
        .await?;
    publish(ctx, &mut tx, &list, PRICE_LIST_CREATED, changes, ip).await?;
    tx.commit().await?;
    reload(ctx, list.id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    id: &str,
    command: SavePriceListCommand,
) -> Result<PriceList, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let current = existing(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut validator = InputValidator::new();
    let ip = command.ip.clone();
    let mut next = apply(ctx, &current, command, &mut validator).await?;
    if next.key.is_empty() {
        next.key = current.key.clone();
    }
    validator.finish(INVALID_PRICE_LIST)?;
    let changes = diff_changes(
        &PriceListSnapshot::from(&current),
        &PriceListSnapshot::from(&next),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    next.updated_at = ctx.now();
    let repository = PriceListStore::of(ctx);
    let mut tx = ctx.begin().await?;
    if next.is_default {
        repository
            .clear_default(&mut tx, &next.currency, next.id)
            .await?;
    }
    repository.update(&mut tx, &next).await?;
    repository
        .set_companies(&mut tx, next.id, &next.company_ids)
        .await?;
    publish(ctx, &mut tx, &next, PRICE_LIST_UPDATED, changes, ip).await?;
    tx.commit().await?;
    reload(ctx, next.id).await
}

pub(crate) async fn delete(ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let current = existing(ctx, id).await?;
    let changes = diff_changes(
        &PriceListSnapshot::from(&current),
        &Value::Null,
        &DiffOptions::default(),
    )?;
    let mut tx = ctx.begin().await?;
    PriceListStore::of(ctx).delete(&mut tx, current.id).await?;
    publish(ctx, &mut tx, &current, PRICE_LIST_DELETED, changes, ip).await?;
    Ok(tx.commit().await?)
}

pub async fn replace_items(
    ctx: &Ctx<'_>,
    id: &str,
    command: ReplacePriceListItemsCommand,
) -> Result<Vec<PriceListItem>, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let list = existing(ctx, id).await?;
    ensure_version(command.expected_updated_at, list.updated_at)?;
    let repository = PriceListStore::of(ctx);
    let mut violations = Vec::new();
    if command.items.len() > MAX_PRICE_LIST_ITEMS {
        violations.push(FieldViolation::new(
            "items",
            format!("a price list can have at most {MAX_PRICE_LIST_ITEMS} items"),
        ));
    }
    let mut parsed = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, input) in command.items.iter().enumerate() {
        let product_id = Uuid::parse_str(input.product_id.trim()).ok();
        if product_id.is_none() {
            violations.push(FieldViolation::new(
                format!("items.{index}.product_id"),
                "must be a product id",
            ));
        }
        let unit_price = parse_money(&input.unit_price);
        if unit_price.is_none() {
            violations.push(FieldViolation::new(
                format!("items.{index}.unit_price"),
                format!(
                    "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                ),
            ));
        }
        let min_quantity = parse_min_quantity(input.min_quantity.as_deref());
        if min_quantity.is_none() {
            violations.push(FieldViolation::new(
                format!("items.{index}.min_quantity"),
                format!(
                    "must be a number greater than zero with at most {QUANTITY_DECIMALS} decimal places"
                ),
            ));
        }
        if let (Some(product_id), Some(unit_price), Some(min_quantity)) =
            (product_id, unit_price, min_quantity)
        {
            if !seen.insert((product_id, min_quantity.normalize().to_string())) {
                violations.push(FieldViolation::new(
                    format!("items.{index}.min_quantity"),
                    "the product already has a price for this minimum quantity",
                ));
            }
            parsed.push((index, product_id, min_quantity, unit_price));
        }
    }
    let product_ids: Vec<Uuid> = parsed
        .iter()
        .map(|(_, product_id, _, _)| *product_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let existing: BTreeSet<Uuid> = repository
        .existing_products(&product_ids)
        .await?
        .into_iter()
        .collect();
    for (index, product_id, _, _) in &parsed {
        if !existing.contains(product_id) {
            violations.push(FieldViolation::new(
                format!("items.{index}.product_id"),
                "is not a product of the organization",
            ));
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_ITEMS, violations));
    }
    let items: Vec<NewPriceListItem> = parsed
        .into_iter()
        .map(
            |(index, product_id, min_quantity, unit_price)| NewPriceListItem {
                id: Uuid::now_v7(),
                product_id,
                min_quantity,
                unit_price,
                position: index as i32,
            },
        )
        .collect();
    let current = repository.items(list.id).await?;
    let before: BTreeSet<String> = current
        .iter()
        .map(|item| item_entry(item.product_id, item.min_quantity, item.unit_price))
        .collect();
    let after: BTreeSet<String> = items
        .iter()
        .map(|item| item_entry(item.product_id, item.min_quantity, item.unit_price))
        .collect();
    if before == after {
        return Ok(current);
    }
    let removed: Vec<&String> = before.difference(&after).collect();
    let added: Vec<&String> = after.difference(&before).collect();
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(PRICE_LIST_RECORD_TYPE, list.id);
    let mut tx = ctx.begin().await?;
    repository.replace_items(&mut tx, list.id, &items).await?;
    repository.touch(&mut tx, list.id, now).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), PRICE_LIST_ITEMS_REPLACED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "key": list.key,
                    "name": list.name,
                    "currency": list.currency,
                    "items": items.len(),
                    "added": added.len(),
                    "removed": removed.len(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, PRICE_LIST_ITEMS_REPLACED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "items",
                    json!(removed),
                    json!(added),
                )])
                .with_ip(command.ip),
        )
        .await?;
    tx.commit().await?;
    repository.items(list.id).await
}

pub async fn discount_limits(ctx: &Ctx<'_>) -> Result<Vec<RoleDiscountLimit>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    DiscountLimitStore::of(ctx).list().await
}

pub(crate) async fn set_discount_limits(
    ctx: &Ctx<'_>,
    command: SetDiscountLimitsCommand,
) -> Result<Vec<RoleDiscountLimit>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = DiscountLimitStore::of(ctx).list().await?;
    let mut violations = Vec::new();
    let mut requested: Vec<(Uuid, Option<Decimal>)> = Vec::new();
    for (index, input) in command.limits.iter().enumerate() {
        let role = Uuid::parse_str(input.role_id.trim())
            .ok()
            .filter(|id| current.iter().any(|limit| limit.role_id == *id));
        let Some(role_id) = role else {
            violations.push(FieldViolation::new(
                format!("limits.{index}.role_id"),
                "is not a role of the organization",
            ));
            continue;
        };
        if requested.iter().any(|(id, _)| *id == role_id) {
            violations.push(FieldViolation::new(
                format!("limits.{index}.role_id"),
                "is declared more than once",
            ));
            continue;
        }
        let percent = match input.max_discount_percent.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(text) => match parse_percent(text) {
                Some(percent) => Some(percent),
                None => {
                    violations.push(FieldViolation::new(
                        format!("limits.{index}.max_discount_percent"),
                        format!(
                            "must be a number between 0 and 100 with at most {DISCOUNT_PERCENT_DECIMALS} decimal places"
                        ),
                    ));
                    continue;
                }
            },
        };
        requested.push((role_id, percent));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_LIMITS, violations));
    }
    let mut changes = Vec::new();
    let mut updates = Vec::new();
    for role in &current {
        let next = requested
            .iter()
            .find(|(id, _)| *id == role.role_id)
            .and_then(|(_, percent)| *percent);
        let previous = role.max_discount_percent.map(|value| value.normalize());
        if previous != next {
            changes.push(FieldChange::new(
                role.role_key.clone(),
                json!(previous.map(|value| value.to_string())),
                json!(next.map(|value| value.to_string())),
            ));
            updates.push((role.role_id, next));
        }
    }
    if updates.is_empty() {
        return Ok(current);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    for (role_id, percent) in &updates {
        DiscountLimitStore::of(ctx)
            .set(&mut tx, *role_id, *percent, Some(scope.user_id()), now)
            .await?;
    }
    let payload: Vec<Value> = changes
        .iter()
        .map(|change| {
            json!({
                "role": change.field,
                "old": change.old,
                "new": change.new,
            })
        })
        .collect();
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), DISCOUNT_LIMITS_UPDATED, now)
                .with_payload(json!({ "changes": payload })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, DISCOUNT_LIMITS_UPDATED, now)
                .with_changes(changes)
                .with_ip(command.ip),
        )
        .await?;
    tx.commit().await?;
    DiscountLimitStore::of(ctx).list().await
}

async fn apply(
    ctx: &Ctx<'_>,
    base: &PriceList,
    command: SavePriceListCommand,
    validator: &mut InputValidator,
) -> Result<PriceList, AppError> {
    let repository = PriceListStore::of(ctx);
    let mut list = base.clone();
    if let Some(name) = command.name {
        list.name = validator.required_text("name", Some(name), MAX_PRICE_LIST_NAME_LENGTH);
    }
    if let Some(description) = command.description {
        list.description = validator.text(
            "description",
            description,
            MAX_PRICE_LIST_DESCRIPTION_LENGTH.min(MAX_LONG_TEXT_LENGTH),
        );
    }
    if let Some(key) = command
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        if !is_config_key(key) {
            validator.violation(
                "key",
                "must start with a lowercase letter and use only lowercase letters, digits and underscores",
            );
        } else if key != base.key {
            if repository.find_by_key(key).await?.is_some() {
                return Err(price_list_key_taken());
            }
            list.key = key.to_owned();
        }
    }
    if let Some(currency) = command.currency {
        match normalize_currency(&currency) {
            None => validator.violation("currency", "must be an ISO 4217 code with three letters"),
            Some(code) if !repository.currency_enabled(&code).await? => {
                validator.violation(
                    "currency",
                    format!("{code} is not one of the enabled currencies"),
                );
            }
            Some(code) => list.currency = code,
        }
    }
    if let Some(applies_to) = command.applies_to {
        match PriceListAppliesTo::parse(applies_to.trim()) {
            Some(value) => list.applies_to = value,
            None => validator.violation("applies_to", "must be all, segment or organizations"),
        }
    }
    apply_segment(ctx, &mut list, command.segment_id, validator).await?;
    apply_companies(ctx, &mut list, command.company_ids, validator).await?;
    if let Some(value) = command.valid_from {
        list.valid_from = parse_date("valid_from", value, validator);
    }
    if let Some(value) = command.valid_to {
        list.valid_to = parse_date("valid_to", value, validator);
    }
    if let (Some(from), Some(to)) = (list.valid_from, list.valid_to)
        && from > to
    {
        validator.violation("valid_to", "must be on or after valid_from");
    }
    if let Some(active) = command.is_active {
        list.is_active = active;
    }
    if let Some(default) = command.is_default {
        list.is_default = default;
    }
    if list.is_default && list.applies_to != PriceListAppliesTo::All {
        validator.violation(
            "is_default",
            "only a price list that applies to all organizations can be the default of its currency",
        );
    }
    Ok(list)
}

async fn apply_segment(
    ctx: &Ctx<'_>,
    list: &mut PriceList,
    segment: Patch<String>,
    validator: &mut InputValidator,
) -> Result<(), AppError> {
    if let Some(value) = segment {
        list.segment_id = None;
        if let Some(segment_id) = validator.record_id("segment_id", value) {
            match aphel_sdk::segments::target(ctx, segment_id).await? {
                Some(record_type) if record_type == RecordType::COMPANY => {
                    list.segment_id = Some(segment_id);
                }
                Some(_) => validator.violation("segment_id", "must be a segment of organizations"),
                None => validator.violation("segment_id", "is not a segment of the organization"),
            }
        }
    }
    if list.applies_to == PriceListAppliesTo::Segment {
        if list.segment_id.is_none() {
            validator.violation(
                "segment_id",
                "is required when the price list applies to a segment",
            );
        }
    } else {
        list.segment_id = None;
    }
    Ok(())
}

async fn apply_companies(
    ctx: &Ctx<'_>,
    list: &mut PriceList,
    organizations: Option<Vec<String>>,
    validator: &mut InputValidator,
) -> Result<(), AppError> {
    if let Some(values) = organizations {
        let mut ids = Vec::new();
        for (index, value) in values.iter().enumerate() {
            match Uuid::parse_str(value.trim()) {
                Ok(id) if !ids.contains(&id) => ids.push(id),
                Ok(_) => {}
                Err(_) => {
                    validator.violation(&format!("company_ids.{index}"), "must be a company id")
                }
            }
        }
        if ids.len() > MAX_PRICE_LIST_COMPANIES {
            validator.violation(
                "company_ids",
                format!("a price list can name at most {MAX_PRICE_LIST_COMPANIES} organizations"),
            );
        }
        let existing: BTreeSet<Uuid> = PriceListStore::of(ctx)
            .existing_companies(&ids)
            .await?
            .into_iter()
            .collect();
        if ids.iter().any(|id| !existing.contains(id)) {
            validator.violation(
                "company_ids",
                "must only list organizations of the organization",
            );
        }
        list.company_ids = ids;
    }
    if list.applies_to == PriceListAppliesTo::Organizations {
        if list.company_ids.is_empty() {
            validator.violation(
                "company_ids",
                "must list at least one company when the price list applies to organizations",
            );
        }
    } else {
        list.company_ids.clear();
    }
    Ok(())
}

async fn generated_key(ctx: &Ctx<'_>, name: &str) -> Result<String, AppError> {
    let base = key_from_name(name);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while PriceListStore::of(ctx)
        .find_by_key(&candidate)
        .await?
        .is_some()
    {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

async fn existing(ctx: &Ctx<'_>, id: &str) -> Result<PriceList, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| price_list_not_found())?;
    PriceListStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(price_list_not_found)
}

async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<PriceList, AppError> {
    PriceListStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(price_list_not_found)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    list: &PriceList,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(PRICE_LIST_RECORD_TYPE, list.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "key": list.key,
                    "name": list.name,
                    "currency": list.currency,
                    "applies_to": list.applies_to.as_str(),
                    "is_active": list.is_active,
                    "is_default": list.is_default,
                    "changes": changes_payload(&changes)["changes"].clone(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}
