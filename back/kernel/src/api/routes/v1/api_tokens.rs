use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::api_tokens::{
    ApiTokenListParams, ApiTokenResponse, ApiTokenScopeResponse, CreateApiTokenRequest,
    CreatedApiTokenResponse,
};
use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody, Pagination, PaginationQuery, SessionContext};
use crate::api::state::AppState;
use crate::application::api_tokens::ApiTokenUseCases;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::api_tokens::MySqlApiTokenPorts;

pub const TAG: &str = "api-tokens";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_api_tokens, create_api_token))
        .routes(routes!(list_api_token_scopes))
        .routes(routes!(revoke_api_token))
}

pub fn api_token_use_cases(
    state: &AppState,
) -> Result<ApiTokenUseCases<MySqlApiTokenPorts>, AppError> {
    Ok(ApiTokenUseCases::new(MySqlApiTokenPorts::new(
        state.db()?,
        state.events.clone(),
        state.audit.clone(),
    ))
    .with_security_events(state.security_events.clone()))
}

fn access_scope(session: SessionContext) -> Result<AccessScope, AppError> {
    session.ensure_two_factor_compliant()?;
    Ok(session.0.scope)
}

#[utoipa::path(
    get,
    path = "/api-tokens",
    tag = TAG,
    operation_id = "api_tokens_list",
    security(("session_cookie" = [])),
    params(ApiTokenListParams, PaginationQuery),
    responses(
        (status = 200, description = "API tokens of the current user (`owner=mine`, default) or of every member (`owner=all`, needs `users.manage`), active ones first and newest first. The secret is never returned again", body = Page<ApiTokenResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or `owner=all` without `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `owner` or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_api_tokens(
    State(state): State<AppState>,
    session: SessionContext,
    Query(params): Query<ApiTokenListParams>,
    pagination: Pagination,
) -> Result<Json<Page<ApiTokenResponse>>, ApiError> {
    let owner = params.owner().ok_or_else(|| {
        AppError::invalid_fields(
            "invalid query",
            vec![FieldViolation::new("owner", "must be mine or all")],
        )
    })?;
    let scope = access_scope(session)?;
    let tokens = api_token_use_cases(&state)?.list(&scope, owner).await?;
    let total = tokens.len() as u64;
    Ok(Json(Page::new(
        tokens
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
    path = "/api-tokens",
    tag = TAG,
    operation_id = "api_tokens_create",
    security(("session_cookie" = [])),
    request_body = CreateApiTokenRequest,
    responses(
        (status = 201, description = "Token created (`api_token.created`). `secret` (`aphel_<prefix>_<secret>`) is shown only in this response; send it as `Authorization: Bearer <secret>`. The token acts as the current user, limited to its scopes and to the user's permissions at the time of each request", body = Data<CreatedApiTokenResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, unknown scope, scope beyond the user's permissions, past `expires_at` (`validation_error`) or too many active tokens (`too_many_api_tokens`)", body = ErrorEnvelope),
    )
)]
async fn create_api_token(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateApiTokenRequest>,
) -> Result<(StatusCode, Json<Data<CreatedApiTokenResponse>>), ApiError> {
    let scope = access_scope(session)?;
    let created = api_token_use_cases(&state)?
        .create(&scope, request.into_command(client.0.ip))
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(created.into()))))
}

#[utoipa::path(
    get,
    path = "/api-tokens/scopes",
    tag = TAG,
    operation_id = "api_tokens_scopes",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Scopes the current user can grant, derived from their permissions (`people.read` → `people:read`, any other action → `<resource>:write`)", body = Data<Vec<ApiTokenScopeResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
    )
)]
async fn list_api_token_scopes(
    State(state): State<AppState>,
    session: SessionContext,
) -> Result<Json<Data<Vec<ApiTokenScopeResponse>>>, ApiError> {
    let scope = access_scope(session)?;
    let scopes = api_token_use_cases(&state)?.available_scopes(&scope);
    Ok(Json(Data::new(
        scopes.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api-tokens/{token_id}/revoke",
    tag = TAG,
    operation_id = "api_tokens_revoke",
    security(("session_cookie" = [])),
    params(("token_id" = String, Path, description = "API token id")),
    responses(
        (status = 200, description = "Token revoked (`api_token.revoked`); requests with it answer 401 `invalid_api_token` from now on. Revoking twice is harmless. Members revoke their own tokens; `users.manage` revokes any", body = Data<ApiTokenResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Token not found or owned by someone else (`api_token_not_found`)", body = ErrorEnvelope),
    )
)]
async fn revoke_api_token(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
    Path(token_id): Path<String>,
) -> Result<Json<Data<ApiTokenResponse>>, ApiError> {
    let scope = access_scope(session)?;
    let token = api_token_use_cases(&state)?
        .revoke(&scope, &token_id, client.0.ip)
        .await?;
    Ok(Json(Data::new(token.into())))
}
