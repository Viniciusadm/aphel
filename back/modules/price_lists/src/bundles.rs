use std::collections::BTreeSet;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    BundleDetails, BundleItem, BundlePricing, MAX_BUNDLE_COMPONENTS, PRODUCT_BUNDLE_REMOVED,
    PRODUCT_BUNDLE_UPDATED, ProductBundle, SaveBundleCommand, price_list_not_found,
    product_bundle_nested, product_bundle_not_found,
};
use crate::hooks::{BundleExpander, CatalogBundleExpander};
use crate::store::BundleStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use products::domain::{QUANTITY_DECIMALS, normalize_quantity, product_not_found, require_catalog};

const INVALID_BUNDLE: &str = "invalid product bundle";

fn components_snapshot(bundle: Option<&ProductBundle>) -> (Value, Value) {
    match bundle {
        None => (Value::Null, Value::Null),
        Some(bundle) => (
            json!(bundle.pricing.as_str()),
            json!(
                bundle
                    .components
                    .iter()
                    .map(|item| format!("{}x{}", item.product_id, item.quantity.normalize()))
                    .collect::<Vec<_>>()
            ),
        ),
    }
}

fn bundle_changes(old: Option<&ProductBundle>, new: Option<&ProductBundle>) -> Vec<FieldChange> {
    let (old_pricing, old_components) = components_snapshot(old);
    let (new_pricing, new_components) = components_snapshot(new);
    let mut changes = Vec::new();
    if old_pricing != new_pricing {
        changes.push(FieldChange::new("bundle_pricing", old_pricing, new_pricing));
    }
    if old_components != new_components {
        changes.push(FieldChange::new(
            "bundle_components",
            old_components,
            new_components,
        ));
    }
    changes
}

struct BundleMutation {
    event_type: &'static str,
    payload: Value,
    changes: Vec<FieldChange>,
    ip: Option<String>,
}

pub(crate) async fn get_bundle(
    ctx: &Ctx<'_>,
    product_id: &str,
    price_list_id: Option<&str>,
) -> Result<Option<BundleDetails>, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_READ)?;
    let product_id = existing_product(ctx, product_id).await?;
    let price_list_id = match price_list_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(text) => Some(Uuid::parse_str(text).map_err(|_| price_list_not_found())?),
        None => None,
    };
    details(ctx, product_id, price_list_id).await
}

