use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    RenewalOwnerMode, RenewalSettings, RenewalSettingsView, SaveRenewalSettingsCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "workflow";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_pipeline_renewals))
        .routes(routes!(get_pipeline_renewal, update_pipeline_renewal))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RenewalOwnerMode)]
pub(crate) enum RenewalOwnerModeSchema {
    Original,
    None,
}

impl From<RenewalOwnerMode> for RenewalOwnerModeSchema {
    fn from(mode: RenewalOwnerMode) -> Self {
        match mode {
            RenewalOwnerMode::Original => Self::Original,
            RenewalOwnerMode::Unassigned => Self::None,
        }
    }
}

impl From<RenewalOwnerModeSchema> for RenewalOwnerMode {
    fn from(mode: RenewalOwnerModeSchema) -> Self {
        match mode {
            RenewalOwnerModeSchema::Original => Self::Original,
            RenewalOwnerModeSchema::None => Self::Unassigned,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RenewalSettingsResponse {
    pub(crate) pipeline_id: String,
    pub(crate) configured: bool,
    pub(crate) is_enabled: bool,
    #[schema(example = 30)]
    pub(crate) days_before: u16,
    pub(crate) target_pipeline_id: String,
    pub(crate) target_stage_id: String,
    #[schema(example = "Renewal: ")]
    pub(crate) title_prefix: String,
    pub(crate) owner_mode: RenewalOwnerModeSchema,
    pub(crate) updated_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: Option<String>,
}

impl RenewalSettingsResponse {
    fn build(settings: RenewalSettings, configured: bool) -> Self {
        Self {
            pipeline_id: settings.pipeline_id.to_string(),
            configured,
            is_enabled: settings.is_enabled,
            days_before: settings.days_before,
            target_pipeline_id: settings.target_pipeline_id.to_string(),
            target_stage_id: settings.target_stage_id.to_string(),
            title_prefix: settings.title_prefix,
            owner_mode: settings.owner_mode.into(),
            updated_by: settings.updated_by.map(|id| id.into_inner()),
            updated_at: configured.then(|| timestamp(settings.updated_at)),
        }
    }
}

impl From<RenewalSettingsView> for RenewalSettingsResponse {
    fn from(view: RenewalSettingsView) -> Self {
        Self::build(view.settings, view.configured)
    }
}

impl From<RenewalSettings> for RenewalSettingsResponse {
    fn from(settings: RenewalSettings) -> Self {
        Self::build(settings, true)
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct RenewalSettingsRequest {
    pub(crate) is_enabled: Option<bool>,
    #[schema(example = 30)]
    pub(crate) days_before: Option<i64>,
    pub(crate) target_pipeline_id: Option<String>,
    pub(crate) target_stage_id: Option<String>,
    #[schema(example = "Renewal: ")]
    pub(crate) title_prefix: Option<String>,
    pub(crate) owner_mode: Option<RenewalOwnerModeSchema>,
}

impl RenewalSettingsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveRenewalSettingsCommand {
        SaveRenewalSettingsCommand {
            is_enabled: self.is_enabled,
            days_before: self.days_before,
            target_pipeline_id: self.target_pipeline_id,
            target_stage_id: self.target_stage_id,
            title_prefix: self.title_prefix,
            owner_mode: self.owner_mode.map(Into::into),
            ip,
            expected_updated_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/pipeline-renewals",
    tag = TAG,
    operation_id = "pipeline_renewals_list",
    security(("session_cookie" = [])),
    description = "Renewal settings saved for the workflow of the organization. Needs `settings.manage`.",
    responses(
        (status = 200, description = "Saved renewal settings", body = Data<Vec<RenewalSettingsResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_pipeline_renewals(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<RenewalSettingsResponse>>>, ApiError> {
    let settings = service::list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        settings.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/workflow/{pipeline_id}/renewal",
    tag = TAG,
    operation_id = "pipeline_renewal_get",
    security(("session_cookie" = [])),
    description = "Automatic renewal of the won deals of the pipeline. While never saved, returns the defaults (`configured: false`, disabled, 30 days, first open stage of the same pipeline). Needs `settings.manage`.",
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    responses(
        (status = 200, description = "Renewal settings of the pipeline", body = Data<RenewalSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_pipeline_renewal(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Data<RenewalSettingsResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::get(&ctx, &pipeline_id).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/workflow/{pipeline_id}/renewal",
    tag = TAG,
    operation_id = "pipeline_renewal_update",
    security(("session_cookie" = [])),
    description = "Saves the automatic renewal of the pipeline and publishes `pipeline.renewal_updated`. When enabled, the `renewals.scan` job (hourly) creates, for each won deal of the pipeline whose `contract_end` is at most `days_before` days ahead (organization timezone, until `contract_end`), one renewal deal in `target_stage_id` with `renewal_of_deal_id` pointing to the original, the same contact, company and currency, the owner of the original (`owner_mode = original`) or no owner (`none`), the title `title_prefix + title`, `contract_start` the day after the original `contract_end` and a copy of the recurring items; then publishes `deal.renewal_created`. Each deal is renewed only once. The renewal runs with the permissions of whoever saved the settings last. Fields left out keep their value. Needs `settings.manage`.",
    params(
        ("pipeline_id" = String, Path, description = "Pipeline id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the settings as read (ISO-8601); ignored while never saved"),
    ),
    request_body = RenewalSettingsRequest,
    responses(
        (status = 200, description = "Renewal settings saved", body = Data<RenewalSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The settings changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid days, target pipeline/stage (must be an active open stage of the target pipeline), prefix or owner mode (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_pipeline_renewal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(pipeline_id): Path<String>,
    JsonBody(request): JsonBody<RenewalSettingsRequest>,
) -> Result<Json<Data<RenewalSettingsResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::save(
        &ctx,
        &pipeline_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}
