use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::auth::{PermissionGrantDto, RecordScopeDto};
use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::roles::{
    CreateRoleRequest, PermissionDefinitionResponse, RoleResponse, UpdateRoleRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, IfMatch, JsonBody, Pagination, PaginationQuery};
use crate::api::routes::v1::members::{admin_use_cases, window};
use crate::api::state::AppState;
use crate::domain::access::AccessScope;

pub const TAG: &str = "roles";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_permissions))
        .routes(routes!(list_roles, create_role))
        .routes(routes!(get_role, update_role, delete_role))
}

#[utoipa::path(
    get,
    path = "/permissions",
    tag = TAG,
    operation_id = "permissions_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Permission catalog; `record_scoped` permissions accept own/team/organization scopes", body = Data<Vec<PermissionDefinitionResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` and `users.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_permissions(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<PermissionDefinitionResponse>>>, ApiError> {
    let catalog = admin_use_cases(&state)?.permission_catalog(&tenant.scope)?;
    Ok(Json(Data::new(
        catalog.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/roles",
    tag = TAG,
    operation_id = "roles_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Roles of the active organization, system roles first", body = Page<RoleResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` and `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_roles(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<RoleResponse>>, ApiError> {
    let listing = admin_use_cases(&state)?
        .list_roles(&tenant.scope, window(pagination))
        .await?;
    Ok(Json(Page::new(
        listing
            .items
            .into_iter()
            .map(|view| with_extension(view.into(), &tenant.scope))
            .collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/roles",
    tag = TAG,
    operation_id = "roles_create",
    security(("session_cookie" = [])),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Custom role created", body = Data<RoleResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`role_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateRoleRequest>,
) -> Result<(StatusCode, Json<Data<RoleResponse>>), ApiError> {
    let role = admin_use_cases(&state)?
        .create_role(&tenant.scope, request.into())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(with_extension(role.into(), &tenant.scope))),
    ))
}

#[utoipa::path(
    get,
    path = "/roles/{role_id}",
    tag = TAG,
    operation_id = "roles_get",
    security(("session_cookie" = [])),
    params(("role_id" = String, Path, description = "Role id")),
    responses(
        (status = 200, description = "Role with its permissions", body = Data<RoleResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Role not found (`role_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(role_id): Path<String>,
) -> Result<Json<Data<RoleResponse>>, ApiError> {
    let role = admin_use_cases(&state)?
        .get_role(&tenant.scope, &role_id)
        .await?;
    Ok(Json(Data::new(with_extension(role.into(), &tenant.scope))))
}

#[utoipa::path(
    patch,
    path = "/roles/{role_id}",
    tag = TAG,
    operation_id = "roles_update",
    security(("session_cookie" = [])),
    params(
        ("role_id" = String, Path, description = "Role id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the role as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated; omitted `name`, `key` or `permissions` stay unchanged", body = Data<RoleResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Role not found (`role_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`role_key_taken`), system role restriction (`system_role_locked`) or the role changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(role_id): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<UpdateRoleRequest>,
) -> Result<Json<Data<RoleResponse>>, ApiError> {
    let role = admin_use_cases(&state)?
        .update_role(
            &tenant.scope,
            &role_id,
            request.into_command(expected_updated_at),
        )
        .await?;
    Ok(Json(Data::new(with_extension(role.into(), &tenant.scope))))
}

#[utoipa::path(
    delete,
    path = "/roles/{role_id}",
    tag = TAG,
    operation_id = "roles_delete",
    security(("session_cookie" = [])),
    params(("role_id" = String, Path, description = "Role id")),
    responses(
        (status = 204, description = "Role deleted"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Role not found (`role_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Role assigned to members (`role_in_use`) or system role (`system_role_locked`)", body = ErrorEnvelope),
    )
)]
async fn delete_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)?
        .delete_role(&tenant.scope, &role_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn with_extension(mut response: RoleResponse, scope: &AccessScope) -> RoleResponse {
    if response.grants_everything {
        response
            .permissions
            .extend(
                scope
                    .permissions()
                    .catalog_extension()
                    .iter()
                    .map(|permission| PermissionGrantDto {
                        permission: permission.as_str().to_owned(),
                        scope: RecordScopeDto::Organization,
                    }),
            );
    }
    response
}
