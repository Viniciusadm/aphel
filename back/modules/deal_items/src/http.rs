use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    DealItemInput, DealItemLine, DealItemSettings, DealItemsSummary, ReorderDealItemsCommand,
    SaveDealItemCommand, UpdateDealItemSettingsCommand,
};
use crate::recurring::monthly_amount;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, decimal_text, double_option, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;
use deals::http::{DecimalInputSchema, money};
use products::domain::{DealAmountMode, DiscountKind, ProductRecurrence};
use products::http::ProductRecurrenceSchema;

pub(crate) const TAG: &str = "deal-items";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_deal_items, create_deal_item))
        .routes(routes!(update_deal_item, delete_deal_item))
        .routes(routes!(reorder_deal_items))
        .routes(routes!(apply_deal_items_total))
        .routes(routes!(get_deal_item_settings, update_deal_item_settings))
}

#[utoipa::path(
    get,
    path = "/deals/{deal_id}/items",
    tag = TAG,
    operation_id = "deal_items_list",
    security(("session_cookie" = [])),
    description = "Lists the items (products or free lines) of a deal in their order, with the totals (permission `deals.read` on the deal, including the pipeline restriction by team). Amounts are in the currency of the deal. `amount_mode` is the organization setting and `amount_managed` is `true` when the deal amount is the sum of the items (mode `items` and at least one item); then `amount` cannot be changed directly on the deal. Requires the `amount` field of deals to be visible for the role.",
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Items and totals of the deal", body = Data<DealItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The `amount` field of deals is hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_deal_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<DealItemsResponse>>, ApiError> {
    let summary =
        service::summary(&Ctx::from_state(&state, &tenant.scope).await?, &deal_id).await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/items",
    tag = TAG,
    operation_id = "deal_items_create",
    security(("session_cookie" = [])),
    description = "Adds an item at the end of a deal (permission `deals.update` on the deal) and publishes `deal.item_added`. With `product_id` (an active product in the currency of the deal) the name, unit and unit price are copied from the product unless sent; without it the item is free and `name` is required. `quantity` defaults to 1 (greater than zero, up to 4 decimals). `discount_type` is `none` (default), `percent` (0–100) or `amount`; the line total is `quantity × unit_price − discount`, rounded to 2 places and never negative. In the `items` amount mode the deal amount becomes the sum of the items in the same transaction and `deal.amount_changed` is published (which requires `amount` to be editable for the role). `recurrence` (`none` = one-time, default, `monthly`, `quarterly`, `yearly`) and `term_months` (1–600, only for recurring items) are copied from the product unless sent. After every change of the items the deal gets `mrr` (sum of the monthly value of the recurring items: line total ÷ 1, 3 or 12), `arr` (`mrr × 12`), `tcv` (one-time totals plus monthly value × term, 12 months when the item has no term) and, when it has `contract_start` (filled with the win date of a won deal), `contract_end` = start + longest term − 1 day; changes publish `deal.updated` with `changes` and `source: items`. At most 200 items per deal.",
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = DealItemRequest,
    responses(
        (status = 201, description = "Item added", body = Data<DealItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid item (`validation_error` with fields such as `name`, `quantity`, `discount_value` or an unknown/inactive `product_id`), product in another currency (`product_currency_mismatch`), item limit reached (`deal_items_limit_reached`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn create_deal_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<DealItemRequest>,
) -> Result<(StatusCode, Json<Data<DealItemResponse>>), ApiError> {
    let item = service::add(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(item.into()))))
}

