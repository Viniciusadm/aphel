use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::auth::MeResponse;
use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::two_factor::{
    ConfirmTwoFactorRequest, DisableTwoFactorRequest, RecoveryCodesResponse,
    RegenerateRecoveryCodesRequest, TwoFactorChallengeRequest, TwoFactorPolicyResponse,
    TwoFactorSetupResponse, TwoFactorStatusResponse, UpdateTwoFactorPolicyRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::auth::{API_TOKEN_NOT_ALLOWED, bearer_token};
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody, SessionContext};
use crate::api::routes::v1::auth::{TOO_MANY_ATTEMPTS, session_response};
use crate::api::session_cookie;
use crate::api::state::AppState;
use crate::application::auth::two_factor::{
    ConfirmTwoFactorCommand, DisableTwoFactorCommand, RegenerateRecoveryCodesCommand,
    TwoFactorPolicy, UpdateTwoFactorPolicyCommand,
};
use crate::domain::Secret;
use crate::errors::AppError;

pub const TAG: &str = "two-factor";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(challenge))
        .routes(routes!(status))
        .routes(routes!(begin_setup))
        .routes(routes!(confirm_setup))
        .routes(routes!(disable))
        .routes(routes!(regenerate_recovery_codes))
        .routes(routes!(policy, update_policy))
}

async fn check_management_limit(
    state: &AppState,
    session: &SessionContext,
) -> Result<(), AppError> {
    state
        .rate_limits
        .two_factor
        .check(&format!("manage|{}", session.user().id), TOO_MANY_ATTEMPTS)
        .await
}

#[utoipa::path(
    post,
    path = "/auth/two-factor/challenge",
    tag = TAG,
    operation_id = "two_factor_challenge",
    description = "Second step of the sign in. Accepts a 6-digit TOTP code (±30 s, each code works once) or an unused recovery code, using the partial session cookie set by `POST /auth/login`. On success the partial session is replaced by a new full session (new cookie). Five wrong codes revoke the partial session (`two_factor_attempts_exceeded`), and the user must sign in again.",
    security(("session_cookie" = [])),
    request_body = TwoFactorChallengeRequest,
    responses(
        (status = 200, description = "Second factor accepted, session started", body = MeResponse),
        (status = 401, description = "Missing, expired or already completed partial session, or too many wrong codes (`two_factor_attempts_exceeded`)", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 422, description = "Wrong, expired or reused code (`invalid_two_factor_code`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts", body = ErrorEnvelope),
    )
)]
async fn challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<TwoFactorChallengeRequest>,
) -> Result<Response, ApiError> {
    if bearer_token(&headers).is_some() {
        return Err(AppError::forbidden_code(
            API_TOKEN_NOT_ALLOWED,
            "this route only accepts a browser session, not an API token",
        )
        .into());
    }
    let token = session_cookie::read(&headers)
        .ok_or_else(|| AppError::Unauthorized("missing session".to_owned()))?;
    let auth = state.auth_use_cases()?;
    let pending = auth.pending_challenge(token.expose()).await?;
    state
        .rate_limits
        .two_factor
        .check(&format!("challenge|{}", pending.user.id), TOO_MANY_ATTEMPTS)
        .await?;
    let session = auth
        .complete_two_factor(pending, &Secret::new(request.code), client)
        .await?;
    session_response(&state, StatusCode::OK, session).await
}

#[utoipa::path(
    get,
    path = "/two-factor",
    tag = TAG,
    operation_id = "two_factor_status",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Two-factor state of the current user", body = Data<TwoFactorStatusResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
    )
)]
async fn status(
    State(state): State<AppState>,
    session: SessionContext,
) -> Result<Json<Data<TwoFactorStatusResponse>>, ApiError> {
    let overview = state
        .two_factor_use_cases()?
        .overview(session.user())
        .await?;
    Ok(Json(Data::new(overview.into())))
}

#[utoipa::path(
    post,
    path = "/two-factor/setup",
    tag = TAG,
    operation_id = "two_factor_setup",
    description = "Starts (or restarts) the activation: generates a new TOTP secret and answers it with the `otpauth://` URI and an SVG QR code. Two-factor authentication is only enabled after `POST /two-factor/confirm` with a valid code.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Secret generated, waiting for confirmation", body = Data<TwoFactorSetupResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 409, description = "Already enabled (`two_factor_already_enabled`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn begin_setup(
    State(state): State<AppState>,
    session: SessionContext,
) -> Result<Json<Data<TwoFactorSetupResponse>>, ApiError> {
    let setup = state
        .two_factor_use_cases()?
        .begin_setup(session.user())
        .await?;
    Ok(Json(Data::new(setup.into())))
}

