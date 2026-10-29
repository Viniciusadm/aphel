use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::teams::{TeamRequest, TeamResponse, UpdateTeamRequest};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, IfMatch, JsonBody, Pagination, PaginationQuery};
use crate::api::routes::v1::members::{admin_use_cases, window};
use crate::api::state::AppState;

pub const TAG: &str = "teams";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_teams, create_team))
        .routes(routes!(get_team, update_team, delete_team))
}

#[utoipa::path(
    get,
    path = "/teams",
    tag = TAG,
    operation_id = "teams_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Teams of the active organization with members and managers", body = Page<TeamResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_teams(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<TeamResponse>>, ApiError> {
    let listing = admin_use_cases(&state)?
        .list_teams(&tenant.scope, window(pagination))
        .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/teams",
    tag = TAG,
    operation_id = "teams_create",
    security(("session_cookie" = [])),
    request_body = TeamRequest,
    responses(
        (status = 201, description = "Team created", body = Data<TeamResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_team(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<TeamRequest>,
) -> Result<(StatusCode, Json<Data<TeamResponse>>), ApiError> {
    let team = admin_use_cases(&state)?
        .create_team(&tenant.scope, request.into())
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(team.into()))))
}

#[utoipa::path(
    get,
    path = "/teams/{team_id}",
    tag = TAG,
    operation_id = "teams_get",
    security(("session_cookie" = [])),
    params(("team_id" = String, Path, description = "Team id")),
    responses(
        (status = 200, description = "Team with members", body = Data<TeamResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Team not found (`team_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_team(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(team_id): Path<String>,
) -> Result<Json<Data<TeamResponse>>, ApiError> {
    let team = admin_use_cases(&state)?
        .get_team(&tenant.scope, &team_id)
        .await?;
    Ok(Json(Data::new(team.into())))
}

#[utoipa::path(
    patch,
    path = "/teams/{team_id}",
    tag = TAG,
    operation_id = "teams_update",
    security(("session_cookie" = [])),
    params(
        ("team_id" = String, Path, description = "Team id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the team as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateTeamRequest,
    responses(
        (status = 200, description = "Team updated; omitted properties stay unchanged, `parent_team_id: null` moves it to the top level and `members` replaces the whole member list", body = Data<TeamResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Team not found (`team_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The team changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`), or the parent would create a cycle (`team_cycle`)", body = ErrorEnvelope),
    )
)]
async fn update_team(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(team_id): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateTeamRequest>,
) -> Result<Json<Data<TeamResponse>>, ApiError> {
    let team = admin_use_cases(&state)?
        .update_team(
            &tenant.scope,
            &team_id,
            request.into_command(expected_updated_at),
        )
        .await?;
    Ok(Json(Data::new(team.into())))
}

#[utoipa::path(
    delete,
    path = "/teams/{team_id}",
    tag = TAG,
    operation_id = "teams_delete",
    security(("session_cookie" = [])),
    params(("team_id" = String, Path, description = "Team id")),
    responses(
        (status = 204, description = "Team deleted; its sub-teams move to the top level"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Team not found (`team_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_team(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(team_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)?
        .delete_team(&tenant.scope, &team_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
