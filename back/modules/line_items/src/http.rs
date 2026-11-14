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
    LineItemInput, LineItemLine, LineItemsSummary, ReorderLineItemsCommand, SaveLineItemCommand,
};

use crate::service;
use aphel_kernel::Ctx;
use aphel_sdk::line_items::money;
use aphel_sdk::line_items::{AmountMode, DiscountKind, Recurrence};
use aphel_sdk::platform::api::dto::common::{
    Data, ErrorEnvelope, decimal_text, double_option, timestamp,
};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::shared::RecordRef;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = Recurrence)]
pub enum RecurrenceSchema {
    None,
    Monthly,
    Quarterly,
    Yearly,
}

impl From<Recurrence> for RecurrenceSchema {
    fn from(value: Recurrence) -> Self {
        match value {
            Recurrence::None => Self::None,
            Recurrence::Monthly => Self::Monthly,
            Recurrence::Quarterly => Self::Quarterly,
            Recurrence::Yearly => Self::Yearly,
        }
    }
}

impl From<RecurrenceSchema> for Recurrence {
    fn from(value: RecurrenceSchema) -> Self {
        match value {
            RecurrenceSchema::None => Self::None,
            RecurrenceSchema::Monthly => Self::Monthly,
            RecurrenceSchema::Quarterly => Self::Quarterly,
            RecurrenceSchema::Yearly => Self::Yearly,
        }
    }
}

pub(crate) const TAG: &str = "line-items";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_line_items, create_line_item))
        .routes(routes!(update_line_item, delete_line_item))
        .routes(routes!(reorder_line_items))
        .routes(routes!(apply_line_items_total))
}