#[utoipa::path(
    post,
    path = "/two-factor/confirm",
    tag = TAG,
    operation_id = "two_factor_confirm",
    description = "Confirms the activation with a code from the authenticator app and enables two-factor authentication. Answers the 10 single-use recovery codes, shown only this time.",
    security(("session_cookie" = [])),
    request_body = ConfirmTwoFactorRequest,
    responses(
        (status = 200, description = "Enabled; recovery codes", body = Data<RecoveryCodesResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 409, description = "Setup not started (`two_factor_setup_not_started`) or already enabled (`two_factor_already_enabled`)", body = ErrorEnvelope),
        (status = 422, description = "Wrong code (`invalid_two_factor_code`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts", body = ErrorEnvelope),
    )
)]
async fn confirm_setup(
    State(state): State<AppState>,
    session: SessionContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<ConfirmTwoFactorRequest>,
) -> Result<Json<Data<RecoveryCodesResponse>>, ApiError> {
    check_management_limit(&state, &session).await?;
    let codes = state
        .two_factor_use_cases()?
        .confirm_setup(
            session.user(),
            ConfirmTwoFactorCommand {
                code: Secret::new(request.code),
                client,
            },
        )
        .await?;
    Ok(Json(Data::new(codes.into())))
}

#[utoipa::path(
    post,
    path = "/two-factor/disable",
    tag = TAG,
    operation_id = "two_factor_disable",
    description = "Disables two-factor authentication. Requires the current password and a TOTP or recovery code; removes the secret and every recovery code.",
    security(("session_cookie" = [])),
    request_body = DisableTwoFactorRequest,
    responses(
        (status = 204, description = "Disabled"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 409, description = "Not enabled (`two_factor_not_enabled`)", body = ErrorEnvelope),
        (status = 422, description = "Wrong password (`invalid_password`) or code (`invalid_two_factor_code`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts", body = ErrorEnvelope),
    )
)]
async fn disable(
    State(state): State<AppState>,
    session: SessionContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<DisableTwoFactorRequest>,
) -> Result<StatusCode, ApiError> {
    check_management_limit(&state, &session).await?;
    state
        .two_factor_use_cases()?
        .disable(
            session.user(),
            DisableTwoFactorCommand {
                password: Secret::new(request.password),
                code: Secret::new(request.code),
                client,
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/two-factor/recovery-codes",
    tag = TAG,
    operation_id = "two_factor_regenerate_recovery_codes",
    description = "Replaces every recovery code with 10 new ones, shown only in this response. Confirm with the current `password` or a two-factor `code`.",
    security(("session_cookie" = [])),
    request_body = RegenerateRecoveryCodesRequest,
    responses(
        (status = 200, description = "New recovery codes", body = Data<RecoveryCodesResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 409, description = "Not enabled (`two_factor_not_enabled`)", body = ErrorEnvelope),
        (status = 422, description = "Missing confirmation (`validation_error`), wrong password (`invalid_password`) or code (`invalid_two_factor_code`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts", body = ErrorEnvelope),
    )
)]
async fn regenerate_recovery_codes(
    State(state): State<AppState>,
    session: SessionContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<RegenerateRecoveryCodesRequest>,
) -> Result<Json<Data<RecoveryCodesResponse>>, ApiError> {
    check_management_limit(&state, &session).await?;
    let codes = state
        .two_factor_use_cases()?
        .regenerate_recovery_codes(
            session.user(),
            RegenerateRecoveryCodesCommand {
                password: request.password.map(Secret::new),
                code: request.code.map(Secret::new),
                client,
            },
        )
        .await?;
    Ok(Json(Data::new(codes.into())))
}

#[utoipa::path(
    get,
    path = "/organization/two-factor-policy",
    tag = TAG,
    operation_id = "two_factor_policy",
    description = "Two-factor policy of the organization (`none`, `admins` or `everyone`) and how many active members it covers still have no second factor. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Current policy", body = Data<TwoFactorPolicyResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` or two-factor setup required (`two_factor_setup_required`)", body = ErrorEnvelope),
    )
)]
async fn policy(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<TwoFactorPolicyResponse>>, ApiError> {
    let view = state.two_factor_use_cases()?.policy(&tenant.scope).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/organization/two-factor-policy",
    tag = TAG,
    operation_id = "two_factor_update_policy",
    description = "Changes who must use two-factor authentication in the organization: `none`, `admins` (members with the system administrator role) or `everyone`. Members covered by the policy without a second factor can only reach their security settings until they enable it (`403 two_factor_setup_required`); API tokens are not affected. Publishes `organization.two_factor_policy_changed` with audit. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    request_body = UpdateTwoFactorPolicyRequest,
    responses(
        (status = 200, description = "Policy saved", body = Data<TwoFactorPolicyResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` or two-factor setup required (`two_factor_setup_required`)", body = ErrorEnvelope),
        (status = 409, description = "The policy would cover the current user, who has no second factor (`two_factor_not_enabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid policy", body = ErrorEnvelope),
    )
)]
async fn update_policy(
    State(state): State<AppState>,
    tenant: AuthContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<UpdateTwoFactorPolicyRequest>,
) -> Result<Json<Data<TwoFactorPolicyResponse>>, ApiError> {
    let view = state
        .two_factor_use_cases()?
        .update_policy(
            &tenant.scope,
            UpdateTwoFactorPolicyCommand {
                policy: TwoFactorPolicy::from(request.policy).as_str().to_owned(),
                client,
            },
        )
        .await?;
    Ok(Json(Data::new(view.into())))
}
