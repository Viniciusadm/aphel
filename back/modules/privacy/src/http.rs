use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    CreatePrivacyRequestBody, CreateRetentionPolicyBody, PrivacyRequestDetailResponse,
    PrivacyRequestListParams, PrivacyRequestResponse, RetentionPolicyResponse,
    RetentionPreviewResponse, RetentionRunListParams, RetentionRunResponse, SubjectSearchRequest,
    SubjectSearchResponse, UpdateRetentionPolicyBody,
};
use crate::service::{requests, retention, search};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::files::content_disposition;
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "privacy";
const DOWNLOAD_CONTENT_SECURITY_POLICY: &str = "default-src 'none'; sandbox";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(search_subject))
        .routes(routes!(catalog))
        .routes(routes!(list_requests, create_request))
        .routes(routes!(get_request))
        .routes(routes!(execute_request))
        .routes(routes!(cancel_request))
        .routes(routes!(download_request))
        .routes(routes!(list_policies, create_policy))
        .routes(routes!(update_policy, delete_policy))
        .routes(routes!(preview_policy))
        .routes(routes!(enable_policy))
        .routes(routes!(disable_policy))
        .routes(routes!(run_policy))
        .routes(routes!(list_runs))
}

fn no_store() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers
}

#[utoipa::path(
    post,
    path = "/privacy/subjects/search",
    tag = TAG,
    operation_id = "privacy_search_subject",
    security(("session_cookie" = [])),
    request_body = SubjectSearchRequest,
    responses(
        (status = 200, description = "Where the data subject appears: records whose personal fields (system or custom, `is_personal`) match the e-mail (lowercase), phone (E.164 or digits) or document (digits) — or contain the text otherwise —, notes that mention the term or belong to those records, their attachments and tasks, and import rows whose raw data contains the term (up to 100 per group). The term goes in the body so it never reaches URLs or access logs; the search is audited (`privacy.subject_searched`) without the term", body = Data<SubjectSearchResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Term shorter than 3 or longer than 255 characters (`invalid_subject_term`)", body = ErrorEnvelope),
    )
)]
async fn search_subject(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<SubjectSearchRequest>,
) -> Result<(HeaderMap, Json<Data<SubjectSearchResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let result = search::search_subject(&ctx, &request.term, client.0.ip).await?;
    Ok((no_store(), Json(Data::new(result.into()))))
}

#[utoipa::path(
    get,
    path = "/privacy/requests",
    tag = TAG,
    operation_id = "privacy_list_requests",
    security(("session_cookie" = [])),
    params(PrivacyRequestListParams, PaginationQuery),
    responses(
        (status = 200, description = "Data subject requests of the organization, most recent first; `overdue` is true for open requests past `due_at`", body = Page<PrivacyRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid status or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_requests(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<PrivacyRequestListParams>,
    pagination: Pagination,
) -> Result<Json<Page<PrivacyRequestResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = requests::list_requests(&ctx, params.into_filter()?, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/privacy/requests",
    tag = TAG,
    operation_id = "privacy_create_request",
    security(("session_cookie" = [])),
    request_body = CreatePrivacyRequestBody,
    responses(
        (status = 201, description = "Request registered as `pending` (`privacy_request.created`) with the selected records (contact, company, deal, activity), notes and import rows that still exist. Only a SHA-256 hash and a masked label of the term are stored. `due_at` defaults to 15 days (LGPD art. 19)", body = Data<PrivacyRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid kind, term, targets or due date (`validation_error`, `invalid_subject_term`), or none of the targets exists (`privacy_targets_empty`)", body = ErrorEnvelope),
    )
)]
async fn create_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreatePrivacyRequestBody>,
) -> Result<(StatusCode, Json<Data<PrivacyRequestResponse>>), ApiError> {
    let command = request.into_command(client.0.ip)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let created = requests::create_request(&ctx, command).await?;
    Ok((StatusCode::CREATED, Json(Data::new(created.into()))))
}

#[utoipa::path(
    get,
    path = "/privacy/requests/{request_id}",
    tag = TAG,
    operation_id = "privacy_get_request",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Privacy request id")),
    responses(
        (status = 200, description = "Request with its targets; record titles are read now (`exists` is false for records already removed)", body = Data<PrivacyRequestDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`privacy_request_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<(HeaderMap, Json<Data<PrivacyRequestDetailResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let detail = requests::get_request(&ctx, &request_id).await?;
    Ok((no_store(), Json(Data::new(detail.into()))))
}

#[utoipa::path(
    post,
    path = "/privacy/requests/{request_id}/execute",
    tag = TAG,
    operation_id = "privacy_execute_request",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Privacy request id")),
    responses(
        (status = 202, description = "Request moved to `processing` and queued (`privacy.request.run`). Export writes a zip with `dados.json` and the attachments; anonymize replaces personal fields with neutral values and removes notes, attachments (and their files) of the records while keeping deals, amounts, stages and dates; delete removes the records for good. Both redact personal values from the history (`audit_logs`, `domain_events`). Finishes with `privacy_request.completed` or `privacy_request.failed`", body = Data<PrivacyRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`privacy_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not pending (`privacy_request_not_pending`)", body = ErrorEnvelope),
    )
)]
async fn execute_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
) -> Result<(StatusCode, Json<Data<PrivacyRequestResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let request = requests::execute_request(&ctx, &request_id, client.0.ip).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(request.into()))))
}

