use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;
use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, Uri};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::domain::{
    Board, BoardColumn, BoardQuery, DEFAULT_BOARD_PER_COLUMN, DealInput, DealStatus, DealView,
    LoseDealCommand, MoveDealCommand, MoveDealPipelineCommand, NamedRef, ReopenDealCommand, date,
    instant,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, double_option};
use aphel_kernel::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, IncludeQuery, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::field_permissions::{record_redactor, redacted, redacted_items};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{
    PendingCommit, RecordCtx, RecordTypeHandler, optional_record, values_request,
};
use pipelines::http::{PipelineStageResponse, PipelineSummaryResponse, StageKindSchema};
use pipelines::rotting::RottingState;

pub(crate) const TAG: &str = "deals";

const DEAL_RELATIONSHIPS: [&str; 5] = ["stage", "contact", "company", "owner", "lost_reason"];

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_deals, create_deal))
        .routes(routes!(deal_board))
        .routes(routes!(get_deal, update_deal, delete_deal))
        .routes(routes!(move_deal))
        .routes(routes!(move_deal_pipeline))
        .routes(routes!(win_deal))
        .routes(routes!(lose_deal))
        .routes(routes!(reopen_deal))
        .routes(routes!(archive_deal))
        .routes(routes!(unarchive_deal))
}

fn query_params<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, AppError> {
    query
        .map(|Query(params)| params)
        .map_err(|rejection| AppError::Validation(rejection.body_text()))
}

