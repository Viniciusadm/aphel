use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::dto::{
    ConfigureIntegrationRequest, IntegrationResponse, IntegrationTestResponse,
    TestIntegrationRequest,
};
use super::service;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::integrations::ConnectionTestOptions;
use aphel_kernel::context::Ctx;

pub(crate) const TAG: &str = "integrations";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_integrations))
        .routes(routes!(
            get_integration,
            configure_integration,
            remove_integration
        ))
        .routes(routes!(test_integration))
        .routes(routes!(activate_integration))
        .routes(routes!(deactivate_integration))
}

#[utoipa::path(
    get,
    path = "/integrations",
    tag = TAG,
    operation_id = "integrations_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Every available integration provider with the organization's instance state. `status` is `not_configured`, `inactive`, `active`, `error` or, for shortcut providers such as the generic webhook, `available`. Secret fields only say whether they are configured", body = Data<Vec<IntegrationResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_integrations(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<IntegrationResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let views = service::list(&ctx).await?;
    Ok(Json(Data::new(views.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    get,
    path = "/integrations/{provider_key}",
    tag = TAG,
    operation_id = "integrations_get",
    security(("session_cookie" = [])),
    params(("provider_key" = String, Path, description = "Provider key, such as `smtp`")),
    responses(
        (status = 200, description = "The provider and the organization's instance", body = Data<IntegrationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(provider_key): Path<String>,
) -> Result<Json<Data<IntegrationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::get(&ctx, &provider_key).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/integrations/{provider_key}",
    tag = TAG,
    operation_id = "integrations_configure",
    security(("session_cookie" = [])),
    params(
        ("provider_key" = String, Path, description = "Provider key, such as `smtp`"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the integration as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = ConfigureIntegrationRequest,
    responses(
        (status = 200, description = "Configuration saved (`integration.configured`). The whole non-secret configuration is replaced and validated against the provider's `fields`. Secret fields are stored encrypted and never returned; leaving one empty keeps the current value and `clear_secrets` removes optional ones. The status does not change", body = Data<IntegrationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The integration changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid configuration (`validation_error`) or a provider without configuration (`integration_not_configurable`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured and a secret was sent (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn configure_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(provider_key): Path<String>,
    IfMatch(expected_updated_at): IfMatch,
    JsonBody(request): JsonBody<ConfigureIntegrationRequest>,
) -> Result<Json<Data<IntegrationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::configure(
        &ctx,
        &provider_key,
        request.into_command(expected_updated_at, client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    delete,
    path = "/integrations/{provider_key}",
    tag = TAG,
    operation_id = "integrations_remove",
    security(("session_cookie" = [])),
    params(("provider_key" = String, Path, description = "Provider key, such as `smtp`")),
    responses(
        (status = 204, description = "Configuration and secrets removed (`integration.removed`); removing an unconfigured integration is harmless"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "A provider without configuration (`integration_not_configurable`)", body = ErrorEnvelope),
    )
)]
async fn remove_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(provider_key): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::remove(&ctx, &provider_key, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/integrations/{provider_key}/test",
    tag = TAG,
    operation_id = "integrations_test",
    security(("session_cookie" = [])),
    params(("provider_key" = String, Path, description = "Provider key, such as `smtp`")),
    request_body = TestIntegrationRequest,
    responses(
        (status = 200, description = "Connection tested with the saved configuration (`integration.tested`); `ok` and `message` carry the outcome, which is also kept in `last_tested_at`, `last_test_ok` and `recent_errors`. With `send_test_email`, the SMTP provider also sends a test email to the current user (`sent_to`). A failing test turns an active integration into `error`; a passing one brings it back to `active`", body = Data<IntegrationTestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Not configured or missing required fields (`integration_not_configured`) or a provider without configuration (`integration_not_configurable`)", body = ErrorEnvelope),
    )
)]
async fn test_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(provider_key): Path<String>,
    JsonBody(request): JsonBody<TestIntegrationRequest>,
) -> Result<Json<Data<IntegrationTestResponse>>, ApiError> {
    let sent_to = request
        .send_test_email
        .then(|| tenant.user.email.as_str().to_owned());
    let options = ConnectionTestOptions {
        send_test_email_to: sent_to.clone(),
    };
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let (view, result) = service::test(&ctx, &provider_key, options, client.0.ip).await?;
    Ok(Json(Data::new(IntegrationTestResponse::new(
        view, result, sent_to,
    ))))
}

#[utoipa::path(
    post,
    path = "/integrations/{provider_key}/activate",
    tag = TAG,
    operation_id = "integrations_activate",
    security(("session_cookie" = [])),
    params(("provider_key" = String, Path, description = "Provider key, such as `smtp`")),
    responses(
        (status = 200, description = "Integration active (`integration.activated`); activating an active one is harmless. The configuration must be complete, and providers that need it (SMTP) run a connection test first. While the organization SMTP is active, notification and invitation emails of the organization go through it", body = Data<IntegrationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Not configured or incomplete (`integration_not_configured`), connection test failed (`integration_test_failed`, recorded in `recent_errors`), activation hook failed (`integration_activation_failed`) or a provider without configuration (`integration_not_configurable`)", body = ErrorEnvelope),
    )
)]
async fn activate_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(provider_key): Path<String>,
) -> Result<Json<Data<IntegrationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::activate(&ctx, &provider_key, client.0.ip).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/integrations/{provider_key}/deactivate",
    tag = TAG,
    operation_id = "integrations_deactivate",
    security(("session_cookie" = [])),
    params(("provider_key" = String, Path, description = "Provider key, such as `smtp`")),
    responses(
        (status = 200, description = "Integration inactive (`integration.deactivated`); the configuration is kept. Deactivating an inactive one is harmless", body = Data<IntegrationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown provider (`integration_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Not configured (`integration_not_configured`) or a provider without configuration (`integration_not_configurable`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_integration(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(provider_key): Path<String>,
) -> Result<Json<Data<IntegrationResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::deactivate(&ctx, &provider_key, client.0.ip).await?;
    Ok(Json(Data::new(view.into())))
}
