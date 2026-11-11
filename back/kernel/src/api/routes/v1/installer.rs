use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::auth::MeResponse;
use crate::api::dto::common::Data;
use crate::api::dto::common::ErrorEnvelope;
use crate::api::dto::installer::{InstallRequest, InstallerStatusResponse};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::session_cookie;
use crate::api::state::AppState;
use crate::application::access::Authorizer;
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
        .routes(routes!(installed_blueprint))
        .routes(routes!(reapply_blueprint))
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
    let installation = install_organization(
        &state,
        InstallCommand {
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
        },
    )
    .await?;
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

pub(crate) async fn install_organization(
    state: &AppState,
    command: InstallCommand,
) -> Result<crate::application::installer::Installation, AppError> {
    let business = command.business.clone();
    let installation = use_cases(state)?.install(command).await?;
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
    let outcome = apply_blueprint(state, &scope, business.as_ref()).await;
    record_configuration(state, outcome).await;
    Ok(installation)
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
        None,
    )?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct BlueprintResponse {
    pub blueprint: Option<crate::application::installer::modules::InstallerIdentity>,
    pub configuration_error: Option<String>,
}

#[utoipa::path(
    get,
    path = "/blueprint",
    tag = TAG,
    operation_id = "installed_blueprint",
    security(("session_cookie" = [])),
    description = "The blueprint the instance was installed from (key, name and navigation home), or `null` for a blank system.",
    responses(
        (status = 200, description = "The installed blueprint", body = Data<BlueprintResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
    )
)]
async fn installed_blueprint(
    State(state): State<AppState>,
    _tenant: AuthContext,
) -> Result<Json<Data<BlueprintResponse>>, ApiError> {
    let installer = installer_settings(&state).await?;
    let key = installer
        .as_ref()
        .and_then(|value| value.get("business_type"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let blueprint = key.and_then(|key| {
        state
            .app
            .contributions::<dyn crate::application::installer::InstallerBlueprintSource>()
            .into_iter()
            .find_map(|source| source.identity(&key))
    });
    let configuration_error = installer
        .as_ref()
        .and_then(|value| value.get("configuration_error"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Ok(Json(Data::new(BlueprintResponse {
        blueprint,
        configuration_error,
    })))
}

#[utoipa::path(
    post,
    path = "/blueprint/configuration",
    tag = TAG,
    operation_id = "installed_blueprint_reapply",
    security(("session_cookie" = [])),
    description = "Applies the installed blueprint's roles and configuration packages again, for example after the first attempt failed. Needs `settings.manage`. Configuration packages reconcile by stable key, so applying twice changes nothing.",
    responses(
        (status = 200, description = "Applied", body = Data<BlueprintResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn reapply_blueprint(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<BlueprintResponse>>, ApiError> {
    tenant
        .scope
        .require(&crate::domain::access::Permission::SETTINGS_MANAGE)?;
    let business = installer_settings(&state).await?;
    let outcome = apply_blueprint(&state, &tenant.scope, business.as_ref()).await;
    record_configuration(&state, outcome).await;
    installed_blueprint(State(state), tenant).await
}

async fn installer_settings(state: &AppState) -> Result<Option<serde_json::Value>, AppError> {
    let document: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT JSON_EXTRACT(settings, '$.installer') FROM organization_settings WHERE id = 1",
    )
    .fetch_optional(&state.db()?)
    .await?
    .flatten();
    Ok(document)
}

async fn apply_blueprint(
    state: &AppState,
    scope: &crate::domain::access::AccessScope,
    business: Option<&serde_json::Value>,
) -> Result<(), AppError> {
    install_blueprint_roles(state, business).await?;
    let context = crate::Ctx::from_state(state, scope).await?;
    crate::application::installer::apply_blueprint_configuration(&context, business).await
}

async fn install_blueprint_roles(
    state: &AppState,
    business: Option<&serde_json::Value>,
) -> Result<(), AppError> {
    let pool = state.db()?;
    let known: std::collections::HashSet<String> =
        crate::domain::access::permission::permission_catalog()
            .into_iter()
            .map(|definition| definition.permission.as_str().to_owned())
            .collect();
    for source in state
        .app
        .contributions::<dyn crate::application::installer::InstallerBlueprintSource>()
    {
        for role in source.roles(business) {
            sqlx::query(
                "INSERT IGNORE INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, FALSE)",
            )
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&role.key)
            .bind(&role.label)
            .execute(&pool)
            .await?;
            let id: String = sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
                .bind(&role.key)
                .fetch_one(&pool)
                .await?;
            for permission in role
                .permissions
                .iter()
                .filter(|permission| known.contains(*permission))
            {
                sqlx::query(
                    "INSERT IGNORE INTO role_permissions (role_id, permission, scope) VALUES (?, ?, 'organization')",
                )
                .bind(&id)
                .bind(permission)
                .execute(&pool)
                .await?;
            }
        }
    }
    Ok(())
}

async fn record_configuration(state: &AppState, outcome: Result<(), AppError>) {
    let Ok(pool) = state.db() else {
        return;
    };
    let result = match outcome {
        Ok(()) => {
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_REMOVE(COALESCE(settings, JSON_OBJECT()), '$.installer.configuration_error') WHERE id = 1",
            )
            .execute(&pool)
            .await
        }
        Err(error) => {
            tracing::error!("blueprint configuration failed: {error}");
            sqlx::query(
                "UPDATE organization_settings SET settings = JSON_SET(COALESCE(settings, JSON_OBJECT()), '$.installer.configuration_error', ?) WHERE id = 1",
            )
            .bind(error.to_string())
            .execute(&pool)
            .await
        }
    };
    if let Err(error) = result {
        tracing::error!("could not record the blueprint configuration state: {error}");
    }
}
