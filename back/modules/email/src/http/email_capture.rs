use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_sync::{EmailCaptureSettingsDto, UpdateEmailCaptureSettingsRequest};
use crate::service::capture;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "email settings";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_capture_settings, update_capture_settings))
}

#[utoipa::path(
    get,
    path = "/email-settings/capture",
    tag = TAG,
    operation_id = "email_capture_settings_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Relationship capture settings of the organization (contact and company creation are off by default)", body = Data<EmailCaptureSettingsDto>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn get_capture_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<EmailCaptureSettingsDto>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = capture::settings(&ctx).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    put,
    path = "/email-settings/capture",
    tag = TAG,
    operation_id = "email_capture_settings_update",
    security(("session_cookie" = [])),
    request_body = UpdateEmailCaptureSettingsRequest,
    responses(
        (status = 200, description = "Capture settings saved (omitted fields keep their value); publishes `email_capture_settings.updated` with audit", body = Data<EmailCaptureSettingsDto>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "`initial_history_days` out of 1–3650 (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_capture_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<UpdateEmailCaptureSettingsRequest>,
) -> Result<Json<Data<EmailCaptureSettingsDto>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = capture::update_settings(&ctx, request.into_command(client.0.ip)).await?;
    Ok(Json(Data::new(settings.into())))
}
