use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::auth::MeResponse;
use crate::api::dto::common::ErrorEnvelope;
use crate::api::dto::installer::{InstallRequest, InstallerStatusResponse};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody};
use crate::api::session_cookie;
use crate::api::state::AppState;
use crate::application::auth::LoginCommand;
use crate::application::installer::{InstallCommand, InstallerUseCases};
use crate::domain::Secret;
use crate::errors::AppError;
use crate::infrastructure::installer::MySqlInstallerPorts;

pub const TAG: &str = "installer";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(installer_status, install))
}

#[utoipa::path(
    get,
    path = "/installer",
    tag = TAG,
    operation_id = "installer_status",
    description = "Tells whether the installation wizard is available: no organization exists and the installation was never completed.",
    responses(
        (status = 200, description = "The installer is available", body = InstallerStatusResponse),
        (status = 404, description = "The instance is already installed (`not_found`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn installer_status(
    State(state): State<AppState>,
) -> Result<Json<InstallerStatusResponse>, ApiError> {
    use_cases(&state)?.status().await?;
    Ok(Json(InstallerStatusResponse { available: true }))
}

#[utoipa::path(
    post,
    path = "/installer",
    tag = TAG,
    operation_id = "installer_install",
    description = "Creates the organization with its administrator (email already verified), marks the instance as installed in the same transaction and starts a session (sets the `aphel_session` cookie). `public_base_url` is the address the browser used (its origin); it is saved as the public address used in email links and decides the `Secure` cookie flag, and stays editable in the settings. Works only once.",
    request_body = InstallRequest,
    responses(
        (status = 201, description = "Instance installed, session started", body = MeResponse),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`) or demo mode (`demo_restricted`)", body = ErrorEnvelope),
        (status = 404, description = "The instance is already installed (`not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Email already registered (`email_taken`)", body = ErrorEnvelope),
        (status = 413, description = "Request body too large (`payload_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error` with `details.fields`: organization name, timezone, currency, locale, administrator name, email, password of 8 to 128 characters, public address)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn install(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<InstallRequest>,
) -> Result<Response, ApiError> {
    let password = Secret::new(request.admin_password);
    let installation = use_cases(&state)?
        .install(InstallCommand {
            organization_name: request.organization_name,
            timezone: request.timezone,
            currency: request.currency,
            locale: request.locale,
            admin_name: request.admin_name,
            admin_email: request.admin_email,
            admin_password: password.clone(),
            public_base_url: request.public_base_url,
            ip: client.ip.clone(),
        })
        .await?;
    state.organization_config.refresh().await;
    let session = state
        .auth_use_cases()?
        .login(LoginCommand {
            email: installation.admin_email,
            password,
            client,
        })
        .await?
        .into_session()?;
    let cookie = session_cookie::issue(
        &session.token,
        session.expires_at,
        Utc::now(),
        state.secure_cookies(),
    )?;
    let mut response =
        (StatusCode::CREATED, Json(MeResponse::from(session.account))).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

fn use_cases(state: &AppState) -> Result<InstallerUseCases<MySqlInstallerPorts>, AppError> {
    Ok(InstallerUseCases::new(MySqlInstallerPorts::new(
        state.db()?,
        state.record_types.builtin(),
        state.events.clone(),
        state.audit.clone(),
    )))
}