#[utoipa::path(
    get,
    path = "/deals",
    tag = TAG,
    operation_id = "deals_list",
    security(("session_cookie" = [])),
    params(RecordQueryParams, PaginationQuery, IncludeQuery),
    responses(
        (status = 200, description = "Deals visible under the `deals.read` scope and in pipelines open to the user's teams, filtered, searched and sorted through the record query engine. Besides `filter` as JSON, simple conditions go as `filter[field][operator]=value` (`filter[status][eq]=open`, `filter[amount][gte]=1000`, `filter[stage_id][in]=a,b`), combined with `and`. Default order: newest first. `stage`, `contact`, `company`, `owner` and `lost_reason` are always embedded; `include` accepts those names", body = Page<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search, include or pagination (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn list_deals(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    RecordListQuery(record_query): RecordListQuery,
    include: Result<Query<IncludeQuery>, QueryRejection>,
) -> Result<Json<Page<Value>>, ApiError> {
    query_params(include)?.parse(&DEAL_RELATIONSHIPS)?;
    let listing =
        service::list(&Ctx::from_state(&state, &tenant.scope).await?, record_query).await?;
    let items: Vec<DealResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &RecordType::DEAL, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/deals",
    tag = TAG,
    operation_id = "deals_create",
    security(("session_cookie" = [])),
    request_body = DealRequest,
    responses(
        (status = 201, description = "Deal created in the given stage, or in the first open stage of the given (or default) pipeline; the owner defaults to the current user and the currency to the organization's. Publishes `deal.created` (and `deal.won`/`deal.lost` when created closed)", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, including `amount` (a decimal string or number), `custom_fields.<key>`, a hidden or inactive pipeline (`validation_error` with `details.fields`), or fields required by the stage (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role)", body = ErrorEnvelope),
    )
)]
async fn create_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<DealRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let deal = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_input(client.0.ip, None),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted(
                &state,
                &tenant.scope,
                &RecordType::DEAL,
                DealResponse::from(deal),
            )
            .await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/deals/board",
    tag = TAG,
    operation_id = "deals_board",
    security(("session_cookie" = [])),
    params(BoardParams, RecordQueryParams),
    responses(
        (status = 200, description = "Kanban of a pipeline (the default one when `pipeline_id` is omitted): one column per active stage with the count and the sum of `amount` of the deals matching the filter and the read scope, plus the cards of the requested page (`per_column` cards per column). Pass `stage_id` to page through a single column. Accepts the same `filter` (JSON or `filter[field][operator]=value`), `search` and `sort` as the deal listing; `total_amount` is a decimal string", body = Data<BoardResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Pipeline not found or restricted to other teams (`pipeline_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search, stage, `per_column` or `page` (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn deal_board(
    State(state): State<AppState>,
    tenant: AuthContext,
    uri: Uri,
    board: Result<Query<BoardParams>, QueryRejection>,
    query: Result<Query<RecordQueryParams>, QueryRejection>,
) -> Result<Json<Data<Value>>, ApiError> {
    let board = query_params(board)?;
    let per_column = board.per_column.unwrap_or(DEFAULT_BOARD_PER_COLUMN);
    let page = board.page.unwrap_or(1);
    let mut violations = Vec::new();
    if !(1..=100).contains(&per_column) {
        violations.push(FieldViolation::new(
            "per_column",
            "must be an integer between 1 and 100",
        ));
    }
    if page < 1 {
        violations.push(FieldViolation::new("page", "must be at least 1"));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid board query", violations).into());
    }
    let record_query = query_params(query)?.into_query_with(Pagination::default(), uri.query())?;
    let result = service::board(
        &Ctx::from_state(&state, &tenant.scope).await?,
        BoardQuery {
            pipeline_id: board.pipeline_id,
            stage_id: board.stage_id,
            per_column,
            page,
            query: record_query,
        },
    )
    .await?;
    let redactor = record_redactor(&state, &tenant.scope, &RecordType::DEAL).await?;
    let mut board = serde_json::to_value(BoardResponse::from(result))
        .map_err(|error| AppError::Internal(format!("failed to serialize the board: {error}")))?;
    if let Some(columns) = board.get_mut("columns").and_then(Value::as_array_mut) {
        for column in columns {
            if let Some(items) = column.get_mut("items").and_then(Value::as_array_mut) {
                *items = redacted_items(
                    &state,
                    &tenant.scope,
                    &RecordType::DEAL,
                    std::mem::take(items),
                )
                .await?;
            }
            if redactor.hides("amount")
                && let Some(column) = column.as_object_mut()
            {
                column.remove("total_amount");
            }
        }
    }
    Ok(Json(Data::new(board)))
}

#[utoipa::path(
    get,
    path = "/deals/{deal_id}",
    tag = TAG,
    operation_id = "deals_get",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id"), IncludeQuery),
    responses(
        (status = 200, description = "Deal with its custom fields and the stage, contact, company, owner and lost reason (always embedded; `include` accepts those names)", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found, outside the read scope or in a pipeline restricted to other teams (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown relationship in `include` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn get_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
    include: Result<Query<IncludeQuery>, QueryRejection>,
) -> Result<Json<Data<Value>>, ApiError> {
    query_params(include)?.parse(&DEAL_RELATIONSHIPS)?;
    let deal = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &deal_id).await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/deals/{deal_id}",
    tag = TAG,
    operation_id = "deals_update",
    security(("session_cookie" = [])),
    params(
        ("deal_id" = String, Path, description = "Deal id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the deal as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = DealRequest,
    responses(
        (status = 200, description = "Deal updated; omitted fields stay unchanged and `null` clears them. A new `stage_id` or `status` moves the deal like `/move`, `/won`, `/lost` and `/reopen`. Publishes `deal.amount_changed`, `deal.owner_changed`, `deal.expected_close_date_changed` and the stage events that apply, plus `deal.updated` with the other changed fields (fields covered by a specific event are not repeated in it)", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The deal changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, including `amount`, `custom_fields.<key>`, a missing `lost_reason_id` when losing or an invalid `If-Match` (`validation_error` with `details.fields`), or fields required by the stage rules when `stage_id` changes (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role)", body = ErrorEnvelope),
    )
)]
async fn update_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<DealRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_input(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/deals/{deal_id}",
    tag = TAG,
    operation_id = "deals_delete",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 204, description = "Deal deleted (soft delete); publishes `deal.deleted`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.delete` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the delete scope (`deal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/move",
    tag = TAG,
    operation_id = "deals_move",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = MoveDealRequest,
    responses(
        (status = 200, description = "Deal moved to the stage; a won stage wins it, a lost stage loses it (requires `lost_reason_id`) and an open stage reopens a closed deal. `fields` fills system or custom fields together with the move (for the stage rules). Publishes `deal.stage_changed`, `deal.pipeline_changed` when the stage belongs to another pipeline, and `deal.won`, `deal.lost` or `deal.reopened` when the status changes", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found, outside the update scope or in a pipeline hidden from the user's teams (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or inactive stage, missing lost reason, or fields required to leave the current stage or enter the new one are empty (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role, with `details.fields`); other invalid input answers `validation_error` with `details.fields`", body = ErrorEnvelope),
    )
)]
async fn move_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<MoveDealRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::move_to_stage(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/move-pipeline",
    tag = TAG,
    operation_id = "deals_move_pipeline",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = MoveDealPipelineRequest,
    responses(
        (status = 200, description = "Deal moved to another pipeline, into `stage_id` or, when omitted, the first active stage of the same kind as the current one. Publishes `deal.pipeline_changed` (old and new pipeline) and `deal.stage_changed`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found, outside the update scope or in a hidden pipeline (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown, hidden or inactive pipeline, stage outside the pipeline, missing lost reason or stage requirements (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role); other invalid input answers `validation_error` with `details.fields`", body = ErrorEnvelope),
    )
)]
async fn move_deal_pipeline(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<MoveDealPipelineRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::move_to_pipeline(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/won",
    tag = TAG,
    operation_id = "deals_win",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Deal won: moved to the first won stage of its pipeline with `closed_at`. Publishes `deal.stage_changed` and `deal.won`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The pipeline has no active won stage (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn win_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::win(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/lost",
    tag = TAG,
    operation_id = "deals_lose",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = LoseDealRequest,
    responses(
        (status = 200, description = "Deal lost with an active lost reason: moved to the first lost stage of its pipeline with `closed_at`. Publishes `deal.stage_changed` and `deal.lost`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Missing or inactive lost reason (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn lose_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<LoseDealRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::lose(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/reopen",
    tag = TAG,
    operation_id = "deals_reopen",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = ReopenDealRequest,
    responses(
        (status = 200, description = "Closed deal reopened in the given open stage (or the first open stage of its pipeline); clears `closed_at` and the lost reason. Publishes `deal.stage_changed` and `deal.reopened`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The stage is not an active open stage (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reopen_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<ReopenDealRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::reopen(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/archive",
    tag = TAG,
    operation_id = "deals_archive",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Deal archived (hidden from listings unless `archived=include|only`); publishes `deal.archived`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn archive_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::archive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/unarchive",
    tag = TAG,
    operation_id = "deals_unarchive",
    security(("session_cookie" = [])),
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Deal restored from the archive; publishes `deal.unarchived`", body = Data<DealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or outside the update scope (`deal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unarchive_deal(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let deal = service::unarchive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &deal_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::DEAL,
            DealResponse::from(deal),
        )
        .await?,
    )))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DealStatus)]
pub(crate) enum DealStatusSchema {
    Open,
    Won,
    Lost,
}

impl From<DealStatus> for DealStatusSchema {
    fn from(status: DealStatus) -> Self {
        match status {
            DealStatus::Open => Self::Open,
            DealStatus::Won => Self::Won,
            DealStatus::Lost => Self::Lost,
        }
    }
}

impl From<DealStatusSchema> for DealStatus {
    fn from(status: DealStatusSchema) -> Self {
        match status {
            DealStatusSchema::Open => Self::Open,
            DealStatusSchema::Won => Self::Won,
            DealStatusSchema::Lost => Self::Lost,
        }
    }
}

pub fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
#[schema(as = DecimalInput, example = "15000.00")]
pub enum DecimalInputSchema {
    Text(String),
    Number(f64),
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealStageResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: StageKindSchema,
    pub(crate) probability: u8,
    #[schema(example = 7)]
    pub(crate) rotting_days: Option<u16>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealNamedResponse {
    pub(crate) id: String,
    pub(crate) name: String,
}

impl From<NamedRef> for DealNamedResponse {
    fn from(value: NamedRef) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealOwnerResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DealResponse {
    pub(crate) id: String,
    #[schema(example = "Acme onboarding")]
    pub(crate) title: String,
    #[schema(example = "15000.50")]
    pub(crate) amount: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) amount_base: Option<String>,
    pub(crate) exchange_rate: Option<String>,
    pub(crate) mrr: Option<String>,
    pub(crate) arr: Option<String>,
    pub(crate) tcv: Option<String>,
    pub(crate) contract_start: Option<String>,
    pub(crate) contract_end: Option<String>,
    pub(crate) renewal_of_deal_id: Option<String>,
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: String,
    pub(crate) status: DealStatusSchema,
    pub(crate) probability: Option<u8>,
    #[schema(example = "2026-12-31")]
    pub(crate) expected_close_date: Option<String>,
    pub(crate) contact_id: Option<String>,
    pub(crate) company_id: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) lost_reason_id: Option<String>,
    pub(crate) lost_reason_note: Option<String>,
    pub(crate) closed_at: Option<String>,
    pub(crate) stage_entered_at: String,
    #[schema(example = 12)]
    pub(crate) days_in_stage: i64,
    pub(crate) is_rotting: bool,
    pub(crate) rotting_at: Option<String>,
    pub(crate) last_activity_at: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) archived_at: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) stage: DealStageResponse,
    pub(crate) contact: Option<DealNamedResponse>,
    pub(crate) company: Option<DealNamedResponse>,
    pub(crate) owner: Option<DealOwnerResponse>,
    pub(crate) lost_reason: Option<DealNamedResponse>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl From<DealView> for DealResponse {
    fn from(view: DealView) -> Self {
        let record = view.record;
        let deal = record.deal;
        let rotting = RottingState::of(
            deal.status == DealStatus::Open,
            deal.stage_entered_at,
            record.stage.rotting_days,
            Utc::now(),
        );
        Self {
            id: deal.id.to_string(),
            title: deal.title,
            amount: deal.amount.map(money),
            currency: deal.currency,
            amount_base: deal.revenue.amount_base.map(money),
            exchange_rate: deal
                .revenue
                .exchange_rate
                .map(|rate| rate.normalize().to_string()),
            mrr: deal.revenue.mrr.map(money),
            arr: deal.revenue.arr.map(money),
            tcv: deal.revenue.tcv.map(money),
            contract_start: deal.revenue.contract_start.map(date),
            contract_end: deal.revenue.contract_end.map(date),
            renewal_of_deal_id: deal.revenue.renewal_of_deal_id.map(|id| id.to_string()),
            pipeline_id: deal.pipeline_id.to_string(),
            stage_id: deal.stage_id.to_string(),
            status: deal.status.into(),
            probability: deal.probability,
            expected_close_date: deal.expected_close_date.map(date),
            contact_id: deal.contact_id.map(|id| id.to_string()),
            company_id: deal.company_id.map(|id| id.to_string()),
            owner_id: deal.owner_id.as_ref().map(|id| id.as_str().to_owned()),
            source: deal.source,
            lost_reason_id: deal.lost_reason_id.map(|id| id.to_string()),
            lost_reason_note: deal.lost_reason_note,
            closed_at: deal.closed_at.map(instant),
            stage_entered_at: instant(deal.stage_entered_at),
            days_in_stage: rotting.days_in_stage,
            is_rotting: rotting.is_rotting,
            rotting_at: rotting.rotting_at.map(instant),
            last_activity_at: deal.last_activity_at.map(instant),
            created_by: deal.created_by.as_ref().map(|id| id.as_str().to_owned()),
            created_at: instant(deal.created_at),
            updated_at: instant(deal.updated_at),
            archived_at: deal.archived_at.map(instant),
            custom_fields: view.custom_fields,
            contributed: view.contributed,
            stage: DealStageResponse {
                id: record.stage.id.to_string(),
                name: record.stage.name,
                kind: record.stage.kind.into(),
                probability: record.stage.probability,
                rotting_days: record.stage.rotting_days,
            },
            contact: record.contact.map(Into::into),
            company: record.company.map(Into::into),
            owner: record.owner.map(|owner| DealOwnerResponse {
                id: UserId::into_inner(owner.id),
                display_name: owner.display_name,
            }),
            lost_reason: record.lost_reason.map(Into::into),
        }
    }
}

fn amount_text(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text),
        Value::Number(number) => Some(number.to_string()),
        other => Some(other.to_string()),
    }
}

