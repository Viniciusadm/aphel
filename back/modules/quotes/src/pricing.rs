use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::{
    MAX_ITEM_DESCRIPTION_LENGTH, QuoteItem, QuoteItemComponent, QuoteItemData, QuoteItemInput,
    normalize_tax_percent,
};
use crate::gateway::QuoteCatalog;
use crate::service::parse_id;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::InputValidator;
use aphel_sdk::line_items::CatalogItem;
use aphel_sdk::line_items::{
    DiscountKind, LineData, MAX_ITEM_NAME_LENGTH, MAX_UNIT_LENGTH, MONEY_DECIMALS,
    QUANTITY_DECIMALS, Recurrence, normalize_quantity, parse_money, parse_term_months,
    term_violation,
};
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::pricing::PriceRequest;

pub(crate) struct PricingContext<'a> {
    pub(crate) currency: &'a str,
    pub(crate) price_list_id: Option<Uuid>,
    pub(crate) on: NaiveDate,
    pub(crate) reprice: bool,
}

pub(crate) fn parse_decimal(text: &str) -> Option<Decimal> {
    let text = text.trim();
    Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| text.trim().is_empty())
}

pub(crate) fn item_input_from_deal(item: &LineData) -> QuoteItemInput {
    QuoteItemInput {
        product_id: item.product_id.map(|id| id.to_string()),
        name: Some(item.name.clone()),
        description: None,
        unit: item.unit.clone(),
        quantity: Some(item.quantity.normalize().to_string()),
        unit_price: Some(format!("{:.2}", item.unit_price.round_dp(2))),
        discount_type: Some(item.discount_kind.as_str().to_owned()),
        discount_value: Some(format!("{:.2}", item.discount_value.round_dp(2))),
        tax_percent: Some(item.tax_percent.to_string()),
        recurrence: Some(item.recurrence.as_str().to_owned()),
        term_months: item.term_months.map(|months| months.to_string()),
    }
}

pub(crate) fn item_input_from_quote(item: &QuoteItemData) -> QuoteItemInput {
    QuoteItemInput {
        product_id: item.product_id.map(|id| id.to_string()),
        name: Some(item.name.clone()),
        description: item.description.clone(),
        unit: item.unit.clone(),
        quantity: Some(item.quantity.normalize().to_string()),
        unit_price: Some(format!("{:.2}", item.unit_price.round_dp(2))),
        discount_type: Some(item.discount_kind.as_str().to_owned()),
        discount_value: Some(format!("{:.2}", item.discount_value.round_dp(2))),
        tax_percent: Some(item.tax_percent.normalize().to_string()),
        recurrence: Some(item.recurrence.as_str().to_owned()),
        term_months: item.term_months.map(|months| months.to_string()),
    }
}

pub(crate) async fn resolve_items(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    inputs: Vec<QuoteItemInput>,
    context: &PricingContext<'_>,
    validator: &mut InputValidator,
) -> Result<Vec<QuoteItemData>, AppError> {
    let mut items = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.into_iter().enumerate() {
        items.push(resolve_item(ctx, scope, index, input, context, validator).await?);
    }
    Ok(items)
}

