use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_outbound::{
    CreateTrackingObjectionRequest, EmailTrackingSettingsDto, TrackingObjectionResponse,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "email tracking";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_tracking_settings, update_tracking_settings))
        .routes(routes!(list_objections, create_objection))
        .routes(routes!(delete_objection))
}

#[utoipa::path(
    get,
    path = "/email-settings/tracking",
    tag = TAG,
    operation_id = "email_tracking_settings_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Open/click tracking of the organization (all off by default) and whether new e-mails are tracked by default", body = Data<EmailTrackingSettingsDto>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` or `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn get_tracking_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<EmailTrackingSettingsDto>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = service::tracking::settings(&ctx).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    put,
    path = "/email-settings/tracking",
    tag = TAG,
    operation_id = "email_tracking_settings_update",
    security(("session_cookie" = [])),
    request_body = EmailTrackingSettingsDto,
    responses(
        (status = 200, description = "Settings saved (`email_tracking.updated` with audit)", body = Data<EmailTrackingSettingsDto>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_tracking_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<EmailTrackingSettingsDto>,
) -> Result<Json<Data<EmailTrackingSettingsDto>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let settings = service::tracking::update_settings(&ctx, request.into(), client.0.ip).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    get,
    path = "/email-settings/tracking/objections",
    tag = TAG,
    operation_id = "email_tracking_objections_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Addresses that objected to tracking (masked; only a hash of the address is stored)", body = Data<Vec<TrackingObjectionResponse>>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_objections(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<TrackingObjectionResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let objections = service::tracking::objections(&ctx).await?;
    Ok(Json(Data::new(
        objections.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/email-settings/tracking/objections",
    tag = TAG,
    operation_id = "email_tracking_objections_create",
    description = "Registers that the owner of an address objected to open/click tracking (LGPD, art. 18, § 2º). E-mails to that address are sent without tracking and opens/clicks of e-mails already sent stop being recorded. The address travels in the body and is stored only as a SHA-256 hash with a masked label.",
    security(("session_cookie" = [])),
    request_body = CreateTrackingObjectionRequest,
    responses(
        (status = 201, description = "Objection registered (`email_tracking.objection_added` with audit)", body = Data<TrackingObjectionResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The address already objected (`email_tracking_objection_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid address (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_objection(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateTrackingObjectionRequest>,
) -> Result<(StatusCode, Json<Data<TrackingObjectionResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let objection =
        service::tracking::add_objection(&ctx, &request.address, request.reason, client.0.ip)
            .await?;
    Ok((StatusCode::CREATED, Json(Data::new(objection.into()))))
}

#[utoipa::path(
    delete,
    path = "/email-settings/tracking/objections/{objection_id}",
    tag = TAG,
    operation_id = "email_tracking_objections_delete",
    security(("session_cookie" = [])),
    params(("objection_id" = String, Path, description = "Objection id")),
    responses(
        (status = 204, description = "Objection removed (`email_tracking.objection_removed` with audit)"),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `privacy.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown objection (`email_tracking_objection_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_objection(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(objection_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::tracking::remove_objection(&ctx, &objection_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}