#[utoipa::path(
    post,
    path = "/privacy/requests/{request_id}/cancel",
    tag = TAG,
    operation_id = "privacy_cancel_request",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Privacy request id")),
    responses(
        (status = 200, description = "Pending request cancelled (`privacy_request.cancelled`)", body = Data<PrivacyRequestResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`privacy_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not pending (`privacy_request_not_pending`)", body = ErrorEnvelope),
    )
)]
async fn cancel_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
) -> Result<Json<Data<PrivacyRequestResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let request = requests::cancel_request(&ctx, &request_id, client.0.ip).await?;
    Ok(Json(Data::new(request.into())))
}

#[utoipa::path(
    get,
    path = "/privacy/requests/{request_id}/download",
    tag = TAG,
    operation_id = "privacy_download_request",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Privacy request id")),
    responses(
        (status = 200, description = "Zip of a completed export request (`dados.json` + `anexos/`), served as an attachment. Each download is audited (`privacy_request.downloaded`). The file is removed 30 days after the request finished", content_type = "application/zip"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`privacy_request_not_found`) or without an available file (`privacy_export_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn download_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(request_id): Path<String>,
) -> Result<Response, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let download =
        requests::download_export(&ctx, state.storage.as_ref(), &request_id, client.0.ip).await?;
    let content_type = HeaderValue::from_str(&download.file.mime_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let disposition = HeaderValue::from_str(&content_disposition(&download.file.original_name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, download.file.size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(
            header::CONTENT_SECURITY_POLICY,
            DOWNLOAD_CONTENT_SECURITY_POLICY,
        )
        .body(Body::from_stream(download.content))
        .map_err(|error| AppError::Internal(format!("cannot build download response: {error}")))?;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/privacy/retention-policies",
    tag = TAG,
    operation_id = "privacy_list_retention_policies",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Retention policies of the organization, oldest first", body = Data<Vec<RetentionPolicyResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_policies(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<RetentionPolicyResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let policies = retention::list_policies(&ctx).await?;
    Ok(Json(Data::new(
        policies.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/privacy/retention-policies",
    tag = TAG,
    operation_id = "privacy_create_retention_policy",
    security(("session_cookie" = [])),
    request_body = CreateRetentionPolicyBody,
    responses(
        (status = 201, description = "Policy created disabled (`retention_policy.created`). Actions: `archive_inactive` (people, organizations or deals without updates, tasks or notes for N days), `purge_archived` (deletes for good records archived N days ago, with their files), `purge_audit_logs`, `purge_security_events`, `purge_domain_events` (dispatched events; the timeline loses that history) and `purge_orphan_files` (files no longer referenced, also removed from the storage). The trash keeps its own retention (`trash.purge`)", body = Data<RetentionPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, action, record type or threshold (`validation_error`), or 50 policies already (`retention_policy_limit`)", body = ErrorEnvelope),
    )
)]
async fn create_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateRetentionPolicyBody>,
) -> Result<(StatusCode, Json<Data<RetentionPolicyResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let policy = retention::create_policy(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(policy.into()))))
}

#[utoipa::path(
    patch,
    path = "/privacy/retention-policies/{policy_id}",
    tag = TAG,
    operation_id = "privacy_update_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    request_body = UpdateRetentionPolicyBody,
    responses(
        (status = 200, description = "Policy updated (`retention_policy.updated`). Changing the threshold disables the policy until a new preview is confirmed", body = Data<RetentionPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or threshold (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(policy_id): Path<String>,
    JsonBody(request): JsonBody<UpdateRetentionPolicyBody>,
) -> Result<Json<Data<RetentionPolicyResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let policy =
        retention::update_policy(&ctx, &policy_id, request.into_command(client.0.ip)).await?;
    Ok(Json(Data::new(policy.into())))
}

#[utoipa::path(
    delete,
    path = "/privacy/retention-policies/{policy_id}",
    tag = TAG,
    operation_id = "privacy_delete_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    responses(
        (status = 204, description = "Policy deleted (`retention_policy.deleted`); its runs stay in the history"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    retention::delete_policy(&ctx, &policy_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/privacy/retention-policies/{policy_id}/preview",
    tag = TAG,
    operation_id = "privacy_preview_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    responses(
        (status = 200, description = "How many items the policy would affect now. Enabling or running the policy requires a preview from the last hour, made after its last change", body = Data<RetentionPreviewResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
    )
)]
async fn preview_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(policy_id): Path<String>,
) -> Result<Json<Data<RetentionPreviewResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let preview = retention::preview_policy(&ctx, &policy_id).await?;
    Ok(Json(Data::new(preview.into())))
}

#[utoipa::path(
    post,
    path = "/privacy/retention-policies/{policy_id}/enable",
    tag = TAG,
    operation_id = "privacy_enable_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    responses(
        (status = 200, description = "Policy enabled (`retention_policy.enabled`); the `privacy.retention` job runs it daily at 04:00 UTC", body = Data<RetentionPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "No recent preview (`retention_preview_required`)", body = ErrorEnvelope),
    )
)]
async fn enable_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(policy_id): Path<String>,
) -> Result<Json<Data<RetentionPolicyResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let policy = retention::set_policy_enabled(&ctx, &policy_id, true, client.0.ip).await?;
    Ok(Json(Data::new(policy.into())))
}

