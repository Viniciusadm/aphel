use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{PriceListSelection, PriceRequest};
use crate::hooks::{CatalogPriceListResolver, PriceListResolver};
use crate::http::{PriceListListParams, PriceListResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};
use aphel_sdk::line_items::{normalize_quantity, parse_money};
use catalog_items::domain::{normalize_currency, require_catalog};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    filter: PriceListListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct ResolveInput {
    product_id: String,
    #[serde(default)]
    price_list_id: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    company_id: Option<String>,
    #[serde(default)]
    quantity: Option<Value>,
    #[serde(default)]
    on: Option<NaiveDate>,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid price request",
        vec![FieldViolation::new(field, message.to_owned())],
    )
}

fn uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|_| invalid(field, "must be a valid id"))
}

fn quantity(value: Option<Value>) -> Result<rust_decimal::Decimal, AppError> {
    let Some(value) = value else {
        return Ok(rust_decimal::Decimal::ONE);
    };
    let text = match value {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text,
        _ => return Err(invalid("quantity", "must be a positive number")),
    };
    parse_money(&text)
        .and_then(normalize_quantity)
        .ok_or_else(|| invalid("quantity", "must be a positive number"))
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let listing = service::list(ctx, input.filter.into_filter(), input.page.window()).await?;
        let lists: Vec<PriceListResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(lists)?, "total": listing.total }))
    })
}

fn resolve<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ResolveInput = operation_input(input)?;
        require_catalog(ctx.scope, &Permission::PRODUCTS_READ)?;
        let product_id = uuid(&input.product_id, "product_id")?;
        let quantity = quantity(input.quantity)?;
        let on = input.on.unwrap_or_else(|| ctx.now().date_naive());
        let resolver = CatalogPriceListResolver;
        let list = match input.price_list_id.as_deref() {
            Some(id) => resolver.find(ctx, uuid(id, "price_list_id")?).await?,
            None => {
                let currency = input
                    .currency
                    .as_deref()
                    .and_then(normalize_currency)
                    .ok_or_else(|| {
                        invalid("currency", "is required when price_list_id is omitted")
                    })?;
                let company_id = input
                    .company_id
                    .as_deref()
                    .map(|id| uuid(id, "company_id"))
                    .transpose()?;
                resolver
                    .default_for(
                        ctx,
                        &PriceListSelection {
                            currency,
                            company_id,
                            on,
                        },
                    )
                    .await?
            }
        };
        let Some(list) = list else {
            return Ok(json!({ "price_list": null, "unit_price": null }));
        };
        let price = resolver
            .price(
                ctx,
                &PriceRequest {
                    price_list_id: list.id,
                    product_id,
                    quantity,
                    on,
                },
            )
            .await?;
        Ok(json!({
            "price_list": { "id": list.id.to_string(), "name": list.name, "currency": list.currency },
            "product_id": product_id.to_string(),
            "quantity": quantity.to_string(),
            "on": on.to_string(),
            "unit_price": price.as_ref().map(|price| price.unit_price.to_string()),
            "currency": price.map(|price| price.currency),
        }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "pricing.list",
            "Price lists, the default of each currency first: { currency?, active?, search?, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::read(
            "pricing.resolve",
            "Unit price of a product from a price list (quantity tiers applied): { product_id, price_list_id? , currency? (required without price_list_id; picks the list for the company or the currency default), company_id?, quantity? (default 1), on? (YYYY-MM-DD, default today) } -> { price_list: { id, name, currency } | null, unit_price (string) | null, currency, quantity, on }",
            resolve,
        )),
    ]
}
