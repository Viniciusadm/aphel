use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{ErrorEnvelope, Page};
use crate::api::dto::directory::{DirectoryParams, DirectoryUserResponse};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, Pagination, PaginationQuery};
use crate::api::state::AppState;
use crate::application::directory::DirectoryUseCases;
use crate::application::organization_admin::ListWindow;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_directory::MySqlUserDirectory;

pub const TAG: &str = "directory";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_users))
}

#[utoipa::path(
    get,
    path = "/users",
    tag = TAG,
    operation_id = "directory_users_list",
    security(("session_cookie" = [])),
    params(DirectoryParams, PaginationQuery),
    responses(
        (status = 200, description = "Active users, by name, for user pickers", body = Page<DirectoryUserResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid search or pagination", body = ErrorEnvelope),
    )
)]
async fn list_users(
    State(state): State<AppState>,
    _auth: AuthContext,
    pagination: Pagination,
    query: Result<Query<DirectoryParams>, QueryRejection>,
) -> Result<Json<Page<DirectoryUserResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = DirectoryUseCases::new(MySqlUserDirectory::new(state.db()?))
        .list_users(
            params.search,
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
