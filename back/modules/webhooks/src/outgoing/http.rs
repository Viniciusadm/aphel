use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::outgoing::dto::{
    CreateWebhookEndpointRequest, UpdateWebhookEndpointRequest, WebhookDeliveryDetailResponse,
    WebhookDeliveryListParams, WebhookDeliveryResponse, WebhookEndpointResponse,
    WebhookEndpointWithSecretResponse,
};
use crate::outgoing::service;
use crate::outgoing::{DeliveryListFilter, delivery_status_filter};
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "webhooks";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_webhook_endpoints, create_webhook_endpoint))
        .routes(routes!(
            get_webhook_endpoint,
            update_webhook_endpoint,
            delete_webhook_endpoint
        ))
        .routes(routes!(rotate_webhook_secret))
        .routes(routes!(test_webhook_endpoint))
        .routes(routes!(list_webhook_deliveries))
        .routes(routes!(get_webhook_delivery))
        .routes(routes!(redeliver_webhook_delivery))
}

#[utoipa::path(
    get,
    path = "/webhook-endpoints",
    tag = TAG,
    operation_id = "webhook_endpoints_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Outgoing webhook endpoints of the organization, newest first. Needs `settings.manage`. The signing secret is never returned here; `secret_configured` tells whether one is stored", body = Page<WebhookEndpointResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_webhook_endpoints(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<WebhookEndpointResponse>>, ApiError> {
    let endpoints = service::list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    let total = endpoints.len() as u64;
    Ok(Json(Page::new(
        endpoints
            .into_iter()
            .skip(usize::try_from(pagination.offset()).unwrap_or(usize::MAX))
            .take(usize::try_from(pagination.limit()).unwrap_or(usize::MAX))
            .map(Into::into)
            .collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/webhook-endpoints",
    tag = TAG,
    operation_id = "webhook_endpoints_create",
    security(("session_cookie" = [])),
    request_body = CreateWebhookEndpointRequest,
    responses(
        (status = 201, description = "Endpoint created (`webhook_endpoint.created`). `events` lists public event types from `GET /events/catalog`, or `[\"*\"]` for all of them. `signing_secret` (`whsec_…`) is shown only in this response; use it to verify `X-Aphel-Signature`. The URL must be http(s), resolve to a public address and, when https is required, use https", body = Data<WebhookEndpointWithSecretResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid URL, description or events (`validation_error`), URL pointing to a private or reserved address or not https (`webhook_url_not_allowed`), or too many endpoints (`too_many_webhook_endpoints`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured, so the signing secret cannot be stored", body = ErrorEnvelope),
    )
)]
async fn create_webhook_endpoint(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateWebhookEndpointRequest>,
) -> Result<(StatusCode, Json<Data<WebhookEndpointWithSecretResponse>>), ApiError> {
    let created = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(created.into()))))
}

#[utoipa::path(
    get,
    path = "/webhook-endpoints/{endpoint_id}",
    tag = TAG,
    operation_id = "webhook_endpoints_get",
    security(("session_cookie" = [])),
    params(("endpoint_id" = String, Path, description = "Webhook endpoint id")),
    responses(
        (status = 200, description = "The endpoint. Needs `settings.manage`", body = Data<WebhookEndpointResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_webhook_endpoint(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(endpoint_id): Path<String>,
) -> Result<Json<Data<WebhookEndpointResponse>>, ApiError> {
    let endpoint =
        service::get(&Ctx::from_state(&state, &tenant.scope).await?, &endpoint_id).await?;
    Ok(Json(Data::new(endpoint.into())))
}

#[utoipa::path(
    patch,
    path = "/webhook-endpoints/{endpoint_id}",
    tag = TAG,
    operation_id = "webhook_endpoints_update",
    security(("session_cookie" = [])),
    params(
        ("endpoint_id" = String, Path, description = "Webhook endpoint id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the endpoint as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = UpdateWebhookEndpointRequest,
    responses(
        (status = 200, description = "Endpoint updated (`webhook_endpoint.updated`); omitted properties stay unchanged. `is_active: true` re-enables an endpoint disabled by hand or after consecutive failures and resets the failure counter", body = Data<WebhookEndpointResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The endpoint changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid URL, description or events, or malformed `If-Match` (`validation_error`), or URL not allowed (`webhook_url_not_allowed`)", body = ErrorEnvelope),
    )
)]
async fn update_webhook_endpoint(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(endpoint_id): Path<String>,
    JsonBody(request): JsonBody<UpdateWebhookEndpointRequest>,
) -> Result<Json<Data<WebhookEndpointResponse>>, ApiError> {
    let endpoint = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(endpoint.into())))
}

#[utoipa::path(
    delete,
    path = "/webhook-endpoints/{endpoint_id}",
    tag = TAG,
    operation_id = "webhook_endpoints_delete",
    security(("session_cookie" = [])),
    params(("endpoint_id" = String, Path, description = "Webhook endpoint id")),
    responses(
        (status = 204, description = "Endpoint, its signing secret and its delivery log deleted (`webhook_endpoint.deleted`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_webhook_endpoint(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(endpoint_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/webhook-endpoints/{endpoint_id}/rotate-secret",
    tag = TAG,
    operation_id = "webhook_endpoints_rotate_secret",
    security(("session_cookie" = [])),
    params(("endpoint_id" = String, Path, description = "Webhook endpoint id")),
    responses(
        (status = 200, description = "New signing secret generated (`webhook_endpoint.secret_rotated`) and shown only in this response. The old secret stops being used immediately, also for retries still pending", body = Data<WebhookEndpointWithSecretResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured", body = ErrorEnvelope),
    )
)]
async fn rotate_webhook_secret(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(endpoint_id): Path<String>,
) -> Result<Json<Data<WebhookEndpointWithSecretResponse>>, ApiError> {
    let rotated = service::rotate_secret(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(rotated.into())))
}

#[utoipa::path(
    post,
    path = "/webhook-endpoints/{endpoint_id}/test",
    tag = TAG,
    operation_id = "webhook_endpoints_test",
    security(("session_cookie" = [])),
    params(("endpoint_id" = String, Path, description = "Webhook endpoint id")),
    responses(
        (status = 201, description = "A synthetic `webhook.test` event was sent to the endpoint right away, in a single attempt with a 10 s limit (`webhook_endpoint.tested`). The response is the logged delivery with its result (`status` `succeeded` or `failed`, `response_status`, `error`); it appears in the delivery log and does not count towards automatic disabling. A disabled endpoint gets a `failed` delivery without being called", body = Data<WebhookDeliveryResponse>),
        (status = 503, description = "Outgoing webhooks are not configured (`configuration_error`)", body = ErrorEnvelope),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
    )
)]
async fn test_webhook_endpoint(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(endpoint_id): Path<String>,
) -> Result<(StatusCode, Json<Data<WebhookDeliveryResponse>>), ApiError> {
    let delivery = service::send_test(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        client.0.ip,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(delivery.into()))))
}

