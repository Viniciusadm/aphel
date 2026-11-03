use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::guidance::domain::{
    ChecklistItem, ChecklistItemInput, RecordChecklist, RecordChecklistItem,
    SaveStageGuidanceCommand, StageGuidance, ToggleChecklistItemCommand,
};
use crate::guidance::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, timestamp};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "workflow";
pub(crate) const RECORDS_TAG: &str = "records";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_stage_guidance, update_stage_guidance))
        .routes(routes!(get_record_checklist))
        .routes(routes!(toggle_record_checklist_item))
}

#[utoipa::path(
    get,
    path = "/pipeline-stages/{stage_id}/guidance",
    tag = TAG,
    operation_id = "workflow_stage_guidance_get",
    security(("session_cookie" = [])),
    description = "Guidance (markdown) and checklist of a pipeline stage. Readable by anyone who reads records.",
    params(("stage_id" = String, Path, description = "Pipeline stage id")),
    responses(
        (status = 200, description = "Guidance and checklist of the stage (empty while never configured)", body = Data<StageGuidanceResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `records.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_stage_guidance(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(stage_id): Path<String>,
) -> Result<Json<Data<StageGuidanceResponse>>, ApiError> {
    let guidance =
        service::get_stage_guidance(&Ctx::from_state(&state, &tenant.scope).await?, &stage_id)
            .await?;
    Ok(Json(Data::new(guidance.into())))
}

#[utoipa::path(
    put,
    path = "/pipeline-stages/{stage_id}/guidance",
    tag = TAG,
    operation_id = "workflow_stage_guidance_update",
    security(("session_cookie" = [])),
    description = "Saves the guidance and the checklist of a stage and publishes `pipeline.stage_guidance_updated`. `items` replaces the whole checklist in the given order: items are matched by `id` (or `key`), new ones get a `key` from the label, and items left out are removed with their progress. `required` items must be done before a record **advances** from the stage (moves to a stage of greater position in the same pipeline, or is won); moving back, to another pipeline or losing does not check the checklist. Needs `settings.manage`.",
    params(
        ("stage_id" = String, Path, description = "Pipeline stage id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the guidance as read (ISO-8601); ignored while never saved"),
    ),
    request_body = StageGuidanceRequest,
    responses(
        (status = 200, description = "Guidance saved", body = Data<StageGuidanceResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The guidance changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Guidance too long, too many items, invalid or duplicated keys, invalid labels (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_stage_guidance(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(stage_id): Path<String>,
    JsonBody(request): JsonBody<StageGuidanceRequest>,
) -> Result<Json<Data<StageGuidanceResponse>>, ApiError> {
    let guidance = service::save_stage_guidance(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &stage_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(guidance.into())))
}

#[utoipa::path(
    get,
    path = "/workflow/records/{record_type}/{record_id}/checklist",
    tag = RECORDS_TAG,
    operation_id = "record_checklist_get",
    security(("session_cookie" = [])),
    description = "Guidance and checklist of the current stage of the record with the progress of the record (who checked each item and when). Needs read access to the record.",
    params(("record_type" = String, Path, description = "Staged entity key"), ("record_id" = String, Path, description = "Record id")),
    responses(
        (status = 200, description = "Checklist of the current stage", body = Data<RecordChecklistResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `records.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not visible (`record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_record_checklist(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<RecordChecklistResponse>>, ApiError> {
    let checklist = service::record_checklist(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
    )
    .await?;
    Ok(Json(Data::new(checklist.into())))
}

#[utoipa::path(
    put,
    path = "/workflow/records/{record_type}/{record_id}/checklist/{item_id}",
    tag = RECORDS_TAG,
    operation_id = "record_checklist_item_update",
    security(("session_cookie" = [])),
    description = "Checks (`done: true`) or unchecks an item of a stage checklist of the record's pipeline and publishes `record.checklist_item_checked` or `record.checklist_item_unchecked`. Needs permission to update the record.",
    params(
        ("record_type" = String, Path, description = "Staged entity key"),
        ("record_id" = String, Path, description = "Record id"),
        ("item_id" = String, Path, description = "Checklist item id"),
    ),
    request_body = ToggleChecklistItemRequest,
    responses(
        (status = 200, description = "Checklist of the current stage after the change", body = Data<RecordChecklistResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `records.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not editable (`record_not_found`) or item not in the record's pipeline (`checklist_item_not_found`)", body = ErrorEnvelope),
    )
)]
async fn toggle_record_checklist_item(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, item_id)): Path<(String, String, String)>,
    JsonBody(request): JsonBody<ToggleChecklistItemRequest>,
) -> Result<Json<Data<RecordChecklistResponse>>, ApiError> {
    let checklist = service::toggle_checklist_item(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &record_type,
        &record_id,
        &item_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(checklist.into())))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ChecklistItemResponse {
    pub(crate) id: String,
    #[schema(example = "send_proposal")]
    pub(crate) key: String,
    #[schema(example = "Send the proposal")]
    pub(crate) label: String,
    pub(crate) required: bool,
    pub(crate) position: i32,
}

