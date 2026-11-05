use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::domain::{ProductDetails, ProductInput, Recurrence, SaveProductCommand, money};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, RecordTypeHandler, optional_record, values_request};
use aphel_sdk::money::DecimalInputSchema;
use aphel_sdk::platform::api::dto::common::{
    Data, ErrorEnvelope, Page, QuickSearchParams, decimal_text, double_option, timestamp,
};
use aphel_sdk::platform::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::field_permissions::{redacted, redacted_items};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::shared::RecordType;

pub use aphel_sdk::catalog_items::API_TAG as TAG;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_products, create_product))
        .routes(routes!(get_product, update_product, delete_product))
}

#[utoipa::path(
    get,
    path = "/catalog-items",
    tag = TAG,
    operation_id = "products_list",
    security(("session_cookie" = [])),
    description = "Lists the product and service catalog of the organization (permission `catalog_items.read` with organization scope; the catalog has no owner, so narrower scopes see nothing). Filters accept the JSON AST in `filter` or `filter[field][operator]=value` (`filter[is_active][eq]=true`, `filter[recurrence][in]=monthly,yearly`, `filter[price][gte]=100`). `sort=field,-other` (up to 3); without `sort`, newest first. `search` (or its alias `q`) matches name and code.",
    params(RecordQueryParams, QuickSearchParams, PaginationQuery),
    responses(
        (status = 200, description = "Page of products with their custom fields", body = Page<ProductResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search or pagination (`validation_error` with `details.fields`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_products(
    State(state): State<AppState>,
    tenant: AuthContext,
    RecordListQuery(mut query): RecordListQuery,
    Query(quick): Query<QuickSearchParams>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    quick.merge_into(&mut query.search);
    let listing = service::list(&Ctx::from_state(&state, &tenant.scope).await?, query).await?;
    let items: Vec<ProductResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &RecordType::PRODUCT, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/catalog-items",
    tag = TAG,
    operation_id = "products_create",
    security(("session_cookie" = [])),
    description = "Creates a product or service (permission `catalog_items.create`) and publishes `product.created`. `price` is a decimal with two places (default `0.00`), `currency` defaults to the organization currency, `recurrence` (`none`, `monthly`, `quarterly`, `yearly`) is informative only (no billing) and `is_active` defaults to `true`. `code` is optional and unique among the products of the organization. Custom fields go in `custom_fields`.",
    request_body = ProductRequest,
    responses(
        (status = 201, description = "Product created", body = Data<ProductResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields, such as a repeated `code` (`validation_error`, fields such as `name`, `price` or `custom_fields.<key>`), or fields the role cannot write (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn create_product(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ProductRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let product = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted(
                &state,
                &tenant.scope,
                &RecordType::PRODUCT,
                ProductResponse::from(product),
            )
            .await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/catalog-items/{product_id}",
    tag = TAG,
    operation_id = "products_get",
    security(("session_cookie" = [])),
    description = "Returns a product with its custom fields (permission `catalog_items.read`). Inactive products are still returned.",
    params(("product_id" = String, Path, description = "Product id")),
    responses(
        (status = 200, description = "The product", body = Data<ProductResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product not found or deleted (`product_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_product(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(product_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let product = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &product_id).await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::PRODUCT,
            ProductResponse::from(product),
        )
        .await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/catalog-items/{product_id}",
    tag = TAG,
    operation_id = "products_update",
    security(("session_cookie" = [])),
    description = "Partially updates a product (permission `catalog_items.update`) and publishes `product.updated` with the changes. Omitted properties stay unchanged and `null` clears an optional one (`price: null` means `0.00`). Deactivating a product (`is_active: false`) only hides it from new deal items; existing items keep their copied name, price and unit. Send `If-Match` with the `updated_at` you read to avoid overwriting someone else's change.",
    params(
        ("product_id" = String, Path, description = "Product id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the product as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = ProductRequest,
    responses(
        (status = 200, description = "Product updated", body = Data<ProductResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product not found or deleted (`product_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The product changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields or malformed `If-Match` (`validation_error`), or fields the role cannot write (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn update_product(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(product_id): Path<String>,
    JsonBody(request): JsonBody<ProductRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let product = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &product_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::PRODUCT,
            ProductResponse::from(product),
        )
        .await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/catalog-items/{product_id}",
    tag = TAG,
    operation_id = "products_delete",
    security(("session_cookie" = [])),
    params(("product_id" = String, Path, description = "Product id")),
    responses(
        (status = 204, description = "Product deleted (soft delete, it does not go to the trash); publishes `product.deleted`. Deal items created from it keep their copied name, price and unit"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `catalog_items.delete` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Product not found or already deleted (`product_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_product(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(product_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &product_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

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

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct ProductRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Monthly consulting")]
    pub(crate) name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "CONS-01")]
    pub(crate) code: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "1500.00")]
    pub(crate) price: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "BRL")]
    pub(crate) currency: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "hour")]
    pub(crate) unit: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<RecurrenceSchema>)]
    pub(crate) recurrence: Option<Option<RecurrenceSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 12)]
    pub(crate) term_months: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>)]
    pub(crate) is_active: Option<Option<bool>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
}

impl ProductRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveProductCommand {
        SaveProductCommand {
            input: ProductInput {
                name: self.name,
                code: self.code,
                description: self.description,
                price: self.price.map(|value| value.and_then(decimal_text)),
                currency: self.currency,
                unit: self.unit,
                recurrence: self.recurrence.map(|value| {
                    value.map(|recurrence| Recurrence::from(recurrence).as_str().to_owned())
                }),
                term_months: self.term_months.map(|value| value.and_then(decimal_text)),
                is_active: self.is_active,
            },
            custom_fields: self.custom_fields.unwrap_or_default(),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProductResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    pub(crate) description: Option<String>,
    #[schema(example = "1500.00")]
    pub(crate) price: String,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) unit: Option<String>,
    pub(crate) recurrence: RecurrenceSchema,
    #[schema(example = 12)]
    pub(crate) term_months: Option<u16>,
    pub(crate) is_active: bool,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
}

impl From<ProductDetails> for ProductResponse {
    fn from(details: ProductDetails) -> Self {
        let product = details.product;
        let data = product.data;
        Self {
            id: product.id.to_string(),
            name: data.name,
            code: data.code,
            description: data.description,
            price: money(data.price),
            currency: data.currency,
            unit: data.unit,
            recurrence: data.recurrence.into(),
            term_months: data.term_months,
            is_active: data.is_active,
            created_by: product.created_by.map(|id| id.into_inner()),
            created_at: timestamp(product.created_at),
            updated_at: timestamp(product.updated_at),
            custom_fields: details.custom_fields,
        }
    }
}

pub(crate) struct ProductHandler;

#[async_trait]
impl RecordTypeHandler for ProductHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        "catalog_items.item"
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let command = values_request::<ProductRequest>(values)?.into_command(None, None);
        let product =
            service::create(&Ctx::from_state(ctx.state, ctx.scope).await?, command).await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::PRODUCT,
            ProductResponse::from(product),
        )
        .await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let command = values_request::<ProductRequest>(values)?.into_command(None, None);
        let product = service::update(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            command,
        )
        .await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::PRODUCT,
            ProductResponse::from(product),
        )
        .await
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::delete(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let module_ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let Some(product) = optional_record(service::get(&module_ctx, &id.to_string()).await)?
        else {
            return Ok(None);
        };
        Ok(Some(
            redacted(
                ctx.state,
                ctx.scope,
                &RecordType::PRODUCT,
                ProductResponse::from(product),
            )
            .await?,
        ))
    }
}