fn integer(value: Value) -> Option<i64> {
    match value {
        Value::Null => None,
        Value::Number(number) => Some(
            number
                .as_i64()
                .or_else(|| {
                    number
                        .as_f64()
                        .filter(|value| value.fract() == 0.0)
                        .map(|value| value as i64)
                })
                .unwrap_or(-1),
        ),
        Value::String(text) => Some(text.trim().parse().unwrap_or(-1)),
        _ => Some(-1),
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct DealRequest {
    #[schema(example = "Acme onboarding")]
    pub(crate) title: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "15000.50")]
    pub(crate) amount: Option<Option<Value>>,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) pipeline_id: Option<String>,
    pub(crate) stage_id: Option<String>,
    pub(crate) status: Option<DealStatusSchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>)]
    pub(crate) probability: Option<Option<Value>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2026-12-31")]
    pub(crate) expected_close_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) contact_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) company_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) owner_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) source: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) lost_reason_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) lost_reason_note: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2026-11-01")]
    pub(crate) contract_start: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2027-10-31")]
    pub(crate) contract_end: Option<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Map<String, Value>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl DealRequest {
    pub(crate) fn into_input(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> DealInput {
        DealInput {
            title: self.title,
            amount: self.amount.map(|value| value.and_then(amount_text)),
            currency: self.currency,
            pipeline_id: self.pipeline_id,
            stage_id: self.stage_id,
            status: self
                .status
                .map(|status| DealStatus::from(status).as_str().to_owned()),
            probability: self.probability.map(|value| value.and_then(integer)),
            expected_close_date: self.expected_close_date,
            contact_id: self.contact_id,
            company_id: self.company_id,
            owner_id: self.owner_id,
            source: self.source,
            lost_reason_id: self.lost_reason_id,
            lost_reason_note: self.lost_reason_note,
            contract_start: self.contract_start,
            contract_end: self.contract_end,
            custom_fields: self.custom_fields,
            contributed: self.contributed,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct MoveDealRequest {
    pub(crate) stage_id: String,
    pub(crate) lost_reason_id: Option<String>,
    pub(crate) lost_reason_note: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<Object>, example = json!({ "amount": 15000, "expected_close_date": "2026-12-31" }))]
    pub(crate) fields: Map<String, Value>,
}

impl MoveDealRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> MoveDealCommand {
        MoveDealCommand {
            stage_id: self.stage_id,
            lost_reason_id: self.lost_reason_id,
            lost_reason_note: self.lost_reason_note,
            fields: self.fields,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct MoveDealPipelineRequest {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: Option<String>,
    pub(crate) lost_reason_id: Option<String>,
    pub(crate) lost_reason_note: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) fields: Map<String, Value>,
}

impl MoveDealPipelineRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> MoveDealPipelineCommand {
        MoveDealPipelineCommand {
            pipeline_id: self.pipeline_id,
            stage_id: self.stage_id,
            lost_reason_id: self.lost_reason_id,
            lost_reason_note: self.lost_reason_note,
            fields: self.fields,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct LoseDealRequest {
    pub(crate) lost_reason_id: Option<String>,
    pub(crate) lost_reason_note: Option<String>,
}

impl LoseDealRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> LoseDealCommand {
        LoseDealCommand {
            lost_reason_id: self.lost_reason_id,
            lost_reason_note: self.lost_reason_note,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct ReopenDealRequest {
    pub(crate) stage_id: Option<String>,
}

impl ReopenDealRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ReopenDealCommand {
        ReopenDealCommand {
            stage_id: self.stage_id,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct BoardParams {
    pub(crate) pipeline_id: Option<String>,
    pub(crate) stage_id: Option<String>,
    #[param(minimum = 1, maximum = 100, example = 20)]
    pub(crate) per_column: Option<u64>,
    #[param(minimum = 1, example = 1)]
    pub(crate) page: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BoardColumnResponse {
    pub(crate) stage: PipelineStageResponse,
    pub(crate) count: u64,
    #[schema(example = "45000.00")]
    pub(crate) total_amount: String,
    pub(crate) has_more: bool,
    pub(crate) items: Vec<DealResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BoardResponse {
    pub(crate) pipeline: PipelineSummaryResponse,
    pub(crate) page: u64,
    pub(crate) per_column: u64,
    pub(crate) columns: Vec<BoardColumnResponse>,
}

impl From<Board> for BoardResponse {
    fn from(board: Board) -> Self {
        let shown = board.page * board.per_column;
        Self {
            pipeline: board.pipeline.into(),
            page: board.page,
            per_column: board.per_column,
            columns: board
                .columns
                .into_iter()
                .map(|column: BoardColumn| BoardColumnResponse {
                    has_more: column.count > shown,
                    count: column.count,
                    total_amount: money(column.total_amount),
                    stage: column.stage.into(),
                    items: column.items.into_iter().map(Into::into).collect(),
                })
                .collect(),
        }
    }
}

fn object(values: Value) -> Map<String, Value> {
    match values {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn deal_body(values: Value) -> Value {
    let mut body = object(values);
    if let Some(Value::String(text)) = body.get("probability") {
        let probability = text
            .trim()
            .parse::<f64>()
            .ok()
            .map_or(Value::Null, |value| Value::from(value.round() as i64));
        body.insert("probability".to_owned(), probability);
    }
    if let Some(Value::String(currency)) = body.get_mut("currency") {
        *currency = currency.to_uppercase();
    }
    Value::Object(body)
}

pub(crate) struct DealHandler;

#[async_trait]
impl RecordTypeHandler for DealHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        "deals.deal"
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let input = values_request::<DealRequest>(deal_body(values))?.into_input(None, None);
        let view = service::create(&Ctx::from_state(ctx.state, ctx.scope).await?, input).await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::DEAL,
            DealResponse::from(view),
        )
        .await
    }

    async fn create_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        values: Value,
    ) -> Result<PendingCommit, AppError> {
        let command = values_request::<DealRequest>(deal_body(values))?.into_input(None, None);
        service::create_in_tx(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            tx,
            command,
            Default::default(),
        )
        .await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let input = values_request::<DealRequest>(deal_body(values))?.into_input(None, None);
        let view = service::update(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            input,
        )
        .await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::DEAL,
            DealResponse::from(view),
        )
        .await
    }

    async fn update_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        values: Value,
    ) -> Result<Option<PendingCommit>, AppError> {
        let input = values_request::<DealRequest>(deal_body(values))?.into_input(None, None);
        service::update_in_tx(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            tx,
            &id.to_string(),
            input,
        )
        .await
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::delete(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let deal_ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let Some(view) = optional_record(service::get(&deal_ctx, &id.to_string()).await)? else {
            return Ok(None);
        };
        Ok(Some(
            redacted(
                ctx.state,
                ctx.scope,
                &RecordType::DEAL,
                DealResponse::from(view),
            )
            .await?,
        ))
    }

    async fn archive(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::archive(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
        .map(|_| ())
    }

    async fn restore(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::unarchive(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
        .map(|_| ())
    }
}