#[utoipa::path(
    post,
    path = "/privacy/retention-policies/{policy_id}/disable",
    tag = TAG,
    operation_id = "privacy_disable_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    responses(
        (status = 200, description = "Policy disabled (`retention_policy.disabled`)", body = Data<RetentionPolicyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
    )
)]
async fn disable_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(policy_id): Path<String>,
) -> Result<Json<Data<RetentionPolicyResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let policy = retention::set_policy_enabled(&ctx, &policy_id, false, client.0.ip).await?;
    Ok(Json(Data::new(policy.into())))
}

#[utoipa::path(
    post,
    path = "/privacy/retention-policies/{policy_id}/run",
    tag = TAG,
    operation_id = "privacy_run_retention_policy",
    security(("session_cookie" = [])),
    params(("policy_id" = String, Path, description = "Retention policy id")),
    responses(
        (status = 202, description = "Manual run queued (`retention_run.requested`, job `privacy.retention.run`), even for a disabled policy. Finishes with `retention_run.completed` or `retention_run.failed`", body = Data<RetentionRunResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Policy not found (`retention_policy_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "No recent preview (`retention_preview_required`)", body = ErrorEnvelope),
    )
)]
async fn run_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(policy_id): Path<String>,
) -> Result<(StatusCode, Json<Data<RetentionRunResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let run = retention::request_run(&ctx, &policy_id, client.0.ip).await?;
    Ok((StatusCode::ACCEPTED, Json(Data::new(run.into()))))
}

#[utoipa::path(
    get,
    path = "/privacy/retention-runs",
    tag = TAG,
    operation_id = "privacy_list_retention_runs",
    security(("session_cookie" = [])),
    params(RetentionRunListParams, PaginationQuery),
    responses(
        (status = 200, description = "Retention runs (scheduled and manual), most recent first, with the number of affected items, removed files and errors", body = Page<RetentionRunResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid policy id or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_runs(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<RetentionRunListParams>,
    pagination: Pagination,
) -> Result<Json<Page<RetentionRunResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = retention::list_runs(&ctx, params.into_filter()?, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/privacy/catalog",
    tag = TAG,
    operation_id = "privacy_catalog",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Available privacy sources and entity policies", body = Data<crate::dto::PrivacyCatalogResponse>),
        (status = 403, description = "Missing privacy.manage", body = ErrorEnvelope),
    )
)]
async fn catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<(HeaderMap, Json<Data<crate::dto::PrivacyCatalogResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    crate::service::require_privacy(ctx.scope)?;
    let mut entities = Vec::new();
    for definition in ctx.registry().definitions() {
        let personal_data = definition.capabilities.has_trait("privacy.personal_data");
        let retention = crate::domain::supports_retention(definition);
        if !(personal_data || retention)
            || !aphel_sdk::privacy::sql::table_exists(&ctx, &definition.table).await?
        {
            continue;
        }
        entities.push(crate::dto::PrivacyEntityResponse {
            record_type: definition.record_type.as_str().into(),
            labels: definition
                .capabilities
                .plural_labels_value()
                .unwrap_or_else(|| definition.record_type.as_str().to_owned().into())
                .0,
            personal_data,
            retention,
        });
    }
    entities.sort_by(|left, right| left.record_type.cmp(&right.record_type));
    let sources = crate::service::sources::available(&ctx)
        .await?
        .into_iter()
        .map(|source| {
            crate::dto::PrivacySourceResponse::from(crate::domain::PrivacySourceInfo {
                key: source.key().into(),
                labels: source.labels().0,
            })
        })
        .collect();
    Ok((
        no_store(),
        Json(Data::new(crate::dto::PrivacyCatalogResponse {
            entities,
            sources,
        })),
    ))
}
