use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::JsonBody;
use crate::api::extractors::{AuthContext, Credential};
use crate::api::state::AppState;
use crate::application::access::Authorizer;
use crate::application::system::updates::{ApplySystemUpdate, SystemUpdateStatus};
use crate::domain::access::Permission;
use crate::errors::AppError;
use crate::infrastructure::system::updates;
use axum::Json;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(check))
        .routes(routes!(prepare))
        .routes(routes!(apply))
}

fn authorize(admin: &AuthContext) -> Result<(), AppError> {
    admin.scope.require(&Permission::SYSTEM_MANAGE)?;
    if !matches!(admin.credential, Credential::Session(_)) {
        return Err(AppError::forbidden_code(
            "api_token_not_allowed",
            "system updates require a browser session",
        ));
    }
    Ok(())
}

fn require_single() -> Result<(), AppError> {
    if crate::tenancy::is_multi() {
        Err(AppError::forbidden_code(
            "system_updates_console_only",
            "updates of a multi-tenant installation are managed from the operator console",
        ))
    } else {
        Ok(())
    }
}

#[utoipa::path(
    get, path = "/system/updates", tag = "system",
    operation_id = "system_updates_status", description = "Update service availability, installed version, latest stable release, operation and history",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn status(admin: AuthContext) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    authorize(&admin)?;
    if crate::tenancy::is_multi() {
        return Ok(Json(Data::new(SystemUpdateStatus {
            managed_by_console: true,
            ..Default::default()
        })));
    }
    Ok(Json(Data::new(
        updates::request("status", None, None).await?,
    )))
}

#[utoipa::path(
    post, path = "/system/updates/check", tag = "system",
    operation_id = "system_updates_check", description = "Check the latest stable release without changing the running application",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn check(admin: AuthContext) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    authorize(&admin)?;
    require_single()?;
    let actor = admin.user.email.as_str().to_owned();
    Ok(Json(Data::new(
        updates::request("check", Some(&actor), None).await?,
    )))
}

#[utoipa::path(
    post, path = "/system/updates/prepare", tag = "system",
    operation_id = "system_updates_prepare", description = "Download and verify the latest stable image; does not restart the application",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn prepare(admin: AuthContext) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    authorize(&admin)?;
    require_single()?;
    let actor = admin.user.email.as_str().to_owned();
    Ok(Json(Data::new(
        updates::request("prepare", Some(&actor), None).await?,
    )))
}

#[utoipa::path(
    post, path = "/system/updates/apply", tag = "system",
    operation_id = "system_updates_apply", description = "Confirm a prepared update; the external service backs up data, restarts the installation and records the outcome",
    request_body = ApplySystemUpdate,
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn apply(
    admin: AuthContext,
    JsonBody(input): JsonBody<ApplySystemUpdate>,
) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    authorize(&admin)?;
    require_single()?;
    if !input.confirm || uuid::Uuid::parse_str(&input.operation_id).is_err() {
        return Err(
            crate::errors::AppError::Validation("confirm the prepared update".to_owned()).into(),
        );
    }
    let actor = admin.user.email.as_str().to_owned();
    Ok(Json(Data::new(
        updates::request("apply", Some(&actor), Some(&input.operation_id)).await?,
    )))
}
