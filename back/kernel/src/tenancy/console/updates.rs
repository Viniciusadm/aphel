use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::JsonBody;
use crate::application::system::updates::{ApplySystemUpdate, SystemUpdateStatus};
use crate::infrastructure::system::updates;
use crate::tenancy::console::{OperatorSession, Shared};
use axum::Json;
use utoipa_axum::{router::OpenApiRouter, routes};

pub(super) fn router() -> OpenApiRouter<Shared> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(check))
        .routes(routes!(prepare))
        .routes(routes!(apply))
}

#[utoipa::path(
    get, path = "/updates", tag = "console",
    operation_id = "console_updates_status", description = "Update service availability, installed version, latest stable release, operation and history",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn status(session: OperatorSession) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    let _ = session;
    Ok(Json(Data::new(
        updates::request("status", None, None).await?,
    )))
}

#[utoipa::path(
    post, path = "/updates/check", tag = "console",
    operation_id = "console_updates_check", description = "Check the latest stable release without changing the running application",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn check(session: OperatorSession) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    let actor = session.0.operator.email.clone();
    Ok(Json(Data::new(
        updates::request("check", Some(&actor), None).await?,
    )))
}

#[utoipa::path(
    post, path = "/updates/prepare", tag = "console",
    operation_id = "console_updates_prepare", description = "Download and verify the latest stable image; does not restart the application",
    responses(
        (status = 200, description = "Update status", body = Data<SystemUpdateStatus>),
        (status = 401, description = "Authentication required", body = ErrorEnvelope),
        (status = 403, description = "Not allowed in this context", body = ErrorEnvelope),
        (status = 409, description = "Update state changed or another operation is running", body = ErrorEnvelope),
        (status = 503, description = "External update service unavailable", body = ErrorEnvelope),
    )
)]
async fn prepare(session: OperatorSession) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    let actor = session.0.operator.email.clone();
    Ok(Json(Data::new(
        updates::request("prepare", Some(&actor), None).await?,
    )))
}

#[utoipa::path(
    post, path = "/updates/apply", tag = "console",
    operation_id = "console_updates_apply", description = "Confirm a prepared update; the external service backs up data, restarts the installation and records the outcome",
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
    session: OperatorSession,
    JsonBody(input): JsonBody<ApplySystemUpdate>,
) -> Result<Json<Data<SystemUpdateStatus>>, ApiError> {
    if !input.confirm || uuid::Uuid::parse_str(&input.operation_id).is_err() {
        return Err(
            crate::errors::AppError::Validation("confirm the prepared update".to_owned()).into(),
        );
    }
    let actor = session.0.operator.email.clone();
    Ok(Json(Data::new(
        updates::request("apply", Some(&actor), Some(&input.operation_id)).await?,
    )))
}
