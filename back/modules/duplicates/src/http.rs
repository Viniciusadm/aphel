use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    DuplicateCandidateResponse, DuplicateCheckRequest, DuplicateListParams, DuplicateMatchResponse,
    DuplicateRulesParams, DuplicateRulesResponse, DuplicateTypeResponse, MergeRequest,
    MergeResponse, ScanStatusResponse, UpdateDuplicateRulesRequest, match_response,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "duplicates";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_duplicates))
        .routes(routes!(list_duplicate_types))
        .routes(routes!(get_rules, update_rules))
        .routes(routes!(check_duplicates))
        .routes(routes!(scan_duplicates, scan_status))
        .routes(routes!(merge_records))
        .routes(routes!(get_duplicate))
        .routes(routes!(dismiss_duplicate))
}

#[utoipa::path(
    get,
    path = "/duplicates",
    tag = TAG,
    operation_id = "duplicates_list",
    security(("session_cookie" = [])),
    params(DuplicateListParams, PaginationQuery),
    responses(
        (status = 200, description = "Duplicate candidates of the record type (default `contact`) with the status (default `pending`), newest first. Only pairs whose two records the user can read are listed", body = Page<DuplicateCandidateResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type, status or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_duplicates(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<DuplicateListParams>,
    pagination: Pagination,
) -> Result<Json<Page<DuplicateCandidateResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, params.into(), window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/duplicates/types",
    tag = TAG,
    operation_id = "duplicates_types",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Record types with duplicate detection that the user can read: people, organizations and every type that declares identifier columns (`label_aliases`, e.g. products by code) or is a custom object. Each comes with its rules (`identifier` and `name` for generic types), the system fields the merge screen may offer (fields hidden from the user are left out) and the permissions needed to merge", body = Data<Vec<DuplicateTypeResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_duplicate_types(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<DuplicateTypeResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let types = service::types(&ctx).await?;
    Ok(Json(Data::new(types.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    get,
    path = "/duplicates/{candidate_id}",
    tag = TAG,
    operation_id = "duplicates_get",
    security(("session_cookie" = [])),
    params(("candidate_id" = String, Path, description = "Duplicate candidate id")),
    responses(
        (status = 200, description = "Duplicate candidate with a summary of both records and its status (`pending`, `dismissed` or `merged`; for a merged candidate the merged record is already in the trash)", body = Data<DuplicateCandidateResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Candidate not found or one of its records is not visible (`duplicate_candidate_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_duplicate(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(candidate_id): Path<String>,
) -> Result<Json<Data<DuplicateCandidateResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let candidate = service::get(&ctx, &candidate_id).await?;
    Ok(Json(Data::new(candidate.into())))
}

#[utoipa::path(
    post,
    path = "/duplicates/{candidate_id}/dismiss",
    tag = TAG,
    operation_id = "duplicates_dismiss",
    security(("session_cookie" = [])),
    params(("candidate_id" = String, Path, description = "Duplicate candidate id")),
    responses(
        (status = 200, description = "Candidate marked as not a duplicate; later scans keep it dismissed. Publishes `duplicate.dismissed`", body = Data<DuplicateCandidateResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the update permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Candidate not found (`duplicate_candidate_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Candidate already dismissed or merged (`duplicate_candidate_resolved`)", body = ErrorEnvelope),
    )
)]
async fn dismiss_duplicate(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(candidate_id): Path<String>,
) -> Result<Json<Data<DuplicateCandidateResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let candidate = service::dismiss(&ctx, &candidate_id, client.0.ip).await?;
    Ok(Json(Data::new(candidate.into())))
}

#[utoipa::path(
    post,
    path = "/duplicates/check",
    tag = TAG,
    operation_id = "duplicates_check",
    security(("session_cookie" = [])),
    request_body = DuplicateCheckRequest,
    responses(
        (status = 200, description = "Visible records that the enabled rules consider possible duplicates of the given values (at most 10), each with the rules that matched. Nothing is stored", body = Data<Vec<DuplicateMatchResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the create or update permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn check_duplicates(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<DuplicateCheckRequest>,
) -> Result<Json<Data<Vec<DuplicateMatchResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let matches = service::check(&ctx, request.into()).await?;
    Ok(Json(Data::new(
        matches.into_iter().map(match_response).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/duplicates/scan",
    tag = TAG,
    operation_id = "duplicates_scan",
    security(("session_cookie" = [])),
    responses(
        (status = 202, description = "Queues the `duplicates.scan` job (it also runs daily) and marks the scan as queued; follow it with `GET /duplicates/scan`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn scan_duplicates(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::request_scan(&ctx).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/duplicates/scan",
    tag = TAG,
    operation_id = "duplicates_scan_status",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "State of the latest duplicate scan (`queued`, `running`, `completed` or `failed`) with when it was requested, started and finished and how many records it analyzed and how many pairs it found; `data` is null when no scan ever ran", body = Data<Option<ScanStatusResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn scan_status(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Option<ScanStatusResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let status = service::scan_status(&ctx).await?;
    Ok(Json(Data::new(status.map(Into::into))))
}

#[utoipa::path(
    get,
    path = "/duplicates/rules",
    tag = TAG,
    operation_id = "duplicates_get_rules",
    security(("session_cookie" = [])),
    params(DuplicateRulesParams),
    responses(
        (status = 200, description = "Detection rules of the record type. Contacts: `email`, `phone`, `name_company` (similar name and same company); organizations: `domain`, `name`; other types: `identifier` (same value in an identifier column, when the type declares one) and `name` (similar title). Every rule is enabled by default", body = Data<DuplicateRulesResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn get_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<DuplicateRulesParams>,
) -> Result<Json<Data<DuplicateRulesResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let rules = service::rules(&ctx, &params.record_type).await?;
    Ok(Json(Data::new(DuplicateRulesResponse::new(
        params.record_type,
        rules,
    ))))
}

#[utoipa::path(
    put,
    path = "/duplicates/rules",
    tag = TAG,
    operation_id = "duplicates_update_rules",
    security(("session_cookie" = [])),
    request_body = UpdateDuplicateRulesRequest,
    responses(
        (status = 200, description = "Rules of the record type replaced by the given switches (omitted rules keep their state). Publishes `duplicate_rules.updated`", body = Data<DuplicateRulesResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type or unknown rule (`validation_error` on `rules.<key>`)", body = ErrorEnvelope),
    )
)]
async fn update_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<UpdateDuplicateRulesRequest>,
) -> Result<Json<Data<DuplicateRulesResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let rules = service::save_rules(&ctx, &request.record_type, request.rules, client.0.ip).await?;
    Ok(Json(Data::new(DuplicateRulesResponse::new(
        request.record_type,
        rules,
    ))))
}

#[utoipa::path(
    post,
    path = "/duplicates/merge",
    tag = TAG,
    operation_id = "duplicates_merge",
    security(("session_cookie" = [])),
    params(
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the primary record as read (ISO-8601, quotes optional); the merge fails with 409 when it changed since"),
    ),
    request_body = MergeRequest,
    responses(
        (status = 200, description = "Merges `duplicate_id` into `primary_id` in one transaction. `fields` chooses, per field key (system or custom), which record's value stays (`primary` or `duplicate`); omitted fields keep the primary value unless it is empty. Fields hidden from the user cannot be chosen: their values stay on the primary and are only filled from the duplicate when empty (redacted in the audit). Activities, notes, attachments, tags, relationships, record references, deals and company links (people), people and deals (organizations) or rows of other types that point to it (other types) move to the primary; the duplicate goes to the trash and its id redirects to the primary. The duplicate keeps its own custom values in the trash (restoring it brings them back), while the moved links stay with the primary. Both writes go through the record pipeline: the primary update runs validation rules, approvals and the other `RecordValidating` hooks and publishes `<type>.updated` (when something changed), the duplicate runs `RecordDeleting`/`RecordDeleted` and publishes `<type>.deleted`; `<type>.merged` is published as well", body = Data<MergeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission on the primary or delete permission on the duplicate (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not visible (`contact_not_found`, `company_not_found`, `record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The primary changed since the `If-Match` version, or one of the records changed or was merged while the merge was prepared (`version_conflict`); an approval blocks the update (`approval_required`, `approval_pending`); a restrict relationship blocks the deletion of the duplicate", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type, unknown field or side (`fields.<key>`), invalid custom value, a validation rule (`validation_rule_failed`), the same record twice (`merge_same_record`) or, for organizations, moving the children of the duplicate would break the hierarchy (`company_hierarchy_cycle`, `company_hierarchy_too_deep`)", body = ErrorEnvelope),
    )
)]
async fn merge_records(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    JsonBody(request): JsonBody<MergeRequest>,
) -> Result<Json<Data<MergeResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let outcome = service::merge(&ctx, request.into_command(expected, client.0.ip)).await?;
    Ok(Json(Data::new(outcome.into())))
}
