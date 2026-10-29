use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::security::{
    ChangePasswordRequest, PasswordChangedResponse, RevokedSessionsResponse,
    SecurityEventQueryParams, SecurityEventResponse, SecuritySessionResponse,
};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody, Pagination, PaginationQuery, SessionContext};
use crate::api::routes::v1::auth::TOO_MANY_ATTEMPTS;
use crate::api::state::AppState;
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::{AccountSecurityUseCases, SecurityEventView};
use crate::errors::AppError;
use crate::infrastructure::security::{MySqlAccountSecurityPorts, account_security_use_cases};

pub const TAG: &str = "security";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sessions))
        .routes(routes!(revoke_session))
        .routes(routes!(revoke_other_sessions))
        .routes(routes!(change_password))
        .routes(routes!(list_own_events))
        .routes(routes!(list_organization_events))
}

fn use_cases(
    state: &AppState,
) -> Result<AccountSecurityUseCases<MySqlAccountSecurityPorts>, AppError> {
    Ok(account_security_use_cases(state.db()?))
}

fn window(pagination: Pagination) -> ListWindow {
    ListWindow {
        offset: pagination.offset(),
        limit: pagination.limit(),
    }
}

fn events_page(
    listing: Listing<SecurityEventView>,
    pagination: Pagination,
) -> Page<SecurityEventResponse> {
    Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )
}

#[utoipa::path(
    get,
    path = "/security/sessions",
    tag = TAG,
    operation_id = "security_sessions_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Active sessions of the current user (signed in, not revoked, not expired; partial sessions waiting for the two-factor challenge are left out), the current one first and then by last access. `device` is parsed from the user agent", body = Data<Vec<SecuritySessionResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
    )
)]
async fn list_sessions(
    State(state): State<AppState>,
    session: SessionContext,
) -> Result<Json<Data<Vec<SecuritySessionResponse>>>, ApiError> {
    let sessions = use_cases(&state)?
        .list_sessions(session.user(), session.session_id())
        .await?;
    Ok(Json(Data::new(
        sessions.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    delete,
    path = "/security/sessions/{session_id}",
    tag = TAG,
    operation_id = "security_sessions_revoke",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Session id")),
    responses(
        (status = 204, description = "Session ended; the device using it is signed out on the next request (`session_revoked` security event)"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Not an active session of the current user (`session_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The current session is ended by signing out (`current_session_not_revocable`)", body = ErrorEnvelope),
    )
)]
async fn revoke_session(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .revoke_session(session.user(), session.session_id(), &session_id, &client.0)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/security/sessions/revoke-others",
    tag = TAG,
    operation_id = "security_sessions_revoke_others",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Every other session of the current user ended; the current one stays (`session_revoked` security event when any was ended)", body = Data<RevokedSessionsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
    )
)]
async fn revoke_other_sessions(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
) -> Result<Json<Data<RevokedSessionsResponse>>, ApiError> {
    let revoked = use_cases(&state)?
        .revoke_other_sessions(session.user(), session.session_id(), &client.0)
        .await?;
    Ok(Json(Data::new(RevokedSessionsResponse { revoked })))
}

#[utoipa::path(
    post,
    path = "/security/password",
    tag = TAG,
    operation_id = "security_password_change",
    security(("session_cookie" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed: every other session is ended, the current one stays, a `password_changed` security event is recorded and the user gets an e-mail notice", body = Data<PasswordChangedResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 422, description = "Wrong current password (`invalid_current_password` on `current_password`) or invalid new password (`validation_error`: 8 to 128 characters, different from the current one)", body = ErrorEnvelope),
        (status = 429, description = "Too many password change attempts (`rate_limited`, `Retry-After` and `details.retry_after`)", body = ErrorEnvelope),
    )
)]
async fn change_password(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ChangePasswordRequest>,
) -> Result<Json<Data<PasswordChangedResponse>>, ApiError> {
    state
        .rate_limits
        .password_change
        .check(
            &format!("password:{}", session.user().id.as_str()),
            TOO_MANY_ATTEMPTS,
        )
        .await?;
    let revoked_sessions = use_cases(&state)?
        .change_password(
            session.user(),
            session.session_id(),
            request.into_command(client.0),
        )
        .await?;
    Ok(Json(Data::new(PasswordChangedResponse {
        revoked_sessions,
    })))
}

#[utoipa::path(
    get,
    path = "/security/events",
    tag = TAG,
    operation_id = "security_events_list",
    security(("session_cookie" = [])),
    params(SecurityEventQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Security events of the current user's account (sign-ins, failures, lockouts, two-factor, password, sessions, API tokens and permission changes), most recent first. `kind` takes a comma-separated list; `from`/`to` are inclusive ISO-8601 instants", body = Page<SecurityEventResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown kind, invalid period or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_own_events(
    State(state): State<AppState>,
    session: SessionContext,
    pagination: Pagination,
    query: Result<Query<SecurityEventQueryParams>, QueryRejection>,
) -> Result<Json<Page<SecurityEventResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = use_cases(&state)?
        .list_own_events(session.user(), params.into(), window(pagination))
        .await?;
    Ok(Json(events_page(listing, pagination)))
}

#[utoipa::path(
    get,
    path = "/security/events/organization",
    tag = TAG,
    operation_id = "security_events_organization_list",
    security(("session_cookie" = [])),
    params(SecurityEventQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Security events of every user of the installation (needs `users.manage`): sign-ins, two-factor, password, sessions, API tokens and permission changes. Same filters as `/security/events`", body = Page<SecurityEventResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `users.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown kind, invalid period or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_organization_events(
    State(state): State<AppState>,
    session: SessionContext,
    pagination: Pagination,
    query: Result<Query<SecurityEventQueryParams>, QueryRejection>,
) -> Result<Json<Page<SecurityEventResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let scope = session.0.scope;
    let listing = use_cases(&state)?
        .list_organization_events(&scope, params.into(), window(pagination))
        .await?;
    Ok(Json(events_page(listing, pagination)))
}
