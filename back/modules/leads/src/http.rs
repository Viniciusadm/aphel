use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    DisqualificationCount, DisqualificationReason, LifecycleReport, StageHistoryEntry, StageOption,
    StageReport, TransitionRule,
};
use crate::hooks::ConvertLeadCommand;
use crate::service;
use crate::service::{
    CreateDisqualificationReasonCommand, DisqualifyLeadCommand, LeadActionResult,
    LifecycleReportQuery, LifecycleSettingsView, SaveLifecycleSettingsCommand, TransitionRuleInput,
    UpdateDisqualificationReasonCommand,
};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::common::timestamp as instant;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "lifecycle";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_lifecycle_settings, update_lifecycle_settings))
        .routes(routes!(list_reasons, create_reason))
        .routes(routes!(reorder_reasons))
        .routes(routes!(update_reason, delete_reason))
        .routes(routes!(lifecycle_history))
        .routes(routes!(lifecycle_report))
        .routes(routes!(qualify_lead))
        .routes(routes!(disqualify_lead))
        .routes(routes!(convert_lead))
}

#[utoipa::path(
    get,
    path = "/lifecycle/settings",
    tag = TAG,
    operation_id = "lifecycle_settings_get",
    security(("session_cookie" = [])),
    description = "Automatic lifecycle transitions and the stage given to disqualified leads, with the lifecycle stages of the contact field. While never saved, returns the defaults (`configured: false`): `deal_created` → `opportunity` (from lead, qualified, former customer or no stage; also on reopening), `deal_won` → `customer` (from any stage but customer), `all_deals_lost` → `qualified` (only from opportunity, when the contact or company has lost deals and no open or won deal; also evaluated when a deal is deleted). A rule never applies when the record is outside `from_stages` (and `from_empty` for records without stage), so it never demotes a record by itself. Needs `lifecycle.manage`.",
    responses(
        (status = 200, description = "Lifecycle settings", body = Data<LifecycleSettingsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn get_lifecycle_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<LifecycleSettingsResponse>>, ApiError> {
    let view = service::settings(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/lifecycle/settings",
    tag = TAG,
    operation_id = "lifecycle_settings_update",
    security(("session_cookie" = [])),
    description = "Saves the lifecycle settings and publishes `lifecycle.settings_updated`. Each item of `rules` changes one trigger (`deal_created`, `deal_won`, `all_deals_lost`); omitted triggers and properties keep their value. `target_stage` must be an active option of the `lifecycle_stage` field of contacts and `from_stages` options of the field. `disqualified_stage` is the stage given to a disqualified lead (`null` clears the stage). The transitions run in the worker (`lifecycle.transitions` consumer) and update the contact and the company of the deal through their use cases (`contact.updated`/`company.updated` with `changes`). Needs `lifecycle.manage`.",
    params(("If-Match" = Option<String>, Header, description = "`updated_at` of the settings as read (ISO-8601); ignored while never saved")),
    request_body = LifecycleSettingsRequest,
    responses(
        (status = 200, description = "Lifecycle settings saved", body = Data<LifecycleSettingsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "The settings changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown trigger or stage (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_lifecycle_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    JsonBody(request): JsonBody<LifecycleSettingsRequest>,
) -> Result<Json<Data<LifecycleSettingsResponse>>, ApiError> {
    let view = service::save_settings(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    get,
    path = "/lead-disqualification-reasons",
    tag = TAG,
    operation_id = "lead_disqualification_reasons_list",
    security(("session_cookie" = [])),
    params(DisqualificationReasonListParams),
    responses(
        (status = 200, description = "Reasons to disqualify a lead in their order (active only unless `include_inactive=true`). Six reasons are created with the organization", body = Data<Vec<DisqualificationReasonResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `contacts.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_reasons(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<DisqualificationReasonListParams>,
) -> Result<Json<Data<Vec<DisqualificationReasonResponse>>>, ApiError> {
    let reasons = service::list_reasons(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.include_inactive.unwrap_or(false),
    )
    .await?;
    Ok(Json(Data::new(
        reasons.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/lead-disqualification-reasons",
    tag = TAG,
    operation_id = "lead_disqualification_reasons_create",
    security(("session_cookie" = [])),
    request_body = CreateDisqualificationReasonRequest,
    responses(
        (status = 201, description = "Reason created at the end of the list (`lead_disqualification_reason.created`); `key` defaults to one derived from the name", body = Data<DisqualificationReasonResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another reason uses the key (`disqualification_reason_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or key, or the limit of 50 reasons was reached (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateDisqualificationReasonRequest>,
) -> Result<(StatusCode, Json<Data<DisqualificationReasonResponse>>), ApiError> {
    let reason = service::create_reason(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(reason.into()))))
}

#[utoipa::path(
    patch,
    path = "/lead-disqualification-reasons/{reason_id}",
    tag = TAG,
    operation_id = "lead_disqualification_reasons_update",
    security(("session_cookie" = [])),
    params(
        ("reason_id" = String, Path, description = "Reason id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the reason as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateDisqualificationReasonRequest,
    responses(
        (status = 200, description = "Reason updated (`lead_disqualification_reason.updated`); the key never changes. Inactive reasons stay on past disqualifications but cannot be picked again", body = Data<DisqualificationReasonResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Reason not found (`disqualification_reason_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The reason changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(reason_id): Path<String>,
    JsonBody(request): JsonBody<UpdateDisqualificationReasonRequest>,
) -> Result<Json<Data<DisqualificationReasonResponse>>, ApiError> {
    let reason = service::update_reason(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &reason_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(reason.into())))
}

#[utoipa::path(
    delete,
    path = "/lead-disqualification-reasons/{reason_id}",
    tag = TAG,
    operation_id = "lead_disqualification_reasons_delete",
    security(("session_cookie" = [])),
    params(("reason_id" = String, Path, description = "Reason id")),
    responses(
        (status = 204, description = "Reason deleted (`lead_disqualification_reason.deleted`); past disqualifications keep the reason name"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Reason not found (`disqualification_reason_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(reason_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete_reason(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &reason_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/lead-disqualification-reasons/order",
    tag = TAG,
    operation_id = "lead_disqualification_reasons_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderDisqualificationReasonsRequest,
    responses(
        (status = 200, description = "Reasons in the new order (`lead_disqualification_reason.reordered`); `ids` lists every reason, active or not, exactly once", body = Data<Vec<DisqualificationReasonResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The list does not match the reasons of the organization (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_reasons(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderDisqualificationReasonsRequest>,
) -> Result<Json<Data<Vec<DisqualificationReasonResponse>>>, ApiError> {
    let reasons = service::reorder_reasons(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request.ids,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        reasons.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/lifecycle/history/{record_type}/{record_id}",
    tag = TAG,
    operation_id = "lifecycle_history",
    security(("session_cookie" = [])),
    description = "Lifecycle stage history of a contact or a company, newest first (at most 200 entries). Each change of `lifecycle_stage` (creation included) closes the open entry (`left_at`) and opens a new one in the same transaction as the record write. Needs read access to the record.",
    params(
        ("record_type" = String, Path, description = "`contact` or `company`"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Stage history", body = Data<Vec<LifecycleHistoryEntryResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `contacts.read`/`companies.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type, or record not found or not visible (`record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn lifecycle_history(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Vec<LifecycleHistoryEntryResponse>>>, ApiError> {
    let entries = service::history(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
    )
    .await?;
    Ok(Json(Data::new(
        entries.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/lifecycle/report",
    tag = TAG,
    operation_id = "lifecycle_report",
    security(("session_cookie" = [])),
    description = "Lifecycle report built from the stage history of the records visible to the user (read scope of `contacts.read` or `companies.read`). Stages follow the order of the active options of the field. For each stage: `current` records in it now (not archived), `entered` records that entered it in the period (UTC days, `from`–`to` inclusive, default the last 90 days, at most 731), `converted` of those that later reached any later stage (skipping counts), `conversion_rate` = converted / entered, and `average_days` spent in the stage by the entries of the period that already left it (`completed`). For contacts, also the disqualifications of the period by reason.",
    params(LifecycleReportParams),
    responses(
        (status = 200, description = "Lifecycle report", body = Data<LifecycleReportResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `contacts.read`/`companies.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record type or period (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn lifecycle_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<LifecycleReportParams>,
) -> Result<Json<Data<LifecycleReportResponse>>, ApiError> {
    let report = service::report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into(),
    )
    .await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    post,
    path = "/leads/{contact_id}/qualify",
    tag = TAG,
    operation_id = "leads_qualify",
    security(("session_cookie" = [])),
    description = "Moves a contact in `lead` to `qualified` and publishes `contact.updated` (with `changes`) and `lead.qualified`, in the same transaction as the stage history. Needs `contacts.update` on the contact.",
    params(("contact_id" = String, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Lead qualified", body = Data<LeadActionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `contacts.update` on the contact or the field is read only (`permission_denied`, `field_read_only`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The contact is not in `lead` (`contact_not_lead`)", body = ErrorEnvelope),
        (status = 422, description = "The stage `qualified` is not an active option (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn qualify_lead(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
) -> Result<Json<Data<LeadActionResponse>>, ApiError> {
    let result = service::qualify_lead(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    post,
    path = "/leads/{contact_id}/disqualify",
    tag = TAG,
    operation_id = "leads_disqualify",
    security(("session_cookie" = [])),
    description = "Disqualifies a contact in `lead` or `qualified` with an active reason and an optional note (kept in the disqualification and the audit, not in the event). The stage becomes the `disqualified_stage` of the lifecycle settings (none by default). Publishes `contact.updated` (when the stage changes) and `lead.disqualified`. Needs `contacts.update` on the contact.",
    params(("contact_id" = String, Path, description = "Contact id")),
    request_body = DisqualifyLeadRequest,
    responses(
        (status = 200, description = "Lead disqualified", body = Data<LeadActionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `contacts.update` on the contact or the field is read only (`permission_denied`, `field_read_only`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The contact is not in `lead` or `qualified` (`contact_not_lead`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or inactive reason, or note too long (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn disqualify_lead(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
    JsonBody(request): JsonBody<DisqualifyLeadRequest>,
) -> Result<Json<Data<LeadActionResponse>>, ApiError> {
    let result = service::disqualify_lead(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    post,
    path = "/leads/{contact_id}/convert",
    tag = TAG,
    operation_id = "leads_convert",
    security(("session_cookie" = [])),
    description = "Converts a contact in `lead` or `qualified`: creates a deal through the deals use case (permission `deals.create`; `deal.created`) in `pipeline_id` (and `stage_id`, default the first open stage), with the contact, its primary company and its source; the title defaults to the contact name. Then moves the contact to the target stage of the `deal_created` rule (`opportunity` by default) and publishes `contact.updated` and `lead.converted` with `deal_id`. Needs `contacts.update` on the contact.",
    params(("contact_id" = String, Path, description = "Contact id")),
    request_body = ConvertLeadRequest,
    responses(
        (status = 200, description = "Lead converted", body = Data<LeadActionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.create` or `contacts.update` on the contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact or pipeline not found (`contact_not_found`, `pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The contact is not in `lead` or `qualified` (`contact_not_lead`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid deal (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn convert_lead(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
    JsonBody(request): JsonBody<ConvertLeadRequest>,
) -> Result<Json<Data<LeadActionResponse>>, ApiError> {
    let result = service::convert_lead(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(result.into())))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleStageOptionResponse {
    #[schema(example = "qualified")]
    pub(crate) value: String,
    #[schema(example = "Qualified")]
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) position: i32,
    pub(crate) is_active: bool,
}

impl From<StageOption> for LifecycleStageOptionResponse {
    fn from(option: StageOption) -> Self {
        Self {
            value: option.value,
            label: option.label,
            color: option.color,
            position: option.position,
            is_active: option.is_active,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleTransitionRuleResponse {
    #[schema(example = "deal_won")]
    pub(crate) trigger: String,
    pub(crate) enabled: bool,
    #[schema(example = "customer")]
    pub(crate) target_stage: String,
    pub(crate) from_stages: Vec<String>,
    pub(crate) from_empty: bool,
}

impl From<TransitionRule> for LifecycleTransitionRuleResponse {
    fn from(rule: TransitionRule) -> Self {
        Self {
            trigger: rule.trigger.as_str().to_owned(),
            enabled: rule.enabled,
            target_stage: rule.target_stage,
            from_stages: rule.from_stages,
            from_empty: rule.from_empty,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleSettingsResponse {
    pub(crate) configured: bool,
    pub(crate) rules: Vec<LifecycleTransitionRuleResponse>,
    pub(crate) disqualified_stage: Option<String>,
    pub(crate) stages: Vec<LifecycleStageOptionResponse>,
    pub(crate) updated_at: Option<String>,
}

impl From<LifecycleSettingsView> for LifecycleSettingsResponse {
    fn from(view: LifecycleSettingsView) -> Self {
        Self {
            configured: view.settings.updated_at.is_some(),
            rules: view.settings.rules.into_iter().map(Into::into).collect(),
            disqualified_stage: view.settings.disqualified_stage,
            stages: view.stages.into_iter().map(Into::into).collect(),
            updated_at: view.settings.updated_at.map(instant),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LifecycleTransitionRuleRequest {
    #[schema(example = "all_deals_lost")]
    pub(crate) trigger: String,
    pub(crate) enabled: Option<bool>,
    pub(crate) target_stage: Option<String>,
    pub(crate) from_stages: Option<Vec<String>>,
    pub(crate) from_empty: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LifecycleSettingsRequest {
    #[serde(default)]
    pub(crate) rules: Vec<LifecycleTransitionRuleRequest>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) disqualified_stage: Option<Option<String>>,
}

impl LifecycleSettingsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveLifecycleSettingsCommand {
        SaveLifecycleSettingsCommand {
            rules: self
                .rules
                .into_iter()
                .map(|rule| TransitionRuleInput {
                    trigger: rule.trigger,
                    enabled: rule.enabled,
                    target_stage: rule.target_stage,
                    from_stages: rule.from_stages,
                    from_empty: rule.from_empty,
                })
                .collect(),
            disqualified_stage: self.disqualified_stage,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DisqualificationReasonResponse {
    pub(crate) id: String,
    #[schema(example = "no_budget")]
    pub(crate) key: String,
    #[schema(example = "No budget")]
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<DisqualificationReason> for DisqualificationReasonResponse {
    fn from(reason: DisqualificationReason) -> Self {
        Self {
            id: reason.id.to_string(),
            key: reason.key,
            name: reason.name,
            is_active: reason.is_active,
            position: reason.position,
            created_at: instant(reason.created_at),
            updated_at: instant(reason.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DisqualificationReasonListParams {
    pub(crate) include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDisqualificationReasonRequest {
    #[schema(example = "wrong_region")]
    pub(crate) key: Option<String>,
    #[schema(example = "Outside service area")]
    pub(crate) name: String,
}

impl CreateDisqualificationReasonRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateDisqualificationReasonCommand {
        CreateDisqualificationReasonCommand {
            key: self.key,
            name: self.name,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateDisqualificationReasonRequest {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateDisqualificationReasonRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateDisqualificationReasonCommand {
        UpdateDisqualificationReasonCommand {
            name: self.name,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReorderDisqualificationReasonsRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleHistoryActorResponse {
    #[schema(example = "user")]
    pub(crate) r#type: String,
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleHistoryEntryResponse {
    pub(crate) id: String,
    pub(crate) from_stage: Option<String>,
    pub(crate) to_stage: String,
    pub(crate) entered_at: String,
    pub(crate) left_at: Option<String>,
    pub(crate) days_in_stage: Option<f64>,
    pub(crate) actor: LifecycleHistoryActorResponse,
}

impl From<StageHistoryEntry> for LifecycleHistoryEntryResponse {
    fn from(entry: StageHistoryEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            days_in_stage: entry
                .left_at
                .map(|left_at| (left_at - entry.entered_at).num_seconds().max(0) as f64 / 86_400.0),
            from_stage: entry.from_stage,
            to_stage: entry.to_stage,
            entered_at: instant(entry.entered_at),
            left_at: entry.left_at.map(instant),
            actor: LifecycleHistoryActorResponse {
                r#type: entry.actor_type,
                id: entry.actor_id,
                name: entry.actor_name,
            },
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct LifecycleReportParams {
    #[param(example = "contact")]
    pub(crate) record_type: Option<String>,
    #[param(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-03-31")]
    pub(crate) to: Option<String>,
}

impl From<LifecycleReportParams> for LifecycleReportQuery {
    fn from(params: LifecycleReportParams) -> Self {
        Self {
            record_type: params.record_type,
            from: params.from,
            to: params.to,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleStageReportResponse {
    pub(crate) stage: String,
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) current: u64,
    pub(crate) entered: u64,
    pub(crate) next_stage: Option<String>,
    pub(crate) converted: u64,
    pub(crate) conversion_rate: Option<f64>,
    pub(crate) completed: u64,
    pub(crate) average_days: Option<f64>,
}

impl From<StageReport> for LifecycleStageReportResponse {
    fn from(stage: StageReport) -> Self {
        Self {
            stage: stage.stage,
            label: stage.label,
            color: stage.color,
            is_active: stage.is_active,
            current: stage.current,
            entered: stage.entered,
            next_stage: stage.next_stage,
            converted: stage.converted,
            conversion_rate: stage.conversion_rate,
            completed: stage.completed,
            average_days: stage.average_days,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleDisqualificationCountResponse {
    pub(crate) reason_id: Option<String>,
    pub(crate) reason_name: String,
    pub(crate) count: u64,
}

impl From<DisqualificationCount> for LifecycleDisqualificationCountResponse {
    fn from(count: DisqualificationCount) -> Self {
        Self {
            reason_id: count.reason_id.map(|id| id.to_string()),
            reason_name: count.reason_name,
            count: count.count,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LifecycleReportResponse {
    #[schema(example = "contact")]
    pub(crate) record_type: String,
    #[schema(example = "2026-01-01")]
    pub(crate) from: String,
    #[schema(example = "2026-03-31")]
    pub(crate) to: String,
    pub(crate) stages: Vec<LifecycleStageReportResponse>,
    pub(crate) disqualifications: Vec<LifecycleDisqualificationCountResponse>,
}

impl From<LifecycleReport> for LifecycleReportResponse {
    fn from(report: LifecycleReport) -> Self {
        Self {
            record_type: report.record_type.as_str().to_owned(),
            from: report.from.to_string(),
            to: report.to.to_string(),
            stages: report.stages.into_iter().map(Into::into).collect(),
            disqualifications: report
                .disqualifications
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DisqualifyLeadRequest {
    pub(crate) reason_id: String,
    #[schema(example = "Asked not to be contacted this semester")]
    pub(crate) note: Option<String>,
}

impl DisqualifyLeadRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> DisqualifyLeadCommand {
        DisqualifyLeadCommand {
            reason_id: self.reason_id,
            note: self.note,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConvertLeadRequest {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: Option<String>,
    #[schema(example = "Onboarding - Acme")]
    pub(crate) title: Option<String>,
    #[schema(example = "15000.00")]
    pub(crate) amount: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
}

impl ConvertLeadRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ConvertLeadCommand {
        ConvertLeadCommand {
            pipeline_id: self.pipeline_id,
            stage_id: self.stage_id,
            title: self.title,
            amount: self.amount,
            currency: self.currency,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadActionResponse {
    pub(crate) contact_id: String,
    pub(crate) lifecycle_stage: Option<String>,
    pub(crate) deal_id: Option<String>,
}

impl From<LeadActionResult> for LeadActionResponse {
    fn from(result: LeadActionResult) -> Self {
        Self {
            contact_id: result.contact_id.to_string(),
            lifecycle_stage: result.lifecycle_stage,
            deal_id: result.deal_id.map(|id| id.to_string()),
        }
    }
}
