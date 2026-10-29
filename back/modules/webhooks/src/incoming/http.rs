use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::incoming::dto::{
    CreateIncomingWebhookRequest, HookReceivedResponse, IncomingActionTargetResponse,
    IncomingWebhookRequestDetailResponse, IncomingWebhookRequestListParams,
    IncomingWebhookRequestResponse, IncomingWebhookResponse, IssuedIncomingWebhookResponse,
    UpdateIncomingWebhookRequest,
};
use crate::incoming::service;
use crate::incoming::{
    DELIVERY_HEADER, IncomingDelivery, MAX_BODY_BYTES, ReceiveFailure, SIGNATURE_HEADER,
    logged_headers,
};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorBody, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "incoming-webhooks";
pub(crate) const RECEIVE_LIMIT_PER_MINUTE: u32 = 120;

static RECEIVE_LIMITER: LazyLock<SharedRateLimiter> = LazyLock::new(|| {
    SharedRateLimiter::new(
        "webhooks.incoming",
        RECEIVE_LIMIT_PER_MINUTE,
        Duration::from_secs(60),
    )
});

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_incoming_webhooks, create_incoming_webhook))
        .routes(routes!(list_incoming_webhook_targets))
        .routes(routes!(
            get_incoming_webhook,
            update_incoming_webhook,
            delete_incoming_webhook
        ))
        .routes(routes!(regenerate_incoming_webhook_token))
        .routes(routes!(
            rotate_incoming_webhook_secret,
            remove_incoming_webhook_secret
        ))
        .routes(routes!(list_incoming_webhook_requests))
        .routes(routes!(get_incoming_webhook_request))
        .routes(routes!(receive_hook))
}

