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
    OpenApiRouter::new()
        .routes(routes!(installer_status, install))
        .routes(routes!(validate_modules))
}

#[utoipa::path(
    get,
    path = "/installer",
    tag = TAG,
    operation_id = "installer_status",
    description = "Returns installer availability, compiled module metadata and profile recommendations. Only available before the company is installed. Secret values are never returned.",
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
    Ok(Json(InstallerStatusResponse {
        available: true,
        modules: state
            .app
            .modules()
            .iter()
            .map(
                |entry| crate::api::dto::installer::InstallerModuleResponse {
                    key: entry.key().to_owned(),
                    name: entry.manifest.name.to_owned(),
                    description: entry.manifest.description.to_owned(),
                    required: entry.manifest.required,
                    compatible: entry.is_compatible(),
                    dependencies: entry
                        .manifest
                        .dependencies
                        .iter()
                        .map(|dependency| dependency.key.to_owned())
                        .collect(),
                    settings_schema: entry
                        .manifest
                        .settings
                        .iter()
                        .map(|field| {
                            let mut schema =
                                crate::api::dto::modules::ModuleSettingFieldResponse::from(field);
                            if field.is_secret() {
                                schema.default = None;
                            }
                            schema
                        })
                        .collect(),
                },
            )
            .collect(),
        profiles: crate::application::installer::modules::profiles(),
    }))
}

#[utoipa::path(
    post,
    path = "/installer",
    tag = TAG,
    operation_id = "installer_install",
    description = "Creates the organization with its administrator (email already verified), marks the instance as installed in the same transaction and starts a session (sets the `aphel_session` cookie). `public_base_url` is the address the browser used (its origin); it is saved as the public address used in email links and decides the `Secure` cookie flag, and stays editable in the settings. `business` holds the answers about the business (`business_type` is a key from `GET /installer` `profiles`, `offering` any of `product`, `service`, `recurring`, `team_size` one of `solo`, `small`, `medium`, `large`); they are kept in the organization settings so later setup steps reuse them instead of asking again. Optional modules default to disabled. The selected modules, their transitive dependencies and validated settings are stored in the installation transaction; secret settings go to the secret vault. Works only once.",
    request_body = InstallRequest,
    responses(
        (status = 201, description = "Instance installed, session started", body = MeResponse),
        (status = 403, description = "Missing `X-Requested-With: aphel` (`csrf_header_missing`) or demo mode (`demo_restricted`)", body = ErrorEnvelope),
        (status = 404, description = "The instance is already installed (`not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Email already registered (`email_taken`)", body = ErrorEnvelope),
        (status = 413, description = "Request body too large (`payload_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error` with `details.fields`: organization name, timezone, currency, locale, administrator name, email, password of 8 to 128 characters, public address, business answers, modules and module settings)", body = ErrorEnvelope),
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
            modules: request.modules,
            module_settings: request.module_settings,
            organization_name: request.organization_name,
            timezone: request.timezone,
            currency: request.currency,
            locale: request.locale,
            admin_name: request.admin_name,
            admin_email: request.admin_email,
            admin_password: password.clone(),
            business: request.business,
            public_base_url: request.public_base_url,
            ip: client.ip.clone(),
        })
        .await?;
    state.app.enabled().refresh(&state.db()?).await?;
    let pool = state.db()?;
    let scope = crate::infrastructure::repositories::mysql_access::build_requester_scope(
        &pool,
        &installation.user_id,
        None,
    )
    .await?
    .ok_or_else(|| {
        AppError::Internal("the installation administrator has no access scope".to_owned())
    })?;
    for entry in state
        .app
        .modules()
        .iter()
        .filter(|entry| !entry.manifest.required && state.app.is_enabled(entry.key()))
    {
        let mut context = crate::modules::ModuleCtx {
            pool: &pool,
            scope: &scope,
            services: state.app.services(),
            module: entry.key(),
        };
        if let Err(error) = entry.module.on_enable(&mut context).await {
            tracing::error!(module = entry.key(), "on_enable failed: {error}");
        }
    }
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
        state.secrets.clone(),
    )))
}

#[utoipa::path(
    post,
    path = "/installer/modules/validate",
    tag = TAG,
    operation_id = "installer_validate_modules",
    description = "Validates module selection, dependencies and settings without storing credentials or changing the installation.",
    request_body = crate::api::dto::installer::InstallerModulesRequest,
    responses(
        (status = 204, description = "Module selection is valid"),
        (status = 404, description = "The instance is already installed", body = ErrorEnvelope),
        (status = 422, description = "Invalid modules or settings", body = ErrorEnvelope),
    )
)]
async fn validate_modules(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<crate::api::dto::installer::InstallerModulesRequest>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?.status().await?;
    crate::application::installer::modules::prepare(
        &state.app,
        &request.modules,
        &request.module_settings,
    )?;
    Ok(StatusCode::NO_CONTENT)
}
