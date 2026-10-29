use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{CreateFavoriteCommand, FavoriteEntry, FavoriteKind};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody, Pagination, PaginationQuery};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "favorites";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_favorites, create_favorite))
        .routes(routes!(reorder_favorites))
        .routes(routes!(delete_favorite))
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = FavoriteType)]
pub(crate) enum FavoriteTypeSchema {
    Record,
    View,
}

impl From<FavoriteKind> for FavoriteTypeSchema {
    fn from(kind: FavoriteKind) -> Self {
        match kind {
            FavoriteKind::Record => Self::Record,
            FavoriteKind::View => Self::View,
        }
    }
}

impl From<FavoriteTypeSchema> for FavoriteKind {
    fn from(kind: FavoriteTypeSchema) -> Self {
        match kind {
            FavoriteTypeSchema::Record => Self::Record,
            FavoriteTypeSchema::View => Self::View,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateFavoriteRequest {
    pub(crate) favorite_type: FavoriteTypeSchema,
    #[schema(example = "contact")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) view_id: Option<String>,
}

impl From<CreateFavoriteRequest> for CreateFavoriteCommand {
    fn from(request: CreateFavoriteRequest) -> Self {
        Self {
            favorite_type: request.favorite_type.into(),
            record_type: request.record_type,
            record_id: request.record_id,
            view_id: request.view_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReorderFavoritesRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct FavoriteResponse {
    pub(crate) id: String,
    pub(crate) favorite_type: FavoriteTypeSchema,
    pub(crate) record_type: String,
    pub(crate) record_id: Option<String>,
    pub(crate) view_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) available: bool,
    pub(crate) position: i32,
    pub(crate) created_at: String,
}

impl From<FavoriteEntry> for FavoriteResponse {
    fn from(entry: FavoriteEntry) -> Self {
        let favorite = entry.favorite;
        Self {
            id: favorite.id.to_string(),
            favorite_type: favorite.kind.into(),
            record_type: favorite.record_type.as_str().to_owned(),
            record_id: favorite.record_id().map(|id| id.to_string()),
            view_id: favorite.view_id().map(|id| id.to_string()),
            title: entry.title,
            available: entry.available,
            position: favorite.position,
            created_at: favorite
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[utoipa::path(
    get,
    path = "/favorites",
    tag = TAG,
    operation_id = "favorites_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Favorites of the current user, in display order. The title is resolved under the caller's read scope: a record that was deleted or is no longer readable, or a view no longer visible, comes with `available = false` and `title = null`", body = Page<FavoriteResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_favorites(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<FavoriteResponse>>, ApiError> {
    let favorites = service::list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    let total = favorites.len() as u64;
    let items = favorites
        .into_iter()
        .skip(usize::try_from(pagination.offset()).unwrap_or(usize::MAX))
        .take(usize::try_from(pagination.limit()).unwrap_or(usize::MAX))
        .map(Into::into)
        .collect();
    Ok(Json(Page::new(items, total, pagination)))
}

#[utoipa::path(
    post,
    path = "/favorites",
    tag = TAG,
    operation_id = "favorites_create",
    security(("session_cookie" = [])),
    request_body = CreateFavoriteRequest,
    responses(
        (status = 201, description = "Favorite added at the end of the list. `record` needs `record_type` and `record_id` of a record the caller can read; `view` needs `view_id` of a visible saved view. A personal preference: no domain event or audit entry", body = Data<FavoriteResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "The record does not exist or is outside the read scope, or the view is not visible (`favorite_target_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already a favorite (`favorite_already_exists`) or the limit of 50 favorites was reached (`favorite_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`validation_error`) or a record type that cannot be a favorite (`unsupported_record_type`)", body = ErrorEnvelope),
    )
)]
async fn create_favorite(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateFavoriteRequest>,
) -> Result<(StatusCode, Json<Data<FavoriteResponse>>), ApiError> {
    let favorite = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(favorite.into()))))
}

#[utoipa::path(
    put,
    path = "/favorites/order",
    tag = TAG,
    operation_id = "favorites_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderFavoritesRequest,
    responses(
        (status = 200, description = "Every favorite of the user in the new order", body = Data<Vec<FavoriteResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "`ids` must list every favorite of the user exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_favorites(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<ReorderFavoritesRequest>,
) -> Result<Json<Data<Vec<FavoriteResponse>>>, ApiError> {
    let favorites =
        service::reorder(&Ctx::from_state(&state, &tenant.scope).await?, &request.ids).await?;
    Ok(Json(Data::new(
        favorites.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    delete,
    path = "/favorites/{favorite_id}",
    tag = TAG,
    operation_id = "favorites_delete",
    security(("session_cookie" = [])),
    params(("favorite_id" = String, Path, description = "Favorite id")),
    responses(
        (status = 204, description = "Favorite removed"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Not a favorite of the current user (`favorite_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_favorite(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(favorite_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(&Ctx::from_state(&state, &tenant.scope).await?, &favorite_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