#[utoipa::path(
    get,
    path = "/webhook-endpoints/{endpoint_id}/deliveries",
    tag = TAG,
    operation_id = "webhook_deliveries_list",
    security(("session_cookie" = [])),
    params(
        ("endpoint_id" = String, Path, description = "Webhook endpoint id"),
        WebhookDeliveryListParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Delivery log of the endpoint, newest first, optionally filtered by `status` (`pending`, `succeeded`, `failed`). Deliveries are kept for the retention set in the organization settings (30 days by default)", body = Page<WebhookDeliveryResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint not found (`webhook_endpoint_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `status` or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_webhook_deliveries(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(endpoint_id): Path<String>,
    Query(params): Query<WebhookDeliveryListParams>,
    pagination: Pagination,
) -> Result<Json<Page<WebhookDeliveryResponse>>, ApiError> {
    let filter = DeliveryListFilter {
        status: delivery_status_filter(params.status.as_deref())?,
        offset: pagination.offset(),
        limit: pagination.limit(),
    };
    let (deliveries, total) = service::list_deliveries(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        filter,
    )
    .await?;
    Ok(Json(Page::new(
        deliveries.into_iter().map(Into::into).collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/webhook-endpoints/{endpoint_id}/deliveries/{delivery_id}",
    tag = TAG,
    operation_id = "webhook_deliveries_get",
    security(("session_cookie" = [])),
    params(
        ("endpoint_id" = String, Path, description = "Webhook endpoint id"),
        ("delivery_id" = String, Path, description = "Delivery id"),
    ),
    responses(
        (status = 200, description = "The delivery with the exact body sent (`request_body`) and the start of the response (up to 2 KB)", body = Data<WebhookDeliveryDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint or delivery not found (`webhook_endpoint_not_found`, `webhook_delivery_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_webhook_delivery(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((endpoint_id, delivery_id)): Path<(String, String)>,
) -> Result<Json<Data<WebhookDeliveryDetailResponse>>, ApiError> {
    let delivery = service::get_delivery(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        &delivery_id,
    )
    .await?;
    Ok(Json(Data::new(delivery.into())))
}

#[utoipa::path(
    post,
    path = "/webhook-endpoints/{endpoint_id}/deliveries/{delivery_id}/redeliver",
    tag = TAG,
    operation_id = "webhook_deliveries_redeliver",
    security(("session_cookie" = [])),
    params(
        ("endpoint_id" = String, Path, description = "Webhook endpoint id"),
        ("delivery_id" = String, Path, description = "Delivery to send again"),
    ),
    responses(
        (status = 201, description = "A new delivery with the same body and event `id` was queued (`webhook_delivery.redelivered`), signed with the current secret and sent in a single attempt", body = Data<WebhookDeliveryResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Endpoint or delivery not found (`webhook_endpoint_not_found`, `webhook_delivery_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The endpoint is disabled (`webhook_endpoint_disabled`)", body = ErrorEnvelope),
    )
)]
async fn redeliver_webhook_delivery(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((endpoint_id, delivery_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<Data<WebhookDeliveryResponse>>), ApiError> {
    let delivery = service::redeliver(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &endpoint_id,
        &delivery_id,
        client.0.ip,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(delivery.into()))))
}
