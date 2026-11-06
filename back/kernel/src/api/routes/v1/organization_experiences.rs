use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use crate::api::state::AppState;
use crate::application::organization_profile::{
    OrganizationExperiences, OrganizationExperiencesUseCases, OrganizationExperiencesView,
};
use crate::errors::AppError;
use crate::infrastructure::organization_profile::MySqlOrganizationExperiencesPorts;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_experiences, replace_experiences))
        .routes(routes!(validate_experiences))
}

async fn use_cases(
    state: &AppState,
) -> Result<OrganizationExperiencesUseCases<MySqlOrganizationExperiencesPorts>, AppError> {
    Ok(OrganizationExperiencesUseCases::new(
        MySqlOrganizationExperiencesPorts::new(
            state.db()?,
            state.registry().await?,
            state.events.clone(),
            state.audit.clone(),
        ),
    ))
}

#[utoipa::path(
    get,
    path = "/organization/experiences",
    tag = "organization",
    operation_id = "organization_experiences_get",
    description = "Reads the organization-owned experience catalog and policy. Any authenticated member can read. A null configuration preserves the existing application baseline. Provider availability is not inferred from configuration: required provider and optional section diagnostics identify the checks the frontend host must perform. This endpoint does not select a personal profile or execute frontend code.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Configuration and organization-row updated_at for If-Match", body = Data<OrganizationExperiencesView>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 404, description = "Organization not installed", body = ErrorEnvelope),
    )
)]
async fn get_experiences(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<OrganizationExperiencesView>>, ApiError> {
    Ok(Json(Data::new(
        use_cases(&state).await?.get(&tenant.scope).await?,
    )))
}

#[utoipa::path(
    put,
    path = "/organization/experiences",
    tag = "organization",
    operation_id = "organization_experiences_replace",
    description = "Atomically replaces the catalog, default and organization customization limits. Requires settings.manage and an exact If-Match from GET; stale configuration returns version_conflict. Duplicate/unsupported/unknown properties and dangling default/allowlist references are rejected. Remove or disable a profile together with its policy/default references in the same request; personal references are not deleted. Writes organization.experiences_updated and audit in the same transaction. Preserves unrelated organization settings and does not install/enable plugins or apply a personal selection.",
    security(("session_cookie" = [])),
    params(("If-Match" = String, Header, description = "Required exact organization updated_at in RFC3339 from GET")),
    request_body = OrganizationExperiences,
    responses(
        (status = 200, description = "Saved configuration", body = Data<OrganizationExperiencesView>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing settings.manage", body = ErrorEnvelope),
        (status = 404, description = "Organization not installed", body = ErrorEnvelope),
        (status = 409, description = "Stale If-Match (version_conflict)", body = ErrorEnvelope),
        (status = 422, description = "Invalid format, catalog, default, policy or missing If-Match", body = ErrorEnvelope),
    )
)]
async fn replace_experiences(
    State(state): State<AppState>,
    tenant: AuthContext,
    IfMatch(expected): IfMatch,
    ClientInfo(client): ClientInfo,
    JsonBody(configuration): JsonBody<OrganizationExperiences>,
) -> Result<Json<Data<OrganizationExperiencesView>>, ApiError> {
    Ok(Json(Data::new(
        use_cases(&state)
            .await?
            .replace(&tenant.scope, configuration, expected, client.ip)
            .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/organization/experiences/validate",
    tag = "organization",
    operation_id = "organization_experiences_validate",
    description = "Validates a proposed declarative catalog without writing it. Requires settings.manage. Rejects unsupported profile formats and unknown properties. Reports unavailable record types and required frontend provider/section verification; it does not install code or certify frontend compatibility. The returned updated_at is the current organization version, not a reservation for a later PUT.",
    security(("session_cookie" = [])),
    request_body = OrganizationExperiences,
    responses(
        (status = 200, description = "Proposed configuration and dependency diagnostics; nothing saved", body = Data<OrganizationExperiencesView>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing settings.manage", body = ErrorEnvelope),
        (status = 404, description = "Organization not installed", body = ErrorEnvelope),
        (status = 422, description = "Invalid or unsupported catalog", body = ErrorEnvelope),
    )
)]
async fn validate_experiences(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(configuration): JsonBody<OrganizationExperiences>,
) -> Result<Json<Data<OrganizationExperiencesView>>, ApiError> {
    Ok(Json(Data::new(
        use_cases(&state)
            .await?
            .preview(&tenant.scope, configuration)
            .await?,
    )))
}