impl From<ChecklistItem> for ChecklistItemResponse {
    fn from(item: ChecklistItem) -> Self {
        Self {
            id: item.id.to_string(),
            key: item.key,
            label: item.label,
            required: item.is_required,
            position: item.position,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct StageGuidanceResponse {
    pub(crate) stage_id: String,
    pub(crate) pipeline_id: String,
    pub(crate) stage_name: String,
    #[schema(example = "Confirm the budget and the decision maker before sending the proposal.")]
    pub(crate) guidance: String,
    pub(crate) items: Vec<ChecklistItemResponse>,
    pub(crate) updated_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: Option<String>,
}

impl From<StageGuidance> for StageGuidanceResponse {
    fn from(guidance: StageGuidance) -> Self {
        Self {
            stage_id: guidance.stage_id.to_string(),
            pipeline_id: guidance.pipeline_id.to_string(),
            stage_name: guidance.stage_name,
            guidance: guidance.guidance,
            items: guidance.items.into_iter().map(Into::into).collect(),
            updated_by: guidance.updated_by.map(|id| id.into_inner()),
            updated_at: guidance.updated_at.map(timestamp),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ChecklistItemRequest {
    pub(crate) id: Option<String>,
    #[schema(example = "send_proposal")]
    pub(crate) key: Option<String>,
    #[schema(example = "Send the proposal")]
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) required: bool,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct StageGuidanceRequest {
    pub(crate) guidance: Option<String>,
    pub(crate) items: Option<Vec<ChecklistItemRequest>>,
}

impl StageGuidanceRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveStageGuidanceCommand {
        SaveStageGuidanceCommand {
            guidance: self.guidance,
            items: self.items.map(|items| {
                items
                    .into_iter()
                    .map(|item| ChecklistItemInput {
                        id: item.id,
                        key: item.key,
                        label: item.label,
                        required: item.required,
                    })
                    .collect()
            }),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ChecklistUserResponse {
    pub(crate) id: String,
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordChecklistItemResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) required: bool,
    pub(crate) position: i32,
    pub(crate) done: bool,
    pub(crate) done_by: Option<ChecklistUserResponse>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) done_at: Option<String>,
}

impl From<RecordChecklistItem> for RecordChecklistItemResponse {
    fn from(entry: RecordChecklistItem) -> Self {
        let done = entry.progress.is_some();
        let (done_by, done_at) = match entry.progress {
            Some(progress) => (
                progress.done_by.map(|id| ChecklistUserResponse {
                    id: id.into_inner(),
                    display_name: progress.done_by_name,
                }),
                Some(timestamp(progress.done_at)),
            ),
            None => (None, None),
        };
        Self {
            id: entry.item.id.to_string(),
            key: entry.item.key,
            label: entry.item.label,
            required: entry.item.is_required,
            position: entry.item.position,
            done,
            done_by,
            done_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RecordChecklistResponse {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) stage_id: String,
    pub(crate) stage_name: String,
    pub(crate) guidance: String,
    pub(crate) items: Vec<RecordChecklistItemResponse>,
    pub(crate) required_pending: usize,
}

impl From<RecordChecklist> for RecordChecklistResponse {
    fn from(checklist: RecordChecklist) -> Self {
        let required_pending = checklist
            .items
            .iter()
            .filter(|entry| entry.item.is_required && entry.progress.is_none())
            .count();
        Self {
            record_type: checklist.record_type.to_string(),
            record_id: checklist.record_id.to_string(),
            stage_id: checklist.guidance.stage_id.to_string(),
            stage_name: checklist.guidance.stage_name,
            guidance: checklist.guidance.guidance,
            items: checklist.items.into_iter().map(Into::into).collect(),
            required_pending,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ToggleChecklistItemRequest {
    pub(crate) done: bool,
}

impl ToggleChecklistItemRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ToggleChecklistItemCommand {
        ToggleChecklistItemCommand {
            done: self.done,
            ip,
        }
    }
}
