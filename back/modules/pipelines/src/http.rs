use std::collections::HashMap;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    CreateLostReasonCommand, CreatePipelineCommand, CreateStageCommand, LostReason, NewStageInput,
    Pipeline, PipelineRemovalCommand, PipelineStage, PipelineView, ReorderCommand, StageKind,
    StageRemovalCommand, StageView, UpdateLostReasonCommand, UpdatePipelineCommand,
    UpdateStageCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, double_option};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "pipelines";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_pipelines, create_pipeline))
        .routes(routes!(reorder_pipelines))
        .routes(routes!(get_pipeline, update_pipeline, delete_pipeline))
        .routes(routes!(set_default_pipeline))
        .routes(routes!(activate_pipeline))
        .routes(routes!(deactivate_pipeline))
        .routes(routes!(create_stage))
        .routes(routes!(reorder_stages))
        .routes(routes!(list_stages))
        .routes(routes!(get_stage, update_stage, delete_stage))
        .routes(routes!(activate_stage))
        .routes(routes!(deactivate_stage))
        .routes(routes!(list_lost_reasons, create_lost_reason))
        .routes(routes!(reorder_lost_reasons))
        .routes(routes!(get_lost_reason, update_lost_reason))
}

fn page<T, U: From<T>>(items: Vec<T>, pagination: Pagination) -> Page<U> {
    let total = items.len() as u64;
    let items = items
        .into_iter()
        .skip(usize::try_from(pagination.offset()).unwrap_or(usize::MAX))
        .take(usize::try_from(pagination.limit()).unwrap_or(usize::MAX))
        .map(Into::into)
        .collect();
    Page::new(items, total, pagination)
}

