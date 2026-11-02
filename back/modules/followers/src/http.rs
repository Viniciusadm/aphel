use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{FollowedRecord, RecordFollower, RecordFollowers};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, timestamp as instant};
use aphel_kernel::api::dto::files::UserReferenceResponse;
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, Pagination, PaginationQuery};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "followers";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_record_followers))
        .routes(routes!(follow_record, unfollow_record))
        .routes(routes!(list_followed_records))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordFollowerResponse {
    pub(crate) user: UserReferenceResponse,
    #[schema(example = "manual")]
    pub(crate) origin: String,
    pub(crate) followed_at: String,
}

impl From<RecordFollower> for RecordFollowerResponse {
    fn from(follower: RecordFollower) -> Self {
        Self {
            user: follower.user.into(),
            origin: follower.origin.as_str().to_owned(),
            followed_at: instant(follower.followed_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordFollowersResponse {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) following: bool,
    pub(crate) total: usize,
    pub(crate) followers: Vec<RecordFollowerResponse>,
}

impl From<RecordFollowers> for RecordFollowersResponse {
    fn from(followers: RecordFollowers) -> Self {
        Self {
            record_type: followers.record.record_type.as_str().to_owned(),
            record_id: followers.record.record_id.to_string(),
            following: followers.following,
            total: followers.followers.len(),
            followers: followers.followers.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct FollowedRecordResponse {
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) title: Option<String>,
    #[schema(example = "owner")]
    pub(crate) origin: String,
    pub(crate) followed_at: String,
}

impl From<FollowedRecord> for FollowedRecordResponse {
    fn from(record: FollowedRecord) -> Self {
        Self {
            record_type: record.follow.record.record_type.as_str().to_owned(),
            record_id: record.follow.record.record_id.to_string(),
            title: record.title,
            origin: record.follow.origin.as_str().to_owned(),
            followed_at: instant(record.follow.followed_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct FollowedRecordsParams {
    #[param(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/records/{record_type}/{record_id}/followers",
    tag = TAG,
    operation_id = "record_followers_list",
    security(("session_cookie" = [])),
    description = "Active members who follow the record and whether the caller follows it. Followers are notified (`followed_record_activity`) of changes, new activities, notes and emails linked to the record while they can read it. The owner follows automatically (`origin: owner`).",
    params(
        ("record_type" = String, Path, description = "Record type, such as `deal` or `custom.contract`"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Followers of the record", body = Data<RecordFollowersResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown type or record not visible (`record_type_not_found`, `record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_record_followers(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<RecordFollowersResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let followers = service::followers(&ctx, &record_type, &record_id).await?;
    Ok(Json(Data::new(followers.into())))
}

#[utoipa::path(
    post,
    path = "/records/{record_type}/{record_id}/follow",
    tag = TAG,
    operation_id = "record_follow",
    security(("session_cookie" = [])),
    description = "The caller starts following a record they can read. Following twice is a no-op. Following is a personal preference: it publishes no event.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Followers of the record after following", body = Data<RecordFollowersResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown type or record not visible (`record_type_not_found`, `record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn follow_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<RecordFollowersResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let followers = service::follow(&ctx, &record_type, &record_id).await?;
    Ok(Json(Data::new(followers.into())))
}

#[utoipa::path(
    delete,
    path = "/records/{record_type}/{record_id}/follow",
    tag = TAG,
    operation_id = "record_unfollow",
    security(("session_cookie" = [])),
    description = "The caller stops following a record (also when they follow it as owner; they only follow again if they become the owner again). Works even when the record is no longer readable.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Followers of the record after unfollowing (empty when the record is not readable)", body = Data<RecordFollowersResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 404, description = "Unknown type or invalid id (`record_type_not_found`, `record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unfollow_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<RecordFollowersResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let followers = service::unfollow(&ctx, &record_type, &record_id).await?;
    Ok(Json(Data::new(followers.into())))
}

#[utoipa::path(
    get,
    path = "/me/followed-records",
    tag = TAG,
    operation_id = "followed_records_list",
    security(("session_cookie" = [])),
    description = "Records the caller follows and can still read, most recently followed first, optionally of a single `record_type`.",
    params(FollowedRecordsParams, PaginationQuery),
    responses(
        (status = 200, description = "Followed records", body = Page<FollowedRecordResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_followed_records(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<FollowedRecordsParams>,
    pagination: Pagination,
) -> Result<Json<Page<FollowedRecordResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing =
        service::followed(&ctx, params.record_type.as_deref(), window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}
