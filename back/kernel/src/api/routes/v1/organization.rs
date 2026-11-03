use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::organization::{OrganizationResponse, UpdateOrganizationRequest};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::state::AppState;
use crate::application::organization_profile::OrganizationUseCases;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationPorts;

pub const TAG: &str = "organization";

const LOGO_CONTENT_SECURITY_POLICY: &str = "default-src 'none'; sandbox";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(current_organization, update_organization))
        .routes(routes!(organization_logo))
}

#[utoipa::path(
    get,
    path = "/organization",
    tag = TAG,
    operation_id = "organization_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Active organization", body = Data<OrganizationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "No active organization", body = ErrorEnvelope),
    )
)]
async fn current_organization(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<Data<OrganizationResponse>>, ApiError> {
    let organization = use_cases(&state)?.current().await?;
    Ok(Json(Data::new(organization.into())))
}

#[utoipa::path(
    patch,
    path = "/organization",
    tag = TAG,
    operation_id = "organization_update",
    description = "Updates name, primary color, locale, time zone, currency and logo. `logo_file_id` is a PNG, JPEG, WebP or GIF of at most 2 MB uploaded through `POST /files`; `null` removes the logo and omitting it keeps the current one. Publishes `organization.updated` and writes the audit entry.",
    security(("session_cookie" = [])),
    request_body = UpdateOrganizationRequest,
    responses(
        (status = 200, description = "Updated organization", body = Data<OrganizationResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields", body = ErrorEnvelope),
    )
)]
async fn update_organization(
    State(state): State<AppState>,
    tenant: AuthContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<UpdateOrganizationRequest>,
) -> Result<Json<Data<OrganizationResponse>>, ApiError> {
    let organization = use_cases(&state)?
        .update(&tenant.scope, request.into_command(client.ip))
        .await?;
    Ok(Json(Data::new(organization.into())))
}

#[utoipa::path(
    get,
    path = "/organization/logo",
    tag = TAG,
    operation_id = "organization_logo",
    description = "Streams the logo of the active organization to any member.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Logo image", content_type = "image/*"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "The organization has no logo (`logo_not_found`)", body = ErrorEnvelope),
    )
)]
async fn organization_logo(State(state): State<AppState>) -> Result<Response, ApiError> {
    let download = use_cases(&state)?.logo().await?;
    let content_type = HeaderValue::from_str(&download.file.mime_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, download.file.size)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .header(
            header::CONTENT_SECURITY_POLICY,
            LOGO_CONTENT_SECURITY_POLICY,
        )
        .body(Body::from_stream(download.content))
        .map_err(|error| AppError::Internal(format!("cannot build logo response: {error}")))?;
    Ok(response)
}

fn use_cases(state: &AppState) -> Result<OrganizationUseCases<MySqlOrganizationPorts>, AppError> {
    Ok(OrganizationUseCases::new(MySqlOrganizationPorts::new(
        state.db()?,
        state.storage.clone(),
        state.events.clone(),
        state.audit.clone(),
    )))
}