#[utoipa::path(
    get,
    path = "/incoming-webhooks",
    tag = TAG,
    operation_id = "incoming_webhooks_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Incoming webhooks of the organization, newest first (needs `settings.manage`). The URL token and the signing secret are never returned again; `token_prefix` identifies the token and `secret_configured` tells whether requests must be signed", body = Page<IncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_incoming_webhooks(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<IncomingWebhookResponse>>, ApiError> {
    let webhooks = service::list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    let total = webhooks.len() as u64;
    Ok(Json(Page::new(
        webhooks
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
    path = "/incoming-webhooks",
    tag = TAG,
    operation_id = "incoming_webhooks_create",
    security(("session_cookie" = [])),
    request_body = CreateIncomingWebhookRequest,
    responses(
        (status = 201, description = "Incoming webhook created (`incoming_webhook.created`), needs `settings.manage`. `token` and `path` (`/api/v1/hooks/<token>`) are shown only in this response; with `generate_signing_secret`, `signing_secret` too, and every request must then carry `X-Aphel-Signature`. The actions run as the current user, with their permissions at the time of each request", body = Data<IssuedIncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, tolerance or actions: unknown type, unknown field for the record type, malformed `$.path`, missing required field (`title`, `first_name`), stage outside the pipeline or inactive activity type, with the offending `actions.<i>...` paths (`validation_error`), or too many webhooks (`too_many_incoming_webhooks`)", body = ErrorEnvelope),
    )
)]
async fn create_incoming_webhook(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateIncomingWebhookRequest>,
) -> Result<(StatusCode, Json<Data<IssuedIncomingWebhookResponse>>), ApiError> {
    let issued = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(issued.into()))))
}

#[utoipa::path(
    get,
    path = "/incoming-webhooks/targets",
    tag = TAG,
    operation_id = "incoming_webhooks_targets",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Fields each action type can map, system and active custom fields of the record type, with `is_required` marking fields the action must map (needs `settings.manage`)", body = Data<Vec<IncomingActionTargetResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_incoming_webhook_targets(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<IncomingActionTargetResponse>>>, ApiError> {
    let targets = service::targets(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        targets.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/incoming-webhooks/{webhook_id}",
    tag = TAG,
    operation_id = "incoming_webhooks_get",
    security(("session_cookie" = [])),
    params(("webhook_id" = String, Path, description = "Incoming webhook id")),
    responses(
        (status = 200, description = "Incoming webhook (needs `settings.manage`)", body = Data<IncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_incoming_webhook(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(webhook_id): Path<String>,
) -> Result<Json<Data<IncomingWebhookResponse>>, ApiError> {
    let webhook = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &webhook_id).await?;
    Ok(Json(Data::new(webhook.into())))
}

#[utoipa::path(
    patch,
    path = "/incoming-webhooks/{webhook_id}",
    tag = TAG,
    operation_id = "incoming_webhooks_update",
    security(("session_cookie" = [])),
    params(
        ("webhook_id" = String, Path, description = "Incoming webhook id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the webhook as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = UpdateIncomingWebhookRequest,
    responses(
        (status = 200, description = "Incoming webhook updated (`incoming_webhook.updated`); omitted properties stay unchanged and `actions` replaces the whole list. Needs `settings.manage`", body = Data<IncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The webhook changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, tolerance or actions, or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_incoming_webhook(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(webhook_id): Path<String>,
    JsonBody(request): JsonBody<UpdateIncomingWebhookRequest>,
) -> Result<Json<Data<IncomingWebhookResponse>>, ApiError> {
    let webhook = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(webhook.into())))
}

#[utoipa::path(
    delete,
    path = "/incoming-webhooks/{webhook_id}",
    tag = TAG,
    operation_id = "incoming_webhooks_delete",
    security(("session_cookie" = [])),
    params(("webhook_id" = String, Path, description = "Incoming webhook id")),
    responses(
        (status = 204, description = "Incoming webhook, its signing secret and its request log deleted (`incoming_webhook.deleted`); its URL answers 404 from now on. Needs `settings.manage`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_incoming_webhook(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(webhook_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/incoming-webhooks/{webhook_id}/regenerate-token",
    tag = TAG,
    operation_id = "incoming_webhooks_regenerate_token",
    security(("session_cookie" = [])),
    params(("webhook_id" = String, Path, description = "Incoming webhook id")),
    responses(
        (status = 200, description = "New URL token issued (`incoming_webhook.token_regenerated`); the previous URL stops working immediately. `token` and `path` are shown only in this response. Needs `settings.manage`", body = Data<IssuedIncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
    )
)]
async fn regenerate_incoming_webhook_token(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(webhook_id): Path<String>,
) -> Result<Json<Data<IssuedIncomingWebhookResponse>>, ApiError> {
    let issued = service::regenerate_token(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(issued.into())))
}

#[utoipa::path(
    post,
    path = "/incoming-webhooks/{webhook_id}/signing-secret",
    tag = TAG,
    operation_id = "incoming_webhooks_rotate_secret",
    security(("session_cookie" = [])),
    params(("webhook_id" = String, Path, description = "Incoming webhook id")),
    responses(
        (status = 200, description = "New signing secret generated (`incoming_webhook.signing_secret_rotated`), replacing the previous one; from now on every request must carry a valid `X-Aphel-Signature`. `signing_secret` is shown only in this response. Needs `settings.manage`", body = Data<IssuedIncomingWebhookResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
        (status = 503, description = "The secret vault is not configured (`configuration_error`, `APP_ENCRYPTION_KEY`)", body = ErrorEnvelope),
    )
)]
async fn rotate_incoming_webhook_secret(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(webhook_id): Path<String>,
) -> Result<Json<Data<IssuedIncomingWebhookResponse>>, ApiError> {
    let issued = service::rotate_signing_secret(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(issued.into())))
}

#[utoipa::path(
    delete,
    path = "/incoming-webhooks/{webhook_id}/signing-secret",
    tag = TAG,
    operation_id = "incoming_webhooks_remove_secret",
    security(("session_cookie" = [])),
    params(("webhook_id" = String, Path, description = "Incoming webhook id")),
    responses(
        (status = 204, description = "Signing secret removed (`incoming_webhook.signing_secret_removed`); requests are accepted without `X-Aphel-Signature`. Needs `settings.manage`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
    )
)]
async fn remove_incoming_webhook_secret(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(webhook_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::remove_signing_secret(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/incoming-webhooks/{webhook_id}/requests",
    tag = TAG,
    operation_id = "incoming_webhooks_requests",
    security(("session_cookie" = [])),
    params(
        ("webhook_id" = String, Path, description = "Incoming webhook id"),
        IncomingWebhookRequestListParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Requests received by the webhook in the last 30 days, newest first, with the result of each action (needs `settings.manage`). Headers and body only come in the detail", body = Page<IncomingWebhookRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Incoming webhook not found (`incoming_webhook_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `status` or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_incoming_webhook_requests(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(webhook_id): Path<String>,
    params: Result<Query<IncomingWebhookRequestListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<IncomingWebhookRequestResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let status = params.status.map(Into::into);
    let listing = service::list_requests(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        status,
        ListWindow {
            offset: pagination.offset(),
            limit: pagination.limit(),
        },
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/incoming-webhooks/{webhook_id}/requests/{request_id}",
    tag = TAG,
    operation_id = "incoming_webhooks_request_get",
    security(("session_cookie" = [])),
    params(
        ("webhook_id" = String, Path, description = "Incoming webhook id"),
        ("request_id" = String, Path, description = "Request id (`request_id` answered to the sender)"),
    ),
    responses(
        (status = 200, description = "Request detail: allow-listed headers (never `Authorization`, `Cookie` or `X-Aphel-Signature`), body truncated to 8 KB and the result of each action (needs `settings.manage`)", body = Data<IncomingWebhookRequestDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Webhook or request not found (`incoming_webhook_not_found`, `incoming_webhook_request_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_incoming_webhook_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((webhook_id, request_id)): Path<(String, String)>,
) -> Result<Json<Data<IncomingWebhookRequestDetailResponse>>, ApiError> {
    let request = service::get_request(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &webhook_id,
        &request_id,
    )
    .await?;
    Ok(Json(Data::new(request.into())))
}

fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn failure_response(request_id: String, failure: ReceiveFailure) -> Response {
    let status =
        StatusCode::from_u16(failure.http_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut details = json!({ "request_id": request_id });
    if !failure.fields.is_empty() {
        details["fields"] = json!(
            failure
                .fields
                .iter()
                .map(|field| json!({ "field": field.field, "message": field.message }))
                .collect::<Vec<_>>()
        );
    }
    (
        status,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: failure.code.to_owned(),
                message: failure.message,
                details: Some(details),
            },
        }),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/hooks/{token}",
    tag = TAG,
    operation_id = "incoming_webhooks_receive",
    description = "Public receiver of an incoming webhook: the secret URL token authenticates the request (no session, API token or `X-Requested-With`). Accepts `application/json` (object or array) and `application/x-www-form-urlencoded` (converted to an object; repeated keys become arrays) up to 256 KB. When the webhook has a signing secret, `X-Aphel-Signature: t=<unix>,v1=<hex hmac-sha256 of \"<t>.<body>\">` is required and `t` must be within the configured window (default 5 min). `X-Aphel-Delivery` (or, for signed requests, the signature itself) is remembered for 24 h and a repetition answers 409. The configured actions run in order as the user who created the webhook; every request is logged and answers its `request_id` (also in `error.details.request_id` on failures).",
    params(("token" = String, Path, description = "Secret token of the webhook URL (`whk_...`)")),
    request_body(content = Object, description = "JSON object/array or URL-encoded form", content_type = "application/json"),
    responses(
        (status = 200, description = "Every action ran (`status = succeeded`)", body = Data<HookReceivedResponse>),
        (status = 401, description = "Missing, malformed or wrong signature, or timestamp outside the window (`invalid_webhook_signature`)", body = ErrorEnvelope),
        (status = 403, description = "Webhook disabled (`incoming_webhook_disabled`), its creator is no longer an active member (`incoming_webhook_owner_unavailable`) or lacks the permission for an action (`incoming_webhook_permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown token (`incoming_webhook_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Delivery already received in the last 24 h (`webhook_replayed`)", body = ErrorEnvelope),
        (status = 413, description = "Body larger than 256 KB (`webhook_payload_too_large`)", body = ErrorEnvelope),
        (status = 429, description = "More than 120 requests in the last minute for this webhook URL (`rate_limited`, with `Retry-After`); the request is not logged", body = ErrorEnvelope),
        (status = 422, description = "Unsupported content type or malformed body (`invalid_webhook_payload`), or an action rejected the mapped values (`webhook_mapping_failed`, with `details.fields` as `actions.<i>.<field>`); earlier actions are kept and the log shows a partial result", body = ErrorEnvelope),
    )
)]
async fn receive_hook(
    State(state): State<AppState>,
    Path(token): Path<String>,
    client: ClientInfo,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    RECEIVE_LIMITER
        .check(
            &token,
            "too many requests for this webhook; wait a little and try again",
        )
        .await?;
    let declared_length = header_text(&headers, header::CONTENT_LENGTH.as_str())
        .and_then(|value| value.trim().parse::<u64>().ok());
    let body = axum::body::to_bytes(body, MAX_BODY_BYTES).await.ok();
    let body_bytes = body
        .as_ref()
        .map(|bytes| bytes.len() as u64)
        .or(declared_length)
        .unwrap_or(0);
    let delivery = IncomingDelivery {
        token,
        content_type: header_text(&headers, header::CONTENT_TYPE.as_str()),
        signature: header_text(&headers, SIGNATURE_HEADER),
        delivery_id: header_text(&headers, DELIVERY_HEADER),
        headers: logged_headers(
            headers
                .iter()
                .filter_map(|(name, value)| Some((name.as_str(), value.to_str().ok()?))),
        ),
        body: body.map(|bytes| bytes.to_vec()),
        body_bytes,
        ip: client.0.ip,
    };
    let scope = AccessScope::system();
    let outcome = service::receive(&Ctx::from_state(&state, &scope).await?, delivery).await?;
    let request_id = outcome.request_id.to_string();
    Ok(match outcome.failure.clone() {
        Some(failure) => failure_response(request_id, failure),
        None => (
            StatusCode::OK,
            Json(Data::new(HookReceivedResponse::from(&outcome))),
        )
            .into_response(),
    })
}