async fn resolve_item(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    index: usize,
    input: QuoteItemInput,
    context: &PricingContext<'_>,
    validator: &mut InputValidator,
) -> Result<QuoteItemData, AppError> {
    let path = |field: &str| format!("items.{index}.{field}");
    let mut data = QuoteItemData {
        quantity: Decimal::ONE,
        ..QuoteItemData::default()
    };
    let product = match input.product_id.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => match parse_id(text) {
            None => {
                validator.violation(&path("product_id"), "must be a valid id");
                None
            }
            Some(id) => {
                match aphel_sdk::line_items::catalog_item(&ctx.with_scope(scope), id).await? {
                    None => {
                        validator.violation(&path("product_id"), "product was not found");
                        None
                    }
                    Some(found) if !found.active => {
                        validator.violation(&path("product_id"), "product is inactive");
                        None
                    }
                    Some(found) => Some(found),
                }
            }
        },
    };
    if let Some(text) = input
        .quantity
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        match parse_decimal(text).and_then(normalize_quantity) {
            Some(quantity) => data.quantity = quantity,
            None => validator.violation(
                &path("quantity"),
                format!(
                    "must be a number greater than zero with at most {QUANTITY_DECIMALS} decimal places"
                ),
            ),
        }
    }
    let repriced = context.reprice && product.is_some();
    let explicit_price = !(blank(&input.unit_price) || repriced);
    if explicit_price {
        match input.unit_price.as_deref().and_then(parse_money) {
            Some(price) => data.unit_price = price,
            None => validator.violation(
                &path("unit_price"),
                format!(
                    "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                ),
            ),
        }
    }
    if let Some(product) = &product {
        data.product_id = Some(product.id);
        data.name = product.name.clone();
        data.unit = product.unit.clone();
        data.recurrence = product.recurrence;
        data.term_months = product.term_months;
        let price = product_price(ctx, scope, product, data.quantity, context).await?;
        if let Some(expansion) = QuoteCatalog::of(ctx)
            .expand(scope, product.id, context.price_list_id)
            .await?
        {
            data.components = expansion
                .components
                .iter()
                .map(|component| QuoteItemComponent {
                    product_id: Some(component.product_id),
                    name: component.name.clone(),
                    code: component.code.clone(),
                    quantity: component.quantity,
                    unit_price: component.unit_price,
                })
                .collect();
            if !explicit_price {
                if expansion.currency == context.currency {
                    data.unit_price = expansion.unit_price;
                } else {
                    currency_violation(validator, index, &expansion.currency, context.currency);
                }
            }
        } else if !explicit_price {
            match price {
                Some(price) => data.unit_price = price,
                None => currency_violation(validator, index, &product.currency, context.currency),
            }
        }
    }
    if let Some(name) = input.name.filter(|name| !name.trim().is_empty()) {
        data.name = validator
            .text(&path("name"), Some(name), MAX_ITEM_NAME_LENGTH)
            .unwrap_or_default();
    }
    if data.name.trim().is_empty() {
        validator.violation(&path("name"), "is required");
    }
    if input.description.is_some() {
        data.description = validator.text(
            &path("description"),
            input.description,
            MAX_ITEM_DESCRIPTION_LENGTH,
        );
    }
    if input.unit.is_some() {
        data.unit = validator.text(&path("unit"), input.unit, MAX_UNIT_LENGTH);
    }
    match input.discount_type.as_deref().map(str::trim) {
        None | Some("") => {}
        Some(text) => match DiscountKind::parse(text) {
            Some(kind) => data.discount_kind = kind,
            None => validator.violation(&path("discount_type"), "must be none, percent or amount"),
        },
    }
    if !blank(&input.discount_value) {
        match input.discount_value.as_deref().and_then(parse_money) {
            Some(value) => data.discount_value = value,
            None => validator.violation(
                &path("discount_value"),
                format!(
                    "must be a non-negative number with at most {MONEY_DECIMALS} decimal places"
                ),
            ),
        }
    }
    if data.discount_kind == DiscountKind::None {
        data.discount_value = Decimal::ZERO;
    }
    if data.discount_kind == DiscountKind::Percent && data.discount_value > Decimal::ONE_HUNDRED {
        validator.violation(
            &path("discount_value"),
            "must be a percentage between 0 and 100",
        );
    }
    if !blank(&input.tax_percent) {
        match input
            .tax_percent
            .as_deref()
            .and_then(parse_decimal)
            .and_then(normalize_tax_percent)
        {
            Some(percent) => data.tax_percent = percent,
            None => validator.violation(
                &path("tax_percent"),
                "must be a percentage between 0 and 100",
            ),
        }
    }
    match input.recurrence.as_deref().map(str::trim) {
        None | Some("") => {}
        Some(text) => match Recurrence::parse(text) {
            Some(recurrence) => data.recurrence = recurrence,
            None => validator.violation(
                &path("recurrence"),
                "must be none, monthly, quarterly or yearly",
            ),
        },
    }
    if !blank(&input.term_months) {
        match input.term_months.as_deref().and_then(parse_term_months) {
            Some(months) => data.term_months = Some(months),
            None => validator.violation(&path("term_months"), term_violation()),
        }
    }
    if data.recurrence == Recurrence::None {
        data.term_months = None;
    }
    Ok(data)
}

fn currency_violation(validator: &mut InputValidator, index: usize, found: &str, expected: &str) {
    validator.violation(
        &format!("items.{index}.product_id"),
        format!("product is priced in {found} but the quote uses {expected}"),
    );
}

async fn product_price(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    product: &CatalogItem,
    quantity: Decimal,
    context: &PricingContext<'_>,
) -> Result<Option<Decimal>, AppError> {
    if let Some(price_list_id) = context.price_list_id
        && let Some(resolved) = QuoteCatalog::of(ctx)
            .price(
                scope,
                &PriceRequest {
                    price_list_id,
                    product_id: product.id,
                    quantity,
                    on: context.on,
                },
            )
            .await?
        && resolved.currency == context.currency
    {
        return Ok(Some(resolved.unit_price));
    }
    Ok((product.currency == context.currency).then_some(product.price))
}

pub(crate) fn deal_item_from_quote(item: &QuoteItem, with_product: bool) -> LineData {
    let discount = item.amounts.discount();
    LineData {
        product_id: item.data.product_id.filter(|_| with_product),
        name: item.data.name.clone(),
        unit: item.data.unit.clone(),
        quantity: item.data.quantity,
        unit_price: item.data.unit_price,
        discount_kind: if discount > Decimal::ZERO {
            DiscountKind::Amount
        } else {
            DiscountKind::None
        },
        discount_value: discount,
        tax_percent: item.data.tax_percent,
        recurrence: item.data.recurrence,
        term_months: item.data.term_months,
    }
}