pub(crate) async fn save_bundle(
    ctx: &Ctx<'_>,
    product_id: &str,
    command: SaveBundleCommand,
) -> Result<BundleDetails, AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let product_id = existing_product(ctx, product_id).await?;
    let repository = BundleStore::of(ctx);
    let current = repository.find(product_id).await?;
    if let Some(current) = &current {
        ensure_version(command.expected_updated_at, current.updated_at)?;
    }
    let mut violations = Vec::new();
    let pricing = command.pricing;
    if command.components.is_empty() {
        violations.push(FieldViolation::new(
            "components",
            "a bundle needs at least one component",
        ));
    }
    if command.components.len() > MAX_BUNDLE_COMPONENTS {
        violations.push(FieldViolation::new(
            "components",
            format!("a bundle can have at most {MAX_BUNDLE_COMPONENTS} components"),
        ));
    }
    let mut items: Vec<(usize, BundleItem)> = Vec::new();
    for (index, input) in command.components.iter().enumerate() {
        let component = Uuid::parse_str(input.product_id.trim()).ok();
        let quantity = Decimal::from_str(input.quantity.trim())
            .ok()
            .and_then(normalize_quantity);
        match component {
            None => violations.push(FieldViolation::new(
                format!("components.{index}.product_id"),
                "must be a product id",
            )),
            Some(id) if id == product_id => violations.push(FieldViolation::new(
                format!("components.{index}.product_id"),
                "a bundle cannot contain itself",
            )),
            Some(id) if items.iter().any(|(_, item)| item.product_id == id) => {
                violations.push(FieldViolation::new(
                    format!("components.{index}.product_id"),
                    "is declared more than once",
                ))
            }
            Some(_) => {}
        }
        if quantity.is_none() {
            violations.push(FieldViolation::new(
                format!("components.{index}.quantity"),
                format!(
                    "must be a number greater than zero with at most {QUANTITY_DECIMALS} decimal places"
                ),
            ));
        }
        if let (Some(id), Some(quantity)) = (component, quantity)
            && id != product_id
            && !items.iter().any(|(_, item)| item.product_id == id)
        {
            items.push((
                index,
                BundleItem {
                    product_id: id,
                    quantity,
                    position: index as i32,
                },
            ));
        }
    }
    let mut ids = vec![product_id];
    ids.extend(items.iter().map(|(_, item)| item.product_id));
    let products = repository.products(&ids).await?;
    let bundled: BTreeSet<Uuid> = repository.bundled(&ids[1..]).await?.into_iter().collect();
    let kit = products
        .iter()
        .find(|product| product.id == product_id)
        .ok_or_else(product_not_found)?;
    for (index, item) in &items {
        match products
            .iter()
            .find(|product| product.id == item.product_id)
        {
            None => violations.push(FieldViolation::new(
                format!("components.{index}.product_id"),
                "is not a product of the organization",
            )),
            Some(_) if bundled.contains(&item.product_id) => violations.push(FieldViolation::new(
                format!("components.{index}.product_id"),
                "is a bundle; bundles cannot contain other bundles",
            )),
            Some(component)
                if pricing == BundlePricing::Sum && component.currency != kit.currency =>
            {
                violations.push(FieldViolation::new(
                    format!("components.{index}.product_id"),
                    format!(
                        "must be priced in {} to be summed into the bundle price",
                        kit.currency
                    ),
                ))
            }
            Some(_) => {}
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_BUNDLE, violations));
    }
    if current.is_none() && repository.is_component(product_id).await? {
        return Err(product_bundle_nested());
    }
    let now = ctx.now();
    let bundle = ProductBundle {
        product_id,
        pricing,
        components: items.into_iter().map(|(_, item)| item).collect(),
        updated_at: now,
    };
    let changes = bundle_changes(current.as_ref(), Some(&bundle));
    if changes.is_empty() {
        return details(ctx, product_id, None)
            .await?
            .ok_or_else(product_bundle_not_found);
    }
    let mut tx = ctx.begin().await?;
    repository
        .save(&mut tx, &bundle, Some(scope.user_id()))
        .await?;
    publish(
        ctx,
        &mut tx,
        product_id,
        BundleMutation {
            event_type: PRODUCT_BUNDLE_UPDATED,
            payload: json!({
                "name": kit.name,
                "pricing": bundle.pricing.as_str(),
                "components": bundle.components.len(),
            }),
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    details(ctx, product_id, None)
        .await?
        .ok_or_else(product_bundle_not_found)
}

pub(crate) async fn remove_bundle(
    ctx: &Ctx<'_>,
    product_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    require_catalog(scope, &Permission::PRODUCTS_UPDATE)?;
    let product_id = existing_product(ctx, product_id).await?;
    let repository = BundleStore::of(ctx);
    let current = repository
        .find(product_id)
        .await?
        .ok_or_else(product_bundle_not_found)?;
    let mut tx = ctx.begin().await?;
    repository.remove(&mut tx, product_id).await?;
    publish(
        ctx,
        &mut tx,
        product_id,
        BundleMutation {
            event_type: PRODUCT_BUNDLE_REMOVED,
            payload: json!({ "pricing": current.pricing.as_str() }),
            changes: bundle_changes(Some(&current), None),
            ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

async fn details(
    ctx: &Ctx<'_>,
    product_id: Uuid,
    price_list_id: Option<Uuid>,
) -> Result<Option<BundleDetails>, AppError> {
    let Some(bundle) = BundleStore::of(ctx).find(product_id).await? else {
        return Ok(None);
    };
    Ok(CatalogBundleExpander
        .expand(ctx, product_id, price_list_id)
        .await?
        .map(|expansion| BundleDetails {
            expansion,
            updated_at: bundle.updated_at,
        }))
}

async fn existing_product(ctx: &Ctx<'_>, product_id: &str) -> Result<Uuid, AppError> {
    let id = Uuid::parse_str(product_id.trim()).map_err(|_| product_not_found())?;
    let found = BundleStore::of(ctx).products(&[id]).await?;
    if found.is_empty() {
        return Err(product_not_found());
    }
    Ok(id)
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    product_id: Uuid,
    mutation: BundleMutation,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let BundleMutation {
        event_type,
        payload,
        changes,
        ip,
    } = mutation;
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::new(RecordType::PRODUCT, product_id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(payload),
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
