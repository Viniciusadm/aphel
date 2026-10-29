use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::auth::{
    AcceptInvitationRequest, ForgotPasswordRequest, InvitationResponse, LoginRequest,
    LoginResponse, MeResponse, ResetPasswordRequest, SsoDiscoverRequest, SsoDiscoverResponse,
    SsoOptionResponse, TokenRequest, TwoFactorRequiredResponse,
};
use crate::api::dto::common::ErrorEnvelope;
use crate::api::error::ApiError;
use crate::api::extractors::auth::{API_TOKEN_NOT_ALLOWED, bearer_token};
use crate::api::extractors::{ClientInfo, JsonBody, SessionContext};
use crate::api::session_cookie;
use crate::api::state::AppState;
use crate::application::auth::AccountOverview;
use crate::application::auth::{
    AcceptInvitationCommand, IssuedSession, LoginCommand, LoginOutcome, ResetPasswordCommand,
};
use crate::domain::Secret;
use crate::errors::AppError;

pub const TAG: &str = "auth";
pub const SSO_REQUIRED: &str = "sso_required";
pub const TOO_MANY_ATTEMPTS: &str = "too many attempts, try again later";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(sso_discover))
        .routes(routes!(logout))
        .routes(routes!(logout_all))
        .routes(routes!(me))
        .routes(routes!(verify_email))
        .routes(routes!(resend_verification))
        .routes(routes!(forgot_password))
        .routes(routes!(reset_password))
        .routes(routes!(invitation))
        .routes(routes!(accept_invitation))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = TAG,
    operation_id = "auth_login",
    description = "Validates the credentials and starts a session (sets the `aphel_session` cookie). When the account has two-factor authentication, answers `{ two_factor_required: true, expires_at }` instead and the cookie holds a partial session that only accepts `POST /auth/two-factor/challenge` (and logout) for 10 minutes.",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Session started (`MeResponse`) or second factor required (`TwoFactorRequiredResponse`)", body = LoginResponse),
        (status = 401, description = "Invalid credentials (`unauthorized`); the answer is the same for unknown emails", body = ErrorEnvelope),
        (status = 403, description = "Single sign-on is required for this email domain (`sso_required`; use `POST /auth/sso/discover`) or missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 413, description = "Request body too large (`payload_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Malformed or invalid request body (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "More than 10 attempts in 5 minutes for this IP and email (`rate_limited`, with `Retry-After`), or the account is locked for this IP after repeated failures (`login_locked`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn login(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<LoginRequest>,
) -> Result<Response, ApiError> {
    let key = format!(
        "{}|{}",
        client.ip.as_deref().unwrap_or("unknown"),
        request.email.trim().to_ascii_lowercase()
    );
    state
        .rate_limits
        .login
        .check(&key, TOO_MANY_ATTEMPTS)
        .await?;
    if crate::api::sso::login_options(&state, &request.email)
        .await
        .iter()
        .any(|option| option.required)
    {
        return Err(AppError::forbidden_code(
            SSO_REQUIRED,
            "this email must sign in with single sign-on",
        )
        .into());
    }

    let outcome = state
        .auth_use_cases()?
        .login(LoginCommand {
            email: request.email,
            password: Secret::new(request.password),
            client,
        })
        .await?;
    login_response(&state, outcome).await
}

#[utoipa::path(
    post,
    path = "/auth/sso/discover",
    tag = TAG,
    operation_id = "auth_sso_discover",
    description = "Lists the single sign-on options offered by enabled modules for an email (by its domain). `required` means password login is refused for it. `url` starts the provider flow in the browser.",
    request_body = SsoDiscoverRequest,
    responses(
        (status = 200, description = "Single sign-on options (may be empty)", body = SsoDiscoverResponse),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Malformed or invalid request body (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "More than 10 lookups in 5 minutes from this IP (`rate_limited`, with `Retry-After`)", body = ErrorEnvelope),
    )
)]
async fn sso_discover(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<SsoDiscoverRequest>,
) -> Result<Json<SsoDiscoverResponse>, ApiError> {
    let key = format!("sso|{}", client.ip.as_deref().unwrap_or("unknown"));
    state
        .rate_limits
        .login
        .check(&key, TOO_MANY_ATTEMPTS)
        .await?;
    let options = crate::api::sso::login_options(&state, &request.email).await;
    Ok(Json(SsoDiscoverResponse {
        data: options.into_iter().map(SsoOptionResponse::from).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = TAG,
    operation_id = "auth_logout",
    description = "Revokes the current session, including a partial session waiting for the second factor.",
    security(("session_cookie" = [])),
    responses(
        (status = 204, description = "Session revoked and cookie cleared"),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    if bearer_token(&headers).is_some() {
        return Err(AppError::forbidden_code(
            API_TOKEN_NOT_ALLOWED,
            "this route only accepts a browser session, not an API token",
        )
        .into());
    }
    let token = session_cookie::read(&headers)
        .ok_or_else(|| AppError::Unauthorized("missing session".to_owned()))?;
    state.auth_use_cases()?.logout_token(token.expose()).await?;
    cleared_session_response(&state)
}

#[utoipa::path(
    post,
    path = "/auth/logout-all",
    tag = TAG,
    operation_id = "auth_logout_all",
    description = "Revokes every session of the current user, on every device, and clears the cookie.",
    security(("session_cookie" = [])),
    responses(
        (status = 204, description = "Every session of the user revoked and cookie cleared"),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn logout_all(
    State(state): State<AppState>,
    auth: SessionContext,
) -> Result<Response, ApiError> {
    state.auth_use_cases()?.logout_all(&auth.user().id).await?;
    cleared_session_response(&state)
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = TAG,
    operation_id = "auth_me",
    description = "Current user of the session with the organization, role, teams and effective permissions.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Current user, organization and permissions", body = MeResponse),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn me(
    State(state): State<AppState>,
    auth: SessionContext,
) -> Result<Json<MeResponse>, ApiError> {
    let account = state.auth_use_cases()?.account(auth.0).await?;
    Ok(Json(me_response(&state, account).await?))
}

#[utoipa::path(
    post,
    path = "/auth/verify-email",
    tag = TAG,
    operation_id = "auth_verify_email",
    description = "Marks the email of the account as verified from the token sent by email. Each token works once.",
    request_body = TokenRequest,
    responses(
        (status = 204, description = "Email verified"),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid, used or expired token (`invalid_token`) or malformed body (`validation_error`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn verify_email(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<TokenRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .auth_use_cases()?
        .verify_email(&Secret::new(request.token))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/auth/resend-verification",
    tag = TAG,
    operation_id = "auth_resend_verification",
    description = "Sends the verification email again (a new link; the previous ones stop working). The email is sent during the request; an SMTP failure is only logged and the answer is still 204.",
    security(("session_cookie" = [])),
    responses(
        (status = 204, description = "Verification email sent again (no-op when already verified)"),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 429, description = "More than 5 requests in 15 minutes for this user (`rate_limited`, with `Retry-After`)", body = ErrorEnvelope),
    )
)]
async fn resend_verification(
    State(state): State<AppState>,
    auth: SessionContext,
) -> Result<StatusCode, ApiError> {
    state
        .rate_limits
        .password_reset
        .check(
            &format!("verification|{}", auth.user().id),
            TOO_MANY_ATTEMPTS,
        )
        .await?;
    state
        .auth_use_cases()?
        .resend_email_verification(auth.user())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = TAG,
    operation_id = "auth_forgot_password",
    description = "Sends a password reset link when the email belongs to an active account with a password. Always answers 202 so it does not reveal which emails exist; the email is sent during the request and an SMTP failure is only logged.",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 202, description = "Request accepted"),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Malformed request body (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "More than 5 requests in 15 minutes from this IP (`rate_limited`, with `Retry-After`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn forgot_password(
    State(state): State<AppState>,
    client: ClientInfo,
    JsonBody(request): JsonBody<ForgotPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .rate_limits
        .password_reset
        .check(
            &format!("forgot|{}", client.ip_or_unknown()),
            TOO_MANY_ATTEMPTS,
        )
        .await?;
    state
        .auth_use_cases()?
        .request_password_reset(&request.email)
        .await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    post,
    path = "/auth/reset-password",
    tag = TAG,
    operation_id = "auth_reset_password",
    description = "Sets a new password from a reset token and revokes every session of the user.",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "Password changed"),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid, used or expired token (`invalid_token`) or invalid password (`validation_error`: 8 to 128 characters)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn reset_password(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<ResetPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .auth_use_cases()?
        .reset_password(ResetPasswordCommand {
            token: Secret::new(request.token),
            password: Secret::new(request.password),
            client,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/auth/invitation",
    tag = TAG,
    operation_id = "auth_invitation",
    description = "Describes a pending invitation so the acceptance page knows whether it must ask for a name and a new password.",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Pending invitation", body = InvitationResponse),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid, used or expired token (`invalid_token`) or malformed body (`validation_error`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn invitation(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<TokenRequest>,
) -> Result<Json<InvitationResponse>, ApiError> {
    let preview = state
        .auth_use_cases()?
        .invitation_preview(&Secret::new(request.token))
        .await?;
    Ok(Json(preview.into()))
}

#[utoipa::path(
    post,
    path = "/auth/accept-invitation",
    tag = TAG,
    operation_id = "auth_accept_invitation",
    description = "Accepts an invitation. New accounts send `display_name` and a new password; existing accounts confirm their current password. Starts a session in the invited organization; accounts with two-factor authentication get a partial session and `{ two_factor_required: true }`, as in login.",
    request_body = AcceptInvitationRequest,
    responses(
        (status = 200, description = "Invitation accepted: session started (`MeResponse`) or second factor required (`TwoFactorRequiredResponse`)", body = LoginResponse),
        (status = 401, description = "Wrong password for an existing account (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`) or invalid, used or expired token (`invalid_token`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn accept_invitation(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<AcceptInvitationRequest>,
) -> Result<Response, ApiError> {
    let outcome = state
        .auth_use_cases()?
        .accept_invitation(AcceptInvitationCommand {
            token: Secret::new(request.token),
            display_name: request.display_name,
            password: Secret::new(request.password),
            client,
        })
        .await?;
    login_response(&state, outcome).await
}

pub async fn login_response(state: &AppState, outcome: LoginOutcome) -> Result<Response, ApiError> {
    match outcome {
        LoginOutcome::Session(session) => session_response(state, StatusCode::OK, *session).await,
        LoginOutcome::TwoFactorRequired(pending) => {
            let cookie = session_cookie::issue(
                &pending.token,
                pending.expires_at,
                Utc::now(),
                state.secure_cookies(),
            )?;
            Ok(with_cookie(
                (
                    StatusCode::OK,
                    Json(LoginResponse::TwoFactorRequired(
                        TwoFactorRequiredResponse::from(pending),
                    )),
                )
                    .into_response(),
                cookie,
            ))
        }
    }
}

pub async fn me_response(
    state: &AppState,
    account: AccountOverview,
) -> Result<MeResponse, ApiError> {
    let team_ids: Vec<String> = account
        .scope
        .team_ids()
        .iter()
        .map(|team| team.as_str().to_owned())
        .collect();
    let mut response = MeResponse::from(account);
    if !team_ids.is_empty()
        && let Ok(pool) = state.db()
    {
        let mut builder =
            sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT id, name FROM teams WHERE id IN (");
        let mut separated = builder.separated(", ");
        for id in &team_ids {
            separated.push_bind(id.clone());
        }
        builder.push(") ORDER BY name");
        let rows: Vec<(String, String)> = builder
            .build_query_as()
            .fetch_all(&pool)
            .await
            .map_err(AppError::from)?;
        response.teams = rows
            .into_iter()
            .map(|(id, name)| crate::api::dto::members::TeamReferenceResponse { id, name })
            .collect();
    }
    Ok(response)
}

pub async fn session_response(
    state: &AppState,
    status: StatusCode,
    session: IssuedSession,
) -> Result<Response, ApiError> {
    let me = me_response(state, session.account).await?;
    let cookie = session_cookie::issue(
        &session.token,
        session.expires_at,
        Utc::now(),
        state.secure_cookies(),
    )?;
    Ok(with_cookie((status, Json(me)).into_response(), cookie))
}

fn cleared_session_response(state: &AppState) -> Result<Response, ApiError> {
    Ok(with_cookie(
        StatusCode::NO_CONTENT.into_response(),
        session_cookie::clear(state.secure_cookies())?,
    ))
}

fn with_cookie(mut response: Response, cookie: HeaderValue) -> Response {
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
}
