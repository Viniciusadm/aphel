use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    BundleComponent, BundleComponentInput, BundleDetails, BundlePricing, DiscountLimitInput,
    PriceList, PriceListAppliesTo, PriceListFilter, PriceListItem, PriceListItemInput,
    ReplacePriceListItemsCommand, RoleDiscountLimit, SaveBundleCommand, SavePriceListCommand,
    SetDiscountLimitsCommand,
};
use crate::{bundles, service};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{
    Data, ErrorEnvelope, Page, decimal_text, double_option, timestamp,
};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_sdk::catalog_items::API_TAG as PRODUCTS_TAG;
use aphel_sdk::line_items::money;
use aphel_sdk::money::DecimalInputSchema;

pub(crate) const TAG: &str = "pricing";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";
const FORBIDDEN_READ: &str = "Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`)";
const FORBIDDEN_WRITE: &str = "Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`)";
const FORBIDDEN_MANAGE: &str =
    "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)";
const NOT_FOUND: &str = "Price list not found (`price_list_not_found`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_price_lists, create_price_list))
        .routes(routes!(
            get_price_list,
            update_price_list,
            delete_price_list
        ))
        .routes(routes!(list_price_list_items, replace_price_list_items))
        .routes(routes!(get_discount_limits, set_discount_limits))
}

#[utoipa::path(
    get,
    path = "/pricing",
    tag = TAG,
    operation_id = "price_lists_list",
    security(("session_cookie" = [])),
    params(PriceListListParams, PaginationQuery),
    responses(
        (status = 200, description = "Price lists of the organization, the default of each currency first, then by name. `currency`, `active` and `search` (name or key) narrow the page", body = Page<PriceListResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
    )
)]
async fn list_price_lists(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    Query(params): Query<PriceListListParams>,
) -> Result<Json<Page<PriceListResponse>>, ApiError> {
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_filter(),
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/pricing",
    tag = TAG,
    operation_id = "price_lists_create",
    security(("session_cookie" = [])),
    request_body = PriceListRequest,
    responses(
        (status = 201, description = "Price list created (`price_list.created`). `currency` defaults to the organization currency and must be enabled; `applies_to` is `all` (default), `segment` (with a company `segment_id`) or `organizations` (with `company_ids`); `valid_from`/`valid_to` (inclusive, optional) bound when it is in effect. Marking `is_default` (only for `all`) replaces the previous default of the currency. `key` is generated from the name when omitted", body = Data<PriceListResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_WRITE, body = ErrorEnvelope),
        (status = 409, description = "Another price list uses the key (`price_list_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn create_price_list(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<PriceListRequest>,
) -> Result<(StatusCode, Json<Data<PriceListResponse>>), ApiError> {
    let list = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(list.into()))))
}

