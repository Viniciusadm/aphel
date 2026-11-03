use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::SegmentMembersQuery;
use crate::dto::{
    CreateSegmentRequest, SegmentListParams, SegmentMemberResponse, SegmentResponse,
    UpdateSegmentRequest,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "segments";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_segments, create_segment))
        .routes(routes!(get_segment, update_segment, delete_segment))
        .routes(routes!(list_segment_members))
}

#[utoipa::path(
    get,
    path = "/segments",
    tag = TAG,
    operation_id = "segments_list",
    security(("session_cookie" = [])),
    params(SegmentListParams, PaginationQuery),
    responses(
        (status = 200, description = "Segments of the record types the user can read. With `counts=true`, `member_count` is the number of records of the segment visible to the user (null when the filter is no longer valid)", body = Page<SegmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.read` for the requested `record_type` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_segments(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<SegmentListParams>,
    pagination: Pagination,
) -> Result<Json<Page<SegmentResponse>>, ApiError> {
    let segments = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.record_type.as_deref(),
        params.counts.unwrap_or(false),
    )
    .await?;
    let total = segments.len() as u64;
    Ok(Json(Page::new(
        segments
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
    path = "/segments",
    tag = TAG,
    operation_id = "segments_create",
    security(("session_cookie" = [])),
    request_body = CreateSegmentRequest,
    responses(
        (status = 201, description = "Segment created (`segment.created`). The filter is the same AST of the listings and is evaluated on every use; it cannot reference other segments (`in_saved_filter`)", body = Data<SegmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` or `<type>.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another segment of the type already has this name (`segment_name_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, description or filter (`validation_error`, `too_many_segments`)", body = ErrorEnvelope),
    )
)]
async fn create_segment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateSegmentRequest>,
) -> Result<(StatusCode, Json<Data<SegmentResponse>>), ApiError> {
    let segment = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(segment.into()))))
}

#[utoipa::path(
    get,
    path = "/segments/{segment_id}",
    tag = TAG,
    operation_id = "segments_get",
    security(("session_cookie" = [])),
    params(("segment_id" = String, Path, description = "Segment id")),
    responses(
        (status = 200, description = "Segment with its current `member_count`", body = Data<SegmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Segment not found or of a type the user cannot read (`segment_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_segment(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(segment_id): Path<String>,
) -> Result<Json<Data<SegmentResponse>>, ApiError> {
    let segment = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &segment_id).await?;
    Ok(Json(Data::new(segment.into())))
}

#[utoipa::path(
    patch,
    path = "/segments/{segment_id}",
    tag = TAG,
    operation_id = "segments_update",
    security(("session_cookie" = [])),
    params(
        ("segment_id" = String, Path, description = "Segment id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the segment as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = UpdateSegmentRequest,
    responses(
        (status = 200, description = "Segment updated (`segment.updated`); omitted properties stay unchanged and `description: null` clears it", body = Data<SegmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Segment not found (`segment_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Another segment of the type already has this name (`segment_name_taken`) or the segment changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, description or filter, or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_segment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(segment_id): Path<String>,
    JsonBody(request): JsonBody<UpdateSegmentRequest>,
) -> Result<Json<Data<SegmentResponse>>, ApiError> {
    let segment = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &segment_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(segment.into())))
}

#[utoipa::path(
    delete,
    path = "/segments/{segment_id}",
    tag = TAG,
    operation_id = "segments_delete",
    security(("session_cookie" = [])),
    params(("segment_id" = String, Path, description = "Segment id")),
    responses(
        (status = 204, description = "Segment deleted (`segment.deleted`). Filters that still reference it answer 422 `segment was not found`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Segment not found (`segment_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_segment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(segment_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &segment_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/segments/{segment_id}/members",
    tag = TAG,
    operation_id = "segments_members",
    security(("session_cookie" = [])),
    params(
        ("segment_id" = String, Path, description = "Segment id"),
        RecordQueryParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Records of the segment visible to the user (same engine, scope and archived handling of the listings). `filter` (JSON AST or `filter[field][operator]=value`, combined with `and`) narrows the segment further; `sort` and `search` work as in the record listings", body = Page<SegmentMemberResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Segment not found (`segment_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search or pagination, or the segment filter is no longer valid (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_segment_members(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(segment_id): Path<String>,
    RecordListQuery(query): RecordListQuery,
    pagination: Pagination,
) -> Result<Json<Page<SegmentMemberResponse>>, ApiError> {
    let listing = service::members(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &segment_id,
        SegmentMembersQuery {
            filter: query.filter,
            sort: query.sort,
            search: query.search,
            archived: query.archived,
            window: query.window,
        },
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}