#[utoipa::path(
    patch,
    path = "/deals/{deal_id}/items/{item_id}",
    tag = TAG,
    operation_id = "deal_items_update",
    security(("session_cookie" = [])),
    description = "Partially updates an item of a deal (permission `deals.update` on the deal) and publishes `deal.item_updated` with the changes; omitted properties stay unchanged. Changing `product_id` copies name, unit and unit price of the new product unless sent; `product_id: null` turns it into a free item keeping its values. In the `items` amount mode the deal amount is recalculated in the same transaction (`deal.amount_changed`).",
    params(
        ("deal_id" = String, Path, description = "Deal id"),
        ("item_id" = String, Path, description = "Item id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the item as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = DealItemRequest,
    responses(
        (status = 200, description = "Item updated", body = Data<DealItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal or item not found (`deal_not_found`, `deal_item_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The item changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid item or malformed `If-Match` (`validation_error`), product in another currency (`product_currency_mismatch`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn update_deal_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path((deal_id, item_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<DealItemRequest>,
) -> Result<Json<Data<DealItemResponse>>, ApiError> {
    let item = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        &item_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(item.into())))
}

#[utoipa::path(
    delete,
    path = "/deals/{deal_id}/items/{item_id}",
    tag = TAG,
    operation_id = "deal_items_delete",
    security(("session_cookie" = [])),
    params(
        ("deal_id" = String, Path, description = "Deal id"),
        ("item_id" = String, Path, description = "Item id"),
    ),
    responses(
        (status = 204, description = "Item removed; publishes `deal.item_removed`. In the `items` amount mode the deal amount is recalculated (removing the last item sets it to `0.00`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal or item not found (`deal_not_found`, `deal_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn delete_deal_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((deal_id, item_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    service::remove(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        &item_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/deals/{deal_id}/items/order",
    tag = TAG,
    operation_id = "deal_items_reorder",
    security(("session_cookie" = [])),
    description = "Replaces the order of the items of a deal (permission `deals.update` on the deal); `item_ids` must list every item exactly once. Publishes `deal.items_reordered` when the order changes.",
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = ReorderDealItemsRequest,
    responses(
        (status = 200, description = "Items in the new order", body = Data<DealItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`item_ids` is not a permutation of the items (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_deal_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<ReorderDealItemsRequest>,
) -> Result<Json<Data<DealItemsResponse>>, ApiError> {
    let summary = service::reorder(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/items/apply-total",
    tag = TAG,
    operation_id = "deal_items_apply_total",
    security(("session_cookie" = [])),
    description = "Sets the deal amount to the total of its items now, in any amount mode (permission `deals.update` on the deal and `amount` editable for the role). Useful in the `manual` mode, or right after switching to `items`, since switching does not rewrite existing amounts. Publishes `deal.amount_changed` when the amount changes.",
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Items and totals after the update", body = Data<DealItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The deal has no items (`deal_has_no_items`) or `amount` is not editable for the role (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn apply_deal_items_total(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<DealItemsResponse>>, ApiError> {
    let summary = service::apply_total(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    get,
    path = "/deal-items/settings",
    tag = TAG,
    operation_id = "deal_item_settings_get",
    security(("session_cookie" = [])),
    description = "Returns how the deal amount relates to its items in the organization (permission `deals.read` or `settings.manage`): `manual` (default; the amount is typed and items are informative) or `items` (the amount of a deal with items is the sum of its items). `updated_at` is `null` while never saved.",
    responses(
        (status = 200, description = "Deal item settings", body = Data<DealItemSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn get_deal_item_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<DealItemSettingsResponse>>, ApiError> {
    let settings = service::settings(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    patch,
    path = "/deal-items/settings",
    tag = TAG,
    operation_id = "deal_item_settings_update",
    security(("session_cookie" = [])),
    description = "Changes the amount mode of the organization (permission `settings.manage`) and publishes `deal_item_settings.updated`. Switching to `items` does not rewrite existing amounts: each deal follows the sum from the next change of its items (or `POST /deals/{deal_id}/items/apply-total`). Deals without items keep a manual amount.",
    params(
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the settings as read (ISO-8601, quotes optional); ignored while the settings were never saved"),
    ),
    request_body = UpdateDealItemSettingsRequest,
    responses(
        (status = 200, description = "Settings updated", body = Data<DealItemSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "The settings changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `amount_mode` or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_deal_item_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    JsonBody(request): JsonBody<UpdateDealItemSettingsRequest>,
) -> Result<Json<Data<DealItemSettingsResponse>>, ApiError> {
    let settings = service::update_settings(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(settings.into())))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DiscountType)]
pub(crate) enum DiscountTypeSchema {
    None,
    Percent,
    Amount,
}

impl From<DiscountKind> for DiscountTypeSchema {
    fn from(value: DiscountKind) -> Self {
        match value {
            DiscountKind::None => Self::None,
            DiscountKind::Percent => Self::Percent,
            DiscountKind::Amount => Self::Amount,
        }
    }
}

impl From<DiscountTypeSchema> for DiscountKind {
    fn from(value: DiscountTypeSchema) -> Self {
        match value {
            DiscountTypeSchema::None => Self::None,
            DiscountTypeSchema::Percent => Self::Percent,
            DiscountTypeSchema::Amount => Self::Amount,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DealAmountMode)]
pub(crate) enum DealAmountModeSchema {
    Manual,
    Items,
}

impl From<DealAmountMode> for DealAmountModeSchema {
    fn from(value: DealAmountMode) -> Self {
        match value {
            DealAmountMode::Manual => Self::Manual,
            DealAmountMode::Items => Self::Items,
        }
    }
}

impl From<DealAmountModeSchema> for DealAmountMode {
    fn from(value: DealAmountModeSchema) -> Self {
        match value {
            DealAmountModeSchema::Manual => Self::Manual,
            DealAmountModeSchema::Items => Self::Items,
        }
    }
}

fn quantity(value: Decimal) -> String {
    value.normalize().to_string()
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct DealItemRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) product_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Onboarding")]
    pub(crate) name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "hour")]
    pub(crate) unit: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "2")]
    pub(crate) quantity: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "1500.00")]
    pub(crate) unit_price: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DiscountTypeSchema>)]
    pub(crate) discount_type: Option<Option<DiscountTypeSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "10")]
    pub(crate) discount_value: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<ProductRecurrenceSchema>)]
    pub(crate) recurrence: Option<Option<ProductRecurrenceSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 12)]
    pub(crate) term_months: Option<Option<Value>>,
}

impl DealItemRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveDealItemCommand {
        let decimal =
            |value: Option<Option<Value>>| value.map(|value| value.and_then(decimal_text));
        SaveDealItemCommand {
            input: DealItemInput {
                product_id: self.product_id,
                name: self.name,
                unit: self.unit,
                quantity: decimal(self.quantity),
                unit_price: decimal(self.unit_price),
                discount_type: self
                    .discount_type
                    .map(|value| value.map(|kind| DiscountKind::from(kind).as_str().to_owned())),
                discount_value: decimal(self.discount_value),
                recurrence: self.recurrence.map(|value| {
                    value.map(|recurrence| ProductRecurrence::from(recurrence).as_str().to_owned())
                }),
                term_months: decimal(self.term_months),
            },
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ReorderDealItemsRequest {
    pub(crate) item_ids: Vec<String>,
}

impl ReorderDealItemsRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ReorderDealItemsCommand {
        ReorderDealItemsCommand {
            item_ids: self.item_ids,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealItemResponse {
    pub(crate) id: String,
    pub(crate) deal_id: String,
    pub(crate) product_id: Option<String>,
    pub(crate) name: String,
    pub(crate) unit: Option<String>,
    #[schema(example = "2")]
    pub(crate) quantity: String,
    #[schema(example = "1500.00")]
    pub(crate) unit_price: String,
    pub(crate) discount_type: DiscountTypeSchema,
    #[schema(example = "10.00")]
    pub(crate) discount_value: String,
    #[schema(example = "3000.00")]
    pub(crate) gross: String,
    #[schema(example = "300.00")]
    pub(crate) discount_amount: String,
    #[schema(example = "2700.00")]
    pub(crate) total: String,
    pub(crate) recurrence: ProductRecurrenceSchema,
    #[schema(example = 12)]
    pub(crate) term_months: Option<u16>,
    #[schema(example = "225.00")]
    pub(crate) monthly_amount: Option<String>,
    #[schema(example = "13500.00")]
    pub(crate) amount_base: Option<String>,
    #[schema(example = "5.4")]
    pub(crate) exchange_rate: Option<String>,
    pub(crate) position: i32,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<DealItemLine> for DealItemResponse {
    fn from(line: DealItemLine) -> Self {
        let monthly_amount = monthly_amount(&line.item).map(money);
        let item = line.item;
        let data = item.data;
        Self {
            id: item.id.to_string(),
            deal_id: item.deal_id.to_string(),
            product_id: data.product_id.map(|id| id.to_string()),
            name: data.name,
            unit: data.unit,
            quantity: quantity(data.quantity),
            unit_price: money(data.unit_price),
            discount_type: data.discount_kind.into(),
            discount_value: money(data.discount_value),
            gross: money(line.gross),
            discount_amount: money(line.discount),
            total: money(item.total),
            recurrence: data.recurrence.into(),
            term_months: data.term_months,
            monthly_amount,
            amount_base: item.amount_base.map(money),
            exchange_rate: item.exchange_rate.map(|rate| rate.normalize().to_string()),
            position: item.position,
            created_by: item.created_by.map(|id| id.into_inner()),
            created_at: timestamp(item.created_at),
            updated_at: timestamp(item.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DealItemsResponse {
    pub(crate) deal_id: String,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) items: Vec<DealItemResponse>,
    #[schema(example = "3000.00")]
    pub(crate) subtotal: String,
    #[schema(example = "300.00")]
    pub(crate) discount_total: String,
    #[schema(example = "2700.00")]
    pub(crate) total: String,
    #[schema(example = "2700.00")]
    pub(crate) deal_amount: Option<String>,
    pub(crate) amount_mode: DealAmountModeSchema,
    pub(crate) amount_managed: bool,
}

impl From<DealItemsSummary> for DealItemsResponse {
    fn from(summary: DealItemsSummary) -> Self {
        Self {
            deal_id: summary.deal_id.to_string(),
            currency: summary.currency,
            items: summary.items.into_iter().map(Into::into).collect(),
            subtotal: money(summary.subtotal),
            discount_total: money(summary.discount_total),
            total: money(summary.total),
            deal_amount: summary.deal_amount.map(money),
            amount_mode: summary.amount_mode.into(),
            amount_managed: summary.amount_managed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealItemSettingsResponse {
    pub(crate) amount_mode: DealAmountModeSchema,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: Option<String>,
}

impl From<DealItemSettings> for DealItemSettingsResponse {
    fn from(settings: DealItemSettings) -> Self {
        Self {
            amount_mode: settings.amount_mode.into(),
            updated_at: settings.updated_at.map(timestamp),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateDealItemSettingsRequest {
    pub(crate) amount_mode: Option<DealAmountModeSchema>,
}

impl UpdateDealItemSettingsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateDealItemSettingsCommand {
        UpdateDealItemSettingsCommand {
            amount_mode: self.amount_mode.map(Into::into),
            ip,
            expected_updated_at,
        }
    }
}