#[utoipa::path(
    get,
    path = "/pipelines",
    tag = TAG,
    operation_id = "pipelines_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Pipelines of the organization in order with their stages (active and inactive) and the number of deals in each stage. Pipelines restricted to teams (`team_ids`) only show up for members of those teams (sub-teams included) and for users with `settings.manage`", body = Page<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_pipelines(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<PipelineResponse>>, ApiError> {
    let pipelines = service::list_pipelines(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(page(pipelines, pagination)))
}

#[utoipa::path(
    get,
    path = "/pipelines/{pipeline_id}",
    tag = TAG,
    operation_id = "pipelines_get",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    responses(
        (status = 200, description = "Pipeline with its stages", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found or restricted to other teams (`pipeline_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline =
        service::get_pipeline(&Ctx::from_state(&state, &tenant.scope).await?, &pipeline_id).await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    patch,
    path = "/pipelines/{pipeline_id}",
    tag = TAG,
    operation_id = "pipelines_update",
    security(("session_cookie" = [])),
    params(
        ("pipeline_id" = String, Path, description = "Pipeline id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the pipeline as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdatePipelineRequest,
    responses(
        (status = 200, description = "Pipeline renamed or restricted to teams (`team_ids`, empty for everyone); publishes `pipeline.updated`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The pipeline changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or teams (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(pipeline_id): Path<String>,
    JsonBody(request): JsonBody<UpdatePipelineRequest>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline = service::update_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    post,
    path = "/pipelines",
    tag = TAG,
    operation_id = "pipelines_create",
    security(("session_cookie" = [])),
    request_body = CreatePipelineRequest,
    responses(
        (status = 201, description = "Pipeline created at the end of the list with the given stages (or the default ones) and teams; the first pipeline of an organization becomes the default. Publishes `pipeline.created`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "The stages lack an open, won or lost stage (`closing_stages_required`), or the organization reached the limit of pipelines (`pipeline_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, teams or stages (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreatePipelineRequest>,
) -> Result<(StatusCode, Json<Data<PipelineResponse>>), ApiError> {
    let pipeline = service::create_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(pipeline.into()))))
}

#[utoipa::path(
    put,
    path = "/pipelines/order",
    tag = TAG,
    operation_id = "pipelines_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Pipelines reordered; publishes `pipeline.reordered`", body = Data<Vec<PipelineResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The ids must list every pipeline exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_pipelines(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderRequest>,
) -> Result<Json<Data<Vec<PipelineResponse>>>, ApiError> {
    let pipelines = service::reorder_pipelines(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        pipelines.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    delete,
    path = "/pipelines/{pipeline_id}",
    tag = TAG,
    operation_id = "pipelines_delete",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    request_body = PipelineRemovalRequest,
    responses(
        (status = 204, description = "Pipeline deleted with its stages; its deals (including deleted ones) move to `target_pipeline_id`, each stage to `stage_mapping[stage_id]` or to the first active stage of the same kind. Publishes `pipeline.deleted` and `deal.pipeline_changed`/`deal.stage_changed` for each moved deal"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The default pipeline cannot be deleted (`default_pipeline_required`), or the pipeline has deals and no target was given (`pipeline_has_deals`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid target pipeline or stage mapping (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn delete_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
    JsonBody(request): JsonBody<PipelineRemovalRequest>,
) -> Result<StatusCode, ApiError> {
    service::delete_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/pipelines/{pipeline_id}/default",
    tag = TAG,
    operation_id = "pipelines_set_default",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    responses(
        (status = 200, description = "Pipeline is now the default one (Kanban and new deals without a pipeline); publishes `pipeline.default_changed`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "An inactive pipeline cannot be the default one (`default_pipeline_required`)", body = ErrorEnvelope),
    )
)]
async fn set_default_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline = service::set_default_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    post,
    path = "/pipelines/{pipeline_id}/activate",
    tag = TAG,
    operation_id = "pipelines_activate",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    responses(
        (status = 200, description = "Pipeline active again; publishes `pipeline.activated`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
    )
)]
async fn activate_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline = service::activate_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    post,
    path = "/pipelines/{pipeline_id}/deactivate",
    tag = TAG,
    operation_id = "pipelines_deactivate",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    responses(
        (status = 200, description = "Pipeline deactivated: its deals stay where they are, but no deal can be created in or moved into it. Publishes `pipeline.deactivated`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The default pipeline cannot be deactivated (`default_pipeline_required`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline = service::deactivate_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    post,
    path = "/pipelines/{pipeline_id}/stages",
    tag = TAG,
    operation_id = "pipelines_create_stage",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    request_body = CreateStageRequest,
    responses(
        (status = 201, description = "Stage created; open stages go after the last open stage, won and lost stages at the end. `entry_required_fields`/`exit_required_fields` list deal fields that must be filled to enter or leave it. Publishes `pipeline_stage.created`", body = Data<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, kind, probability or required fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
    JsonBody(request): JsonBody<CreateStageRequest>,
) -> Result<(StatusCode, Json<Data<PipelineStageResponse>>), ApiError> {
    let stage = service::create_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(stage.into()))))
}

#[utoipa::path(
    put,
    path = "/pipelines/{pipeline_id}/stages/order",
    tag = TAG,
    operation_id = "pipelines_reorder_stages",
    security(("session_cookie" = [])),
    params(("pipeline_id" = String, Path, description = "Pipeline id")),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Stages reordered; publishes `pipeline_stage.reordered`", body = Data<PipelineResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The ids must list every stage of the pipeline exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_stages(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(pipeline_id): Path<String>,
    JsonBody(request): JsonBody<ReorderRequest>,
) -> Result<Json<Data<PipelineResponse>>, ApiError> {
    let pipeline = service::reorder_stages(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &pipeline_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(pipeline.into())))
}

#[utoipa::path(
    get,
    path = "/pipeline-stages",
    tag = TAG,
    operation_id = "pipeline_stages_list",
    security(("session_cookie" = [])),
    params(StageListParams, PaginationQuery),
    responses(
        (status = 200, description = "Stages of every pipeline (or of `pipeline_id`) in order; inactive ones only with `include_inactive=true`, filtered by name with `search`", body = Page<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid parameters (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_stages(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<StageListParams>, QueryRejection>,
) -> Result<Json<Page<PipelineStageResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let stages = service::list_stages(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.pipeline_id.as_deref(),
        params.include_inactive.unwrap_or(false),
        params.search.as_deref(),
    )
    .await?;
    Ok(Json(page(stages, pagination)))
}

#[utoipa::path(
    get,
    path = "/pipeline-stages/{stage_id}",
    tag = TAG,
    operation_id = "pipeline_stages_get",
    security(("session_cookie" = [])),
    params(("stage_id" = String, Path, description = "Stage id")),
    responses(
        (status = 200, description = "Stage", body = Data<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(stage_id): Path<String>,
) -> Result<Json<Data<PipelineStageResponse>>, ApiError> {
    let stage =
        service::get_stage(&Ctx::from_state(&state, &tenant.scope).await?, &stage_id).await?;
    Ok(Json(Data::new(stage.into())))
}

#[utoipa::path(
    patch,
    path = "/pipeline-stages/{stage_id}",
    tag = TAG,
    operation_id = "pipeline_stages_update",
    security(("session_cookie" = [])),
    params(
        ("stage_id" = String, Path, description = "Stage id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the stage as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateStageRequest,
    responses(
        (status = 200, description = "Stage renamed or changed, including the fields required to enter (`entry_required_fields`) or leave it (`exit_required_fields`): system fields `amount`, `currency`, `probability`, `expected_close_date`, `contact_id`, `company_id`, `owner_id`, `source` or active custom fields. `rotting_days` (1 to 3650, `null` clears it, open stages only) marks deals that stay longer in the stage as rotting. Publishes `pipeline_stage.updated`", body = Data<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The kind of a stage with deals cannot change (`stage_has_deals`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`), or the stage changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, kind, probability or required fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(stage_id): Path<String>,
    JsonBody(request): JsonBody<UpdateStageRequest>,
) -> Result<Json<Data<PipelineStageResponse>>, ApiError> {
    let stage = service::update_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &stage_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(stage.into())))
}

#[utoipa::path(
    post,
    path = "/pipeline-stages/{stage_id}/activate",
    tag = TAG,
    operation_id = "pipeline_stages_activate",
    security(("session_cookie" = [])),
    params(("stage_id" = String, Path, description = "Stage id")),
    responses(
        (status = 200, description = "Stage active again; publishes `pipeline_stage.updated`", body = Data<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
    )
)]
async fn activate_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(stage_id): Path<String>,
) -> Result<Json<Data<PipelineStageResponse>>, ApiError> {
    let stage = service::activate_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &stage_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(stage.into())))
}

#[utoipa::path(
    post,
    path = "/pipeline-stages/{stage_id}/deactivate",
    tag = TAG,
    operation_id = "pipeline_stages_deactivate",
    security(("session_cookie" = [])),
    params(("stage_id" = String, Path, description = "Stage id")),
    request_body = StageRemovalRequest,
    responses(
        (status = 200, description = "Stage deactivated; its deals move to `target_stage_id` (another active stage of the same kind). Publishes `pipeline_stage.deactivated` and `deal.stage_changed` for each moved deal", body = Data<PipelineStageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The stage has deals and no target was given (`stage_has_deals`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid target stage (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(stage_id): Path<String>,
    JsonBody(request): JsonBody<StageRemovalRequest>,
) -> Result<Json<Data<PipelineStageResponse>>, ApiError> {
    let stage = service::deactivate_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &stage_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(stage.into())))
}

#[utoipa::path(
    delete,
    path = "/pipeline-stages/{stage_id}",
    tag = TAG,
    operation_id = "pipeline_stages_delete",
    security(("session_cookie" = [])),
    params(("stage_id" = String, Path, description = "Stage id"), StageRemovalRequest),
    responses(
        (status = 204, description = "Stage deleted; its deals (including deleted ones) move to `target_stage_id`. Publishes `pipeline_stage.deleted`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Stage not found (`pipeline_stage_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The stage has deals and no target was given (`stage_has_deals`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid target stage (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn delete_stage(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(stage_id): Path<String>,
    query: Result<Query<StageRemovalRequest>, QueryRejection>,
) -> Result<StatusCode, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    service::delete_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &stage_id,
        params.into_command(client.0.ip),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/lost-reasons",
    tag = TAG,
    operation_id = "lost_reasons_list",
    security(("session_cookie" = [])),
    params(LostReasonParams, PaginationQuery),
    responses(
        (status = 200, description = "Lost reasons in order; inactive ones only with `include_inactive=true`", body = Page<LostReasonResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid parameters (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_lost_reasons(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    query: Result<Query<LostReasonParams>, QueryRejection>,
) -> Result<Json<Page<LostReasonResponse>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let reasons = service::list_lost_reasons(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.include_inactive.unwrap_or(false),
        params.search.as_deref(),
    )
    .await?;
    Ok(Json(page(reasons, pagination)))
}

#[utoipa::path(
    post,
    path = "/lost-reasons",
    tag = TAG,
    operation_id = "lost_reasons_create",
    security(("session_cookie" = [])),
    request_body = CreateLostReasonRequest,
    responses(
        (status = 201, description = "Lost reason created at the end; publishes `lost_reason.created`", body = Data<LostReasonResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_lost_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateLostReasonRequest>,
) -> Result<(StatusCode, Json<Data<LostReasonResponse>>), ApiError> {
    let reason = service::create_lost_reason(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(reason.into()))))
}

#[utoipa::path(
    put,
    path = "/lost-reasons/order",
    tag = TAG,
    operation_id = "lost_reasons_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Lost reasons reordered; publishes `lost_reason.reordered`", body = Data<Vec<LostReasonResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The ids must list every lost reason exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_lost_reasons(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderRequest>,
) -> Result<Json<Data<Vec<LostReasonResponse>>>, ApiError> {
    let reasons = service::reorder_lost_reasons(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        reasons.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/lost-reasons/{reason_id}",
    tag = TAG,
    operation_id = "lost_reasons_get",
    security(("session_cookie" = [])),
    params(("reason_id" = String, Path, description = "Lost reason id")),
    responses(
        (status = 200, description = "Lost reason", body = Data<LostReasonResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Lost reason not found (`lost_reason_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_lost_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(reason_id): Path<String>,
) -> Result<Json<Data<LostReasonResponse>>, ApiError> {
    let reason =
        service::get_lost_reason(&Ctx::from_state(&state, &tenant.scope).await?, &reason_id)
            .await?;
    Ok(Json(Data::new(reason.into())))
}

#[utoipa::path(
    patch,
    path = "/lost-reasons/{reason_id}",
    tag = TAG,
    operation_id = "lost_reasons_update",
    security(("session_cookie" = [])),
    params(
        ("reason_id" = String, Path, description = "Lost reason id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the lost reason as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateLostReasonRequest,
    responses(
        (status = 200, description = "Lost reason renamed, activated or deactivated (they are never deleted); publishes `lost_reason.updated`", body = Data<LostReasonResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Lost reason not found (`lost_reason_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "At least one lost reason must stay active (`last_lost_reason`), or the lost reason changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_lost_reason(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(reason_id): Path<String>,
    JsonBody(request): JsonBody<UpdateLostReasonRequest>,
) -> Result<Json<Data<LostReasonResponse>>, ApiError> {
    let reason = service::update_lost_reason(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &reason_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(reason.into())))
}

fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = StageKind)]
pub enum StageKindSchema {
    Open,
    Won,
    Lost,
}

impl From<StageKind> for StageKindSchema {
    fn from(kind: StageKind) -> Self {
        match kind {
            StageKind::Open => Self::Open,
            StageKind::Won => Self::Won,
            StageKind::Lost => Self::Lost,
        }
    }
}

impl From<StageKindSchema> for StageKind {
    fn from(kind: StageKindSchema) -> Self {
        match kind {
            StageKindSchema::Open => Self::Open,
            StageKindSchema::Won => Self::Won,
            StageKindSchema::Lost => Self::Lost,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PipelineStageResponse {
    pub(crate) id: String,
    pub(crate) pipeline_id: String,
    #[schema(example = "Proposal")]
    pub(crate) name: String,
    pub(crate) position: i32,
    #[schema(example = 50)]
    pub(crate) probability: u8,
    #[schema(example = 7)]
    pub(crate) rotting_days: Option<u16>,
    pub(crate) kind: StageKindSchema,
    pub(crate) is_active: bool,
    #[schema(example = json!(["amount", "expected_close_date"]))]
    pub(crate) entry_required_fields: Vec<String>,
    pub(crate) exit_required_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) deal_count: Option<u64>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<PipelineStage> for PipelineStageResponse {
    fn from(stage: PipelineStage) -> Self {
        Self {
            id: stage.id.to_string(),
            pipeline_id: stage.pipeline_id.to_string(),
            name: stage.name,
            position: stage.position,
            probability: stage.probability,
            rotting_days: stage.rotting_days,
            kind: stage.kind.into(),
            is_active: stage.is_active,
            entry_required_fields: stage.entry_required_fields,
            exit_required_fields: stage.exit_required_fields,
            deal_count: None,
            created_at: instant(stage.created_at),
            updated_at: instant(stage.updated_at),
        }
    }
}

impl From<StageView> for PipelineStageResponse {
    fn from(view: StageView) -> Self {
        Self {
            deal_count: Some(view.deal_count),
            ..view.stage.into()
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PipelineSummaryResponse {
    pub(crate) id: String,
    #[schema(example = "Sales pipeline")]
    pub(crate) name: String,
    pub(crate) is_default: bool,
    pub(crate) is_active: bool,
}

impl From<Pipeline> for PipelineSummaryResponse {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            is_default: pipeline.is_default,
            is_active: pipeline.is_active,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PipelineResponse {
    pub(crate) id: String,
    #[schema(example = "Sales pipeline")]
    pub(crate) name: String,
    pub(crate) is_default: bool,
    pub(crate) position: i32,
    pub(crate) is_active: bool,
    pub(crate) team_ids: Vec<String>,
    pub(crate) stages: Vec<PipelineStageResponse>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<PipelineView> for PipelineResponse {
    fn from(view: PipelineView) -> Self {
        let pipeline = view.pipeline;
        Self {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            is_default: pipeline.is_default,
            position: pipeline.position,
            is_active: pipeline.is_active,
            team_ids: pipeline.team_ids.iter().map(ToString::to_string).collect(),
            stages: view.stages.into_iter().map(Into::into).collect(),
            created_at: instant(pipeline.created_at),
            updated_at: instant(pipeline.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LostReasonResponse {
    pub(crate) id: String,
    #[schema(example = "Price")]
    pub(crate) name: String,
    pub(crate) position: i32,
    pub(crate) is_active: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<LostReason> for LostReasonResponse {
    fn from(reason: LostReason) -> Self {
        Self {
            id: reason.id.to_string(),
            name: reason.name,
            position: reason.position,
            is_active: reason.is_active,
            created_at: instant(reason.created_at),
            updated_at: instant(reason.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdatePipelineRequest {
    pub(crate) name: Option<String>,
    pub(crate) team_ids: Option<Vec<String>>,
}

impl UpdatePipelineRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdatePipelineCommand {
        UpdatePipelineCommand {
            name: self.name,
            team_ids: self.team_ids,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct NewStageRequest {
    #[schema(example = "Visit")]
    pub(crate) name: String,
    pub(crate) probability: Option<i64>,
    pub(crate) kind: Option<StageKindSchema>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreatePipelineRequest {
    #[schema(example = "Rentals")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) team_ids: Vec<String>,
    #[serde(default)]
    pub(crate) stages: Vec<NewStageRequest>,
}

impl CreatePipelineRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreatePipelineCommand {
        CreatePipelineCommand {
            name: self.name,
            team_ids: self.team_ids,
            stages: self
                .stages
                .into_iter()
                .map(|stage| NewStageInput {
                    name: stage.name,
                    probability: stage.probability,
                    kind: stage.kind.map(Into::into),
                })
                .collect(),
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PipelineRemovalRequest {
    pub(crate) target_pipeline_id: Option<String>,
    #[serde(default)]
    pub(crate) stage_mapping: HashMap<String, String>,
}

impl PipelineRemovalRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> PipelineRemovalCommand {
        PipelineRemovalCommand {
            target_pipeline_id: self.target_pipeline_id,
            stage_mapping: self.stage_mapping,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateStageRequest {
    #[schema(example = "Demo")]
    pub(crate) name: String,
    #[schema(example = 40)]
    pub(crate) probability: Option<i64>,
    #[schema(example = 7)]
    pub(crate) rotting_days: Option<i64>,
    pub(crate) kind: Option<StageKindSchema>,
    #[serde(default)]
    pub(crate) entry_required_fields: Vec<String>,
    #[serde(default)]
    pub(crate) exit_required_fields: Vec<String>,
}

impl CreateStageRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateStageCommand {
        CreateStageCommand {
            name: self.name,
            probability: self.probability,
            rotting_days: self.rotting_days,
            kind: self.kind.map(Into::into),
            entry_required_fields: self.entry_required_fields,
            exit_required_fields: self.exit_required_fields,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateStageRequest {
    pub(crate) name: Option<String>,
    pub(crate) probability: Option<i64>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 7)]
    pub(crate) rotting_days: Option<Option<i64>>,
    pub(crate) kind: Option<StageKindSchema>,
    #[schema(example = json!(["amount", "expected_close_date"]))]
    pub(crate) entry_required_fields: Option<Vec<String>>,
    pub(crate) exit_required_fields: Option<Vec<String>>,
}

impl UpdateStageRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateStageCommand {
        UpdateStageCommand {
            name: self.name,
            probability: self.probability,
            rotting_days: self.rotting_days,
            kind: self.kind.map(Into::into),
            entry_required_fields: self.entry_required_fields,
            exit_required_fields: self.exit_required_fields,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct StageRemovalRequest {
    pub(crate) target_stage_id: Option<String>,
}

impl StageRemovalRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> StageRemovalCommand {
        StageRemovalCommand {
            target_stage_id: self.target_stage_id,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ReorderRequest {
    pub(crate) ids: Vec<String>,
}

impl ReorderRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ReorderCommand {
        ReorderCommand { ids: self.ids, ip }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateLostReasonRequest {
    #[schema(example = "Delivery time")]
    pub(crate) name: String,
}

impl CreateLostReasonRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateLostReasonCommand {
        CreateLostReasonCommand {
            name: self.name,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateLostReasonRequest {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateLostReasonRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateLostReasonCommand {
        UpdateLostReasonCommand {
            name: self.name,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct LostReasonParams {
    pub(crate) include_inactive: Option<bool>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct StageListParams {
    pub(crate) pipeline_id: Option<String>,
    pub(crate) include_inactive: Option<bool>,
    pub(crate) search: Option<String>,
}