#[utoipa::path(
    get,
    path = "/pricing/{price_list_id}",
    tag = TAG,
    operation_id = "price_lists_get",
    security(("session_cookie" = [])),
    params(("price_list_id" = String, Path, description = "Price list id")),
    responses(
        (status = 200, description = "Price list", body = Data<PriceListResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn get_price_list(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(price_list_id): Path<String>,
) -> Result<Json<Data<PriceListResponse>>, ApiError> {
    let list = service::get(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &price_list_id,
    )
    .await?;
    Ok(Json(Data::new(list.into())))
}

#[utoipa::path(
    patch,
    path = "/pricing/{price_list_id}",
    tag = TAG,
    operation_id = "price_lists_update",
    security(("session_cookie" = [])),
    params(
        ("price_list_id" = String, Path, description = "Price list id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the price list as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = PriceListRequest,
    responses(
        (status = 200, description = "Price list updated (`price_list.updated` with the changes); only the fields sent change", body = Data<PriceListResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_WRITE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
        (status = 409, description = "Another price list uses the key (`price_list_key_taken`) or the list changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_price_list(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(price_list_id): Path<String>,
    JsonBody(request): JsonBody<PriceListRequest>,
) -> Result<Json<Data<PriceListResponse>>, ApiError> {
    let list = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &price_list_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(list.into())))
}

#[utoipa::path(
    delete,
    path = "/pricing/{price_list_id}",
    tag = TAG,
    operation_id = "price_lists_delete",
    security(("session_cookie" = [])),
    params(("price_list_id" = String, Path, description = "Price list id")),
    responses(
        (status = 204, description = "Price list and its items removed (`price_list.deleted`). Quotes that used it keep their prices"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_WRITE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn delete_price_list(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(price_list_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &price_list_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/pricing/{price_list_id}/items",
    tag = TAG,
    operation_id = "price_lists_items_list",
    security(("session_cookie" = [])),
    params(("price_list_id" = String, Path, description = "Price list id")),
    responses(
        (status = 200, description = "Every price of the list, with the product name, code and catalog price for reference. A product can have several tiers (`min_quantity`)", body = Data<Vec<PriceListItemResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
    )
)]
async fn list_price_list_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(price_list_id): Path<String>,
) -> Result<Json<Data<Vec<PriceListItemResponse>>>, ApiError> {
    let items = service::items(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &price_list_id,
    )
    .await?;
    Ok(Json(Data::new(items.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    put,
    path = "/pricing/{price_list_id}/items",
    tag = TAG,
    operation_id = "price_lists_items_replace",
    security(("session_cookie" = [])),
    params(
        ("price_list_id" = String, Path, description = "Price list id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the price list as last read; replacing the items bumps it"),
    ),
    request_body = ReplacePriceListItemsRequest,
    responses(
        (status = 200, description = "Items replaced in bulk by the list sent (`price_list.items_replaced`; at most 5000). `min_quantity` defaults to 1 and is unique per product: a quantity uses the tier with the largest `min_quantity` not above it, or the smallest tier when it is below all of them", body = Data<Vec<PriceListItemResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_WRITE, body = ErrorEnvelope),
        (status = 404, description = NOT_FOUND, body = ErrorEnvelope),
        (status = 409, description = "The list changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid items (`validation_error` with `details.fields` as `items.<index>.<field>`)", body = ErrorEnvelope),
    )
)]
async fn replace_price_list_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(price_list_id): Path<String>,
    JsonBody(request): JsonBody<ReplacePriceListItemsRequest>,
) -> Result<Json<Data<Vec<PriceListItemResponse>>>, ApiError> {
    let items = service::replace_items(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &price_list_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(items.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    get,
    path = "/pricing/discount-limits",
    tag = TAG,
    operation_id = "discount_limits_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Every role of the organization with its maximum discount in percent (`null` = no limit). A quote whose discount passes the limit of the role of its owner goes through the `quote_discount_limit` approval policy before it can be sent", body = Data<Vec<DiscountLimitResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
    )
)]
async fn get_discount_limits(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<DiscountLimitResponse>>>, ApiError> {
    let limits = service::discount_limits(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        limits.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    put,
    path = "/pricing/discount-limits",
    tag = TAG,
    operation_id = "discount_limits_set",
    security(("session_cookie" = [])),
    request_body = SetDiscountLimitsRequest,
    responses(
        (status = 200, description = "Limits replaced (`discount_limits.updated` with the changes): roles left out or sent with `max_discount_percent: null` have no limit. Percent between 0 and 100 with up to 2 decimal places", body = Data<Vec<DiscountLimitResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 422, description = "Unknown role or invalid percent (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn set_discount_limits(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<SetDiscountLimitsRequest>,
) -> Result<Json<Data<Vec<DiscountLimitResponse>>>, ApiError> {
    let limits = service::set_discount_limits(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        limits.into_iter().map(Into::into).collect(),
    )))
}

pub(crate) fn bundle_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(
        get_product_bundle,
        save_product_bundle,
        delete_product_bundle
    ))
}

#[utoipa::path(
    get,
    path = "/pricing/items/{product_id}/bundle",
    tag = PRODUCTS_TAG,
    operation_id = "products_bundle_get",
    security(("session_cookie" = [])),
    params(
        ("product_id" = String, Path, description = "Product id"),
        ProductBundleParams,
    ),
    responses(
        (status = 200, description = "Bundle of the product expanded into its components (`data: null` when the product is not a bundle). `fixed` uses the price of the product itself; `sum` adds component price × quantity. With `price_list_id`, prices come from the list in effect today (falling back to the catalog price) and `currency` is the currency of the list", body = Data<Option<ProductBundleResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product or price list not found (`product_not_found`, `price_list_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_product_bundle(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(product_id): Path<String>,
    Query(params): Query<ProductBundleParams>,
) -> Result<Json<Data<Option<ProductBundleResponse>>>, ApiError> {
    let bundle = bundles::get_bundle(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &product_id,
        params.price_list_id.as_deref(),
    )
    .await?;
    Ok(Json(Data::new(bundle.map(Into::into))))
}

#[utoipa::path(
    put,
    path = "/pricing/items/{product_id}/bundle",
    tag = PRODUCTS_TAG,
    operation_id = "products_bundle_save",
    security(("session_cookie" = [])),
    params(
        ("product_id" = String, Path, description = "Product id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the bundle as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = ProductBundleRequest,
    responses(
        (status = 200, description = "Product turned into a bundle, or its bundle replaced (`product_bundle.updated`). 1 to 50 components, each a different product with a positive quantity; bundles cannot contain bundles (one level only), and with `sum` every component must use the currency of the bundle", body = Data<ProductBundleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product not found (`product_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The product is a component of another bundle (`product_bundle_nested`) or the bundle changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pricing or components (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn save_product_bundle(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(product_id): Path<String>,
    JsonBody(request): JsonBody<ProductBundleRequest>,
) -> Result<Json<Data<ProductBundleResponse>>, ApiError> {
    let bundle = bundles::save_bundle(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &product_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(bundle.into())))
}

#[utoipa::path(
    delete,
    path = "/pricing/items/{product_id}/bundle",
    tag = PRODUCTS_TAG,
    operation_id = "products_bundle_delete",
    security(("session_cookie" = [])),
    params(("product_id" = String, Path, description = "Product id")),
    responses(
        (status = 204, description = "The product is no longer a bundle (`product_bundle.removed`); the product itself is kept"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product not found or not a bundle (`product_not_found`, `product_bundle_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_product_bundle(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(product_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    bundles::remove_bundle(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &product_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = PriceListAppliesTo)]
pub(crate) enum PriceListAppliesToSchema {
    All,
    Segment,
    Organizations,
}

impl From<PriceListAppliesTo> for PriceListAppliesToSchema {
    fn from(value: PriceListAppliesTo) -> Self {
        match value {
            PriceListAppliesTo::All => Self::All,
            PriceListAppliesTo::Segment => Self::Segment,
            PriceListAppliesTo::Organizations => Self::Organizations,
        }
    }
}

impl From<PriceListAppliesToSchema> for PriceListAppliesTo {
    fn from(value: PriceListAppliesToSchema) -> Self {
        match value {
            PriceListAppliesToSchema::All => Self::All,
            PriceListAppliesToSchema::Segment => Self::Segment,
            PriceListAppliesToSchema::Organizations => Self::Organizations,
        }
    }
}

fn date_text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn quantity_text(value: Decimal) -> String {
    value.normalize().to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PriceListResponse {
    pub(crate) id: String,
    #[schema(example = "wholesale")]
    pub(crate) key: String,
    #[schema(example = "Wholesale")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) applies_to: PriceListAppliesToSchema,
    pub(crate) segment_id: Option<String>,
    pub(crate) company_ids: Vec<String>,
    #[schema(example = "2026-01-01")]
    pub(crate) valid_from: Option<String>,
    #[schema(example = "2026-12-31")]
    pub(crate) valid_to: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) is_default: bool,
    pub(crate) item_count: u64,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<PriceList> for PriceListResponse {
    fn from(list: PriceList) -> Self {
        Self {
            id: list.id.to_string(),
            key: list.key,
            name: list.name,
            description: list.description,
            currency: list.currency,
            applies_to: list.applies_to.into(),
            segment_id: list.segment_id.map(|id| id.to_string()),
            company_ids: list.company_ids.iter().map(|id| id.to_string()).collect(),
            valid_from: list.valid_from.map(date_text),
            valid_to: list.valid_to.map(date_text),
            is_active: list.is_active,
            is_default: list.is_default,
            item_count: list.item_count,
            created_by: list.created_by.map(|id| id.into_inner()),
            created_at: timestamp(list.created_at),
            updated_at: timestamp(list.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct PriceListListParams {
    #[param(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) active: Option<bool>,
    pub(crate) search: Option<String>,
}

impl PriceListListParams {
    pub(crate) fn into_filter(self) -> PriceListFilter {
        PriceListFilter {
            currency: self.currency,
            active: self.active,
            search: self.search,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PriceListRequest {
    #[schema(example = "wholesale")]
    pub(crate) key: Option<String>,
    #[schema(example = "Wholesale")]
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) applies_to: Option<PriceListAppliesToSchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) segment_id: Option<Option<String>>,
    pub(crate) company_ids: Option<Vec<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2026-01-01")]
    pub(crate) valid_from: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2026-12-31")]
    pub(crate) valid_to: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_default: Option<bool>,
}

impl PriceListRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SavePriceListCommand {
        SavePriceListCommand {
            key: self.key,
            name: self.name,
            description: self.description,
            currency: self.currency,
            applies_to: self
                .applies_to
                .map(|value| PriceListAppliesTo::from(value).as_str().to_owned()),
            segment_id: self.segment_id,
            company_ids: self.company_ids,
            valid_from: self.valid_from,
            valid_to: self.valid_to,
            is_active: self.is_active,
            is_default: self.is_default,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PriceListItemResponse {
    pub(crate) id: String,
    pub(crate) product_id: String,
    pub(crate) product_name: String,
    pub(crate) product_code: Option<String>,
    #[schema(example = "1500.00")]
    pub(crate) product_price: String,
    #[schema(example = "BRL")]
    pub(crate) product_currency: String,
    #[schema(example = "1")]
    pub(crate) min_quantity: String,
    #[schema(example = "1350.00")]
    pub(crate) unit_price: String,
}

impl From<PriceListItem> for PriceListItemResponse {
    fn from(item: PriceListItem) -> Self {
        Self {
            id: item.id.to_string(),
            product_id: item.product_id.to_string(),
            product_name: item.product_name,
            product_code: item.product_code,
            product_price: money(item.product_price),
            product_currency: item.product_currency,
            min_quantity: quantity_text(item.min_quantity),
            unit_price: money(item.unit_price),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PriceListItemRequest {
    pub(crate) product_id: String,
    #[schema(value_type = DecimalInputSchema, example = "1350.00")]
    pub(crate) unit_price: Value,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "1")]
    pub(crate) min_quantity: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ReplacePriceListItemsRequest {
    pub(crate) items: Vec<PriceListItemRequest>,
}

impl ReplacePriceListItemsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> ReplacePriceListItemsCommand {
        ReplacePriceListItemsCommand {
            items: self
                .items
                .into_iter()
                .map(|item| PriceListItemInput {
                    product_id: item.product_id,
                    unit_price: decimal_text(item.unit_price).unwrap_or_default(),
                    min_quantity: item.min_quantity.and_then(decimal_text),
                })
                .collect(),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DiscountLimitResponse {
    pub(crate) role_id: String,
    #[schema(example = "sales")]
    pub(crate) role_key: String,
    #[schema(example = "Sales rep")]
    pub(crate) role_name: String,
    #[schema(example = "15")]
    pub(crate) max_discount_percent: Option<String>,
}

impl From<RoleDiscountLimit> for DiscountLimitResponse {
    fn from(limit: RoleDiscountLimit) -> Self {
        Self {
            role_id: limit.role_id.to_string(),
            role_key: limit.role_key,
            role_name: limit.role_name,
            max_discount_percent: limit.max_discount_percent.map(|value| value.to_string()),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct DiscountLimitRequest {
    pub(crate) role_id: String,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "15")]
    pub(crate) max_discount_percent: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct SetDiscountLimitsRequest {
    pub(crate) limits: Vec<DiscountLimitRequest>,
}

impl SetDiscountLimitsRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SetDiscountLimitsCommand {
        SetDiscountLimitsCommand {
            limits: self
                .limits
                .into_iter()
                .map(|limit| DiscountLimitInput {
                    role_id: limit.role_id,
                    max_discount_percent: limit.max_discount_percent.and_then(decimal_text),
                })
                .collect(),
            ip,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = BundlePricing)]
pub(crate) enum BundlePricingSchema {
    Fixed,
    Sum,
}

impl From<BundlePricing> for BundlePricingSchema {
    fn from(value: BundlePricing) -> Self {
        match value {
            BundlePricing::Fixed => Self::Fixed,
            BundlePricing::Sum => Self::Sum,
        }
    }
}

impl From<BundlePricingSchema> for BundlePricing {
    fn from(value: BundlePricingSchema) -> Self {
        match value {
            BundlePricingSchema::Fixed => Self::Fixed,
            BundlePricingSchema::Sum => Self::Sum,
        }
    }
}

fn bundle_quantity_text(value: Decimal) -> String {
    value.normalize().to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BundleComponentResponse {
    pub(crate) product_id: String,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    #[schema(example = "2")]
    pub(crate) quantity: String,
    #[schema(example = "120.00")]
    pub(crate) unit_price: String,
    #[schema(example = "240.00")]
    pub(crate) total: String,
}

impl From<BundleComponent> for BundleComponentResponse {
    fn from(component: BundleComponent) -> Self {
        Self {
            product_id: component.product_id.to_string(),
            name: component.name,
            code: component.code,
            quantity: bundle_quantity_text(component.quantity),
            unit_price: money(component.unit_price),
            total: money(component.unit_price * component.quantity),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProductBundleResponse {
    pub(crate) product_id: String,
    pub(crate) pricing: BundlePricingSchema,
    #[schema(example = "990.00")]
    pub(crate) unit_price: String,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) components: Vec<BundleComponentResponse>,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<BundleDetails> for ProductBundleResponse {
    fn from(details: BundleDetails) -> Self {
        let expansion = details.expansion;
        Self {
            product_id: expansion.product_id.to_string(),
            pricing: expansion.pricing.into(),
            unit_price: money(expansion.unit_price),
            currency: expansion.currency,
            components: expansion.components.into_iter().map(Into::into).collect(),
            updated_at: timestamp(details.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ProductBundleParams {
    pub(crate) price_list_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct BundleComponentRequest {
    pub(crate) product_id: String,
    #[schema(value_type = DecimalInputSchema, example = "2")]
    pub(crate) quantity: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ProductBundleRequest {
    pub(crate) pricing: BundlePricingSchema,
    pub(crate) components: Vec<BundleComponentRequest>,
}

impl ProductBundleRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveBundleCommand {
        SaveBundleCommand {
            pricing: self.pricing.into(),
            components: self
                .components
                .into_iter()
                .map(|component| BundleComponentInput {
                    product_id: component.product_id,
                    quantity: decimal_text(component.quantity).unwrap_or_default(),
                })
                .collect(),
            expected_updated_at,
            ip,
        }
    }
}
