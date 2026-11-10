use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::organization_settings::{
    OrganizationSettingsResponse, StorageTestRequest, StorageTestResponse,
    UpdateOrganizationSettingsRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::state::AppState;
use crate::application::organization_profile::OrganizationConfigUseCases;
use crate::domain::Secret;
use crate::errors::AppError;
use crate::infrastructure::organization_profile::MySqlOrganizationConfigPorts;

pub const TAG: &str = "organization";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_settings, update_settings))
        .routes(routes!(test_storage))
}

fn use_cases(
    state: &AppState,
) -> Result<OrganizationConfigUseCases<MySqlOrganizationConfigPorts>, AppError> {
    Ok(OrganizationConfigUseCases::new(
        MySqlOrganizationConfigPorts::new(
            state.db()?,
            state.secrets.clone(),
            state.events.clone(),
            state.audit.clone(),
            state.organization_config.clone(),
        ),
    ))
}

#[utoipa::path(
    get,
    path = "/organization/settings",
    tag = TAG,
    operation_id = "organization_settings_get",
    description = "Installation settings edited on screen (permission `settings.manage`): public address, upload limit, file storage, webhook delivery, API token limits and the community catalog. Secrets are never returned: `storage_secret_configured` tells whether the S3 secret key is stored, and `mail_configured` whether the outgoing email (the `smtp` integration) is active.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Current settings", body = Data<OrganizationSettingsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn get_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<OrganizationSettingsResponse>>, ApiError> {
    let view = use_cases(&state)?.get(&tenant.scope).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    patch,
    path = "/organization/settings",
    tag = TAG,
    operation_id = "organization_settings_update",
    description = "Updates the sections that are present; omitted sections keep their value. `storage_secret_access_key` stores the S3 secret key in the vault (`null` or an empty string removes it). Changes apply without restarting the server or the worker. Publishes `organization.settings_updated` (payload with the names of the changed settings, never values of secrets) and writes the audit entry with secrets as `\"[redacted]\"`. Changing the storage driver does not copy existing files. `app_base_path` moves the internal app (`/` or a single segment such as `/app`) and also requires `system.manage`; the app always stays reachable at `/_aphel/app/`.",
    security(("session_cookie" = [])),
    request_body = UpdateOrganizationSettingsRequest,
    responses(
        (status = 200, description = "Updated settings", body = Data<OrganizationSettingsResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage`, or `system.manage` when changing `app_base_path` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields", body = ErrorEnvelope),
    )
)]
async fn update_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<UpdateOrganizationSettingsRequest>,
) -> Result<Json<Data<OrganizationSettingsResponse>>, ApiError> {
    let view = use_cases(&state)?
        .update(&tenant.scope, request.into_command(client.ip))
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/organization/settings/storage/test",
    tag = TAG,
    operation_id = "organization_settings_test_storage",
    description = "Tests a file storage configuration without saving it: writes, reads and deletes a small object. Without `secret_access_key`, an S3 test uses the stored secret key. The outcome is in `ok` and `message`.",
    security(("session_cookie" = [])),
    request_body = StorageTestRequest,
    responses(
        (status = 200, description = "Test outcome", body = Data<StorageTestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid storage fields", body = ErrorEnvelope),
    )
)]
async fn test_storage(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<StorageTestRequest>,
) -> Result<Json<Data<StorageTestResponse>>, ApiError> {
    let secret = request
        .secret_access_key
        .filter(|value| !value.is_empty())
        .map(Secret::new);
    let outcome = use_cases(&state)?
        .test_storage(&tenant.scope, request.storage.into(), secret)
        .await?;
    Ok(Json(Data::new(outcome.into())))
}
