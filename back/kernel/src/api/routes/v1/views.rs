use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::views::{CreateViewRequest, SavedViewResponse, UpdateViewRequest};
use crate::api::error::ApiError;
use crate::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use crate::api::routes::v1::members::window;
use crate::api::state::AppState;
use crate::application::views::ViewUseCases;
use crate::errors::AppError;
use crate::infrastructure::views::MySqlViewPorts;

pub const TAG: &str = "views";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_views, create_view))
        .routes(routes!(update_view, delete_view))
        .routes(routes!(set_default_view, clear_default_view))
}

pub async fn view_use_cases(state: &AppState) -> Result<ViewUseCases<MySqlViewPorts>, AppError> {
    Ok(ViewUseCases::new(MySqlViewPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/views/{record_type}",
    tag = TAG,
    operation_id = "views_list",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Views visible to the user: their own, the organization ones and those of their teams", body = Page<SavedViewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_views(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(record_type): Path<String>,
    pagination: Pagination,
) -> Result<Json<Page<SavedViewResponse>>, ApiError> {
    let listing = view_use_cases(&state)
        .await?
        .list_views(&tenant.scope, &record_type, window(pagination))
        .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/views/{record_type}",
    tag = TAG,
    operation_id = "views_create",
    security(("session_cookie" = [])),
    params(("record_type" = String, Path, description = "Record type, such as `contact`")),
    request_body = CreateViewRequest,
    responses(
        (status = 201, description = "View created; filter, sort and columns are validated against the fields of the record type", body = Data<SavedViewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission, or sharing with the organization (or setting its default) without `settings.manage`, or with a team the user is not part of (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, filter, sort or columns (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_view(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(record_type): Path<String>,
    JsonBody(request): JsonBody<CreateViewRequest>,
) -> Result<(StatusCode, Json<Data<SavedViewResponse>>), ApiError> {
    let view = view_use_cases(&state)
        .await?
        .create_view(
            &tenant.scope,
            &record_type,
            request.into_command(client.0.ip),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(view.into()))))
}

#[utoipa::path(
    patch,
    path = "/views/{record_type}/{view_id}",
    tag = TAG,
    operation_id = "views_update",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("view_id" = String, Path, description = "View id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the view as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateViewRequest,
    responses(
        (status = 200, description = "View updated; omitted properties stay unchanged and `filter: null` removes the filter", body = Data<SavedViewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Only the owner or someone with `settings.manage` can change the view (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or view not visible (`record_type_not_found`, `view_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The view changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, filter, sort or columns (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_view(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, view_id)): Path<(String, String)>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateViewRequest>,
) -> Result<Json<Data<SavedViewResponse>>, ApiError> {
    let view = view_use_cases(&state)
        .await?
        .update_view(
            &tenant.scope,
            &record_type,
            &view_id,
            request.into_command(expected_updated_at, client.0.ip),
        )
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    delete,
    path = "/views/{record_type}/{view_id}",
    tag = TAG,
    operation_id = "views_delete",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("view_id" = String, Path, description = "View id"),
    ),
    responses(
        (status = 204, description = "View deleted"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Only the owner or someone with `settings.manage` can delete the view (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or view not visible (`record_type_not_found`, `view_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_view(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, view_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    view_use_cases(&state)
        .await?
        .delete_view(&tenant.scope, &record_type, &view_id, client.0.ip)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/views/{record_type}/{view_id}/default",
    tag = TAG,
    operation_id = "views_set_default",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("view_id" = String, Path, description = "View id"),
    ),
    responses(
        (status = 200, description = "The view is now the default of the current user for the record type", body = Data<SavedViewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or view not visible (`record_type_not_found`, `view_not_found`)", body = ErrorEnvelope),
    )
)]
async fn set_default_view(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, view_id)): Path<(String, String)>,
) -> Result<Json<Data<SavedViewResponse>>, ApiError> {
    let view = view_use_cases(&state)
        .await?
        .set_user_default(&tenant.scope, &record_type, &view_id)
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    delete,
    path = "/views/{record_type}/{view_id}/default",
    tag = TAG,
    operation_id = "views_clear_default",
    security(("session_cookie" = [])),
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("view_id" = String, Path, description = "View id (any visible view of the record type)"),
    ),
    responses(
        (status = 204, description = "The view is no longer the default of the current user (no change when it was not)"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or view not visible (`record_type_not_found`, `view_not_found`)", body = ErrorEnvelope),
    )
)]
async fn clear_default_view(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, view_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    view_use_cases(&state)
        .await?
        .clear_user_default(&tenant.scope, &record_type, &view_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