#[utoipa::path(
    get,
    path = "/line-items/{record_type}/{record_id}/items",
    tag = TAG,
    operation_id = "line_items_list",
    security(("session_cookie" = [])),
    description = "Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction.",
    params(("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id")),
    responses(
        (status = 200, description = "Items and totals of the record", body = Data<LineItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity read permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found, not visible or not attachable (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "A declared monetary field is hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_line_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<LineItemsResponse>>, ApiError> {
    let summary = service::summary(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    post,
    path = "/line-items/{record_type}/{record_id}/items",
    tag = TAG,
    operation_id = "line_items_create",
    security(("session_cookie" = [])),
    description = "Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction.",
    params(("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id")),
    request_body = LineItemRequest,
    responses(
        (status = 201, description = "Item added", body = Data<LineItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity update permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found, not visible or not attachable (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid item (`validation_error` with fields such as `name`, `quantity`, `discount_value` or an unknown/inactive `product_id`), product in another currency (`product_currency_mismatch`), item limit reached (`line_items_limit_reached`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn create_line_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<LineItemRequest>,
) -> Result<(StatusCode, Json<Data<LineItemResponse>>), ApiError> {
    let item = service::add(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(item.into()))))
}

#[utoipa::path(
    patch,
    path = "/line-items/{record_type}/{record_id}/items/{item_id}",
    tag = TAG,
    operation_id = "line_items_update",
    security(("session_cookie" = [])),
    description = "Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction.",
    params(
        ("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id"),
        ("item_id" = String, Path, description = "Item id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the item as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = LineItemRequest,
    responses(
        (status = 200, description = "Item updated", body = Data<LineItemResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity update permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record or item not found (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The item changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid item or malformed `If-Match` (`validation_error`), product in another currency (`product_currency_mismatch`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn update_line_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path((record_type, record_id, item_id)): Path<(String, String, String)>,
    JsonBody(request): JsonBody<LineItemRequest>,
) -> Result<Json<Data<LineItemResponse>>, ApiError> {
    let item = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
        &item_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(item.into())))
}

#[utoipa::path(
    delete,
    path = "/line-items/{record_type}/{record_id}/items/{item_id}",
    tag = TAG,
    operation_id = "line_items_delete",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id"),
        ("item_id" = String, Path, description = "Item id"),
    ),
    responses(
        (status = 204, description = "Item removed; publishes `line_items.item_removed`. In the `items` amount mode the parent amount is recalculated (removing the last item sets it to `0.00`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity update permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record or item not found (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn delete_line_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, item_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    service::remove(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
        &item_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/line-items/{record_type}/{record_id}/items/order",
    tag = TAG,
    operation_id = "line_items_reorder",
    security(("session_cookie" = [])),
    description = "Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction.",
    params(("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id")),
    request_body = ReorderLineItemsRequest,
    responses(
        (status = 200, description = "Items in the new order", body = Data<LineItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity update permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found, not visible or not attachable (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`item_ids` is not a permutation of the items (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_line_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<ReorderLineItemsRequest>,
) -> Result<Json<Data<LineItemsResponse>>, ApiError> {
    let summary = service::reorder(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    post,
    path = "/line-items/{record_type}/{record_id}/items/apply-total",
    tag = TAG,
    operation_id = "line_items_apply_total",
    security(("session_cookie" = [])),
    description = "Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction.",
    params(("record_type" = String, Path, description = "Entity key"), ("record_id" = String, Path, description = "Record id")),
    responses(
        (status = 200, description = "Items and totals after the update", body = Data<LineItemsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the parent entity update permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found, not visible or not attachable (`line_item_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The parent has no items (`record_has_no_items`) or `amount` is not editable for the role (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn apply_line_items_total(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<LineItemsResponse>>, ApiError> {
    let summary = service::apply_total(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &RecordRef::parse(&record_type, &record_id)?,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
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
#[schema(as = AmountMode)]
pub(crate) enum AmountModeSchema {
    Manual,
    Items,
}

impl From<AmountMode> for AmountModeSchema {
    fn from(value: AmountMode) -> Self {
        match value {
            AmountMode::Manual => Self::Manual,
            AmountMode::Items => Self::Items,
        }
    }
}

impl From<AmountModeSchema> for AmountMode {
    fn from(value: AmountModeSchema) -> Self {
        match value {
            AmountModeSchema::Manual => Self::Manual,
            AmountModeSchema::Items => Self::Items,
        }
    }
}

fn quantity(value: Decimal) -> String {
    value.normalize().to_string()
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct LineItemRequest {
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
    #[schema(value_type = Option<Value>, example = "2")]
    pub(crate) quantity: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<Value>, example = "1500.00")]
    pub(crate) unit_price: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DiscountTypeSchema>)]
    pub(crate) discount_type: Option<Option<DiscountTypeSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<Value>, example = "10")]
    pub(crate) discount_value: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    pub(crate) tax_percent: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<RecurrenceSchema>)]
    pub(crate) recurrence: Option<Option<RecurrenceSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 12)]
    pub(crate) term_months: Option<Option<Value>>,
}

impl LineItemRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveLineItemCommand {
        let decimal =
            |value: Option<Option<Value>>| value.map(|value| value.and_then(decimal_text));
        SaveLineItemCommand {
            input: LineItemInput {
                product_id: self.product_id,
                name: self.name,
                unit: self.unit,
                quantity: decimal(self.quantity),
                unit_price: decimal(self.unit_price),
                discount_type: self
                    .discount_type
                    .map(|value| value.map(|kind| DiscountKind::from(kind).as_str().to_owned())),
                discount_value: decimal(self.discount_value),
                tax_percent: decimal(self.tax_percent),
                recurrence: self.recurrence.map(|value| {
                    value.map(|recurrence| Recurrence::from(recurrence).as_str().to_owned())
                }),
                term_months: decimal(self.term_months),
            },
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ReorderLineItemsRequest {
    pub(crate) item_ids: Vec<String>,
}

impl ReorderLineItemsRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ReorderLineItemsCommand {
        ReorderLineItemsCommand {
            item_ids: self.item_ids,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LineItemResponse {
    pub(crate) id: String,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
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
    pub(crate) tax_percent: String,
    pub(crate) tax: String,
    #[schema(example = "3000.00")]
    pub(crate) gross: String,
    #[schema(example = "300.00")]
    pub(crate) discount_amount: String,
    #[schema(example = "2700.00")]
    pub(crate) total: String,
    pub(crate) recurrence: RecurrenceSchema,
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

impl From<LineItemLine> for LineItemResponse {
    fn from(line: LineItemLine) -> Self {
        let monthly_amount = aphel_sdk::line_items::monthly_amount(&line.item).map(money);
        let item = line.item;
        let data = item.data;
        let tax = money(data.amounts().map_or(Decimal::ZERO, |amounts| amounts.tax));
        Self {
            id: item.id.to_string(),
            record_type: item.record.record_type.to_string(),
            record_id: item.record.record_id.to_string(),
            product_id: data.product_id.map(|id| id.to_string()),
            name: data.name,
            unit: data.unit,
            quantity: quantity(data.quantity),
            unit_price: money(data.unit_price),
            discount_type: data.discount_kind.into(),
            discount_value: money(data.discount_value),
            tax_percent: data.tax_percent.normalize().to_string(),
            tax,
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
pub struct LineItemsResponse {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) items: Vec<LineItemResponse>,
    #[schema(example = "3000.00")]
    pub(crate) subtotal: String,
    #[schema(example = "300.00")]
    pub(crate) discount_total: String,
    #[schema(example = "2700.00")]
    pub(crate) total: String,
    #[schema(example = "2700.00")]
    pub(crate) record_amount: Option<String>,
    pub(crate) amount_mode: AmountModeSchema,
    pub(crate) amount_managed: bool,
    pub(crate) can_apply_total: bool,
}

impl From<LineItemsSummary> for LineItemsResponse {
    fn from(summary: LineItemsSummary) -> Self {
        Self {
            record_type: summary.record.record_type.to_string(),
            record_id: summary.record.record_id.to_string(),
            currency: summary.currency,
            items: summary.items.into_iter().map(Into::into).collect(),
            subtotal: money(summary.subtotal),
            discount_total: money(summary.discount_total),
            total: money(summary.total),
            record_amount: summary.record_amount.map(money),
            amount_mode: summary.amount_mode.into(),
            amount_managed: summary.amount_managed,
            can_apply_total: summary.can_apply_total,
        }
    }
}
