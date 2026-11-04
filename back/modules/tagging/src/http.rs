use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use std::collections::HashMap;

use crate::dto::{
    CreateTagRequest, RecordTagLookupParams, RecordTagsEntry, TagListParams, TagRecordRequest,
    TagResponse, TagSummary, TaggingResponse, UpdateTagRequest,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::state::AppState;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "tagging";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_tags, create_tag))
        .routes(routes!(get_tag, update_tag, delete_tag))
        .routes(routes!(lookup_record_tags))
        .routes(routes!(list_record_tags, tag_record))
        .routes(routes!(untag_record))
}

pub const RECORD_INCLUDES: &[&str] = &["tags"];

pub async fn record_tag_map(
    state: &AppState,
    scope: &AccessScope,
    record_type: &str,
    record_ids: &[String],
) -> Result<HashMap<String, Vec<TagSummary>>, AppError> {
    if record_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(service::tags_for_records(
        &Ctx::from_state(state, scope).await?,
        record_type,
        record_ids,
    )
    .await?
    .into_iter()
    .map(|(id, tags)| (id.to_string(), tags.into_iter().map(Into::into).collect()))
    .collect())
}

#[utoipa::path(
    get,
    path = "/tagging",
    tag = TAG,
    operation_id = "tags_list",
    security(("session_cookie" = [])),
    params(TagListParams, PaginationQuery),
    responses(
        (status = 200, description = "Tags of the organization ordered by name, with how many records use each one. With `record_type`, only tags allowed on that type. Any member (and any valid API token) can list them", body = Page<TagResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Record type cannot be tagged (`record_type_not_taggable`) or invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_tags(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<TagListParams>,
    pagination: Pagination,
) -> Result<Json<Page<TagResponse>>, ApiError> {
    let tags = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.record_type.as_deref(),
    )
    .await?;
    let total = tags.len() as u64;
    Ok(Json(Page::new(
        tags.into_iter()
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
    path = "/tagging",
    tag = TAG,
    operation_id = "tags_create",
    security(("session_cookie" = [])),
    request_body = CreateTagRequest,
    responses(
        (status = 201, description = "Tag created (`tag.created`). Empty `record_types` means the tag can be used on people, organizations and deals", body = Data<TagResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another tag already has this name (`tag_name_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, color or record types (`validation_error`) or the organization already has the maximum number of tags (`too_many_tags`)", body = ErrorEnvelope),
    )
)]
async fn create_tag(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateTagRequest>,
) -> Result<(StatusCode, Json<Data<TagResponse>>), ApiError> {
    let tag = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(tag.into()))))
}

#[utoipa::path(
    get,
    path = "/tagging/{tag_id}",
    tag = TAG,
    operation_id = "tags_get",
    security(("session_cookie" = [])),
    params(("tag_id" = String, Path, description = "Tag id")),
    responses(
        (status = 200, description = "Tag with how many records use it. Any member can read it", body = Data<TagResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Tag not found (`tag_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_tag(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(tag_id): Path<String>,
) -> Result<Json<Data<TagResponse>>, ApiError> {
    let tag = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &tag_id).await?;
    Ok(Json(Data::new(tag.into())))
}

#[utoipa::path(
    patch,
    path = "/tagging/{tag_id}",
    tag = TAG,
    operation_id = "tags_update",
    security(("session_cookie" = [])),
    params(
        ("tag_id" = String, Path, description = "Tag id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the tag as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = UpdateTagRequest,
    responses(
        (status = 200, description = "Tag updated (`tag.updated`); omitted properties stay unchanged. Narrowing `record_types` keeps the existing taggings", body = Data<TagResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Tag not found (`tag_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Another tag already has this name (`tag_name_taken`) or the tag changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, color or record types, or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_tag(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(tag_id): Path<String>,
    JsonBody(request): JsonBody<UpdateTagRequest>,
) -> Result<Json<Data<TagResponse>>, ApiError> {
    let tag = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &tag_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(tag.into())))
}

#[utoipa::path(
    delete,
    path = "/tagging/{tag_id}",
    tag = TAG,
    operation_id = "tags_delete",
    security(("session_cookie" = [])),
    params(("tag_id" = String, Path, description = "Tag id")),
    responses(
        (status = 204, description = "Tag and all its taggings deleted (`tag.deleted`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Tag not found (`tag_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_tag(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(tag_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &tag_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/tagging/records",
    tag = TAG,
    operation_id = "record_tags_lookup",
    security(("session_cookie" = [])),
    params(RecordTagLookupParams),
    responses(
        (status = 200, description = "Tags of up to 100 records (comma separated `ids`), one entry per record that has tags, ordered by record id. Records the user cannot read are left out", body = Data<Vec<RecordTagsEntry>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `<type>.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Record type cannot be tagged (`record_type_not_taggable`) or invalid ids (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn lookup_record_tags(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<RecordTagLookupParams>,
) -> Result<Json<Data<Vec<RecordTagsEntry>>>, ApiError> {
    let ids = params.ids();
    let tags = service::tags_for_records(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &params.record_type,
        &ids,
    )
    .await?;
    let mut items: Vec<RecordTagsEntry> = tags
        .into_iter()
        .map(|(record_id, tags)| RecordTagsEntry {
            record_id: record_id.to_string(),
            tags: tags.into_iter().map(Into::into).collect(),
        })
        .collect();
    items.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    Ok(Json(Data::new(items)))
}

#[utoipa::path(
    get,
    path = "/tagging/records/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "record_tags_list",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "contact, company or deal"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Tags of the record ordered by name", body = Data<Vec<TagSummary>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Record does not exist or cannot be read (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Record type cannot be tagged (`record_type_not_taggable`)", body = ErrorEnvelope),
    )
)]
async fn list_record_tags(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Vec<TagSummary>>>, ApiError> {
    let tags = service::record_tags(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
    )
    .await?;
    Ok(Json(Data::new(tags.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    post,
    path = "/tagging/records/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "record_tags_add",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "contact, company or deal"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    request_body = TagRecordRequest,
    responses(
        (status = 200, description = "Tag applied (`record.tagged` with the record and `tag_id`/`tag_name` in the payload). `changed` is false when the record already had it; `tags` are the record tags after the change", body = Data<TaggingResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Record does not exist or the user cannot update it (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Tag not found (`tag_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Tag not allowed on this record type (`tag_not_allowed`) or type cannot be tagged", body = ErrorEnvelope),
    )
)]
async fn tag_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<TagRecordRequest>,
) -> Result<Json<Data<TaggingResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let changed =
        service::tag_record(&ctx, &record_type, &record_id, &request.tag_id, client.0.ip).await?;
    let tags = service::record_tags(&ctx, &record_type, &record_id).await?;
    Ok(Json(Data::new(TaggingResponse {
        changed,
        tags: tags.into_iter().map(Into::into).collect(),
    })))
}

#[utoipa::path(
    delete,
    path = "/tagging/records/{record_type}/{record_id}/{tag_id}",
    tag = TAG,
    operation_id = "record_tags_remove",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "contact, company or deal"),
        ("record_id" = String, Path, description = "Record id"),
        ("tag_id" = String, Path, description = "Tag id"),
    ),
    responses(
        (status = 204, description = "Tag removed from the record (`record.untagged`); also 204 when the record did not have it"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Record does not exist or the user cannot update it (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Tag not found (`tag_not_found`)", body = ErrorEnvelope),
    )
)]
async fn untag_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, tag_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    service::untag_record(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
        &tag_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
