use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{PublicationRequest, Revision};
use crate::service::{self, CaptureInput, RequestInput, RestoreInput, StatusView, WithdrawInput};
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, JsonBody, Pagination, PaginationQuery};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "publication";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_revisions, capture_revision))
        .routes(routes!(get_revision))
        .routes(routes!(restore_revision))
        .routes(routes!(create_request))
        .routes(routes!(create_withdrawal))
        .routes(routes!(cancel_request))
        .routes(routes!(retry_request))
        .routes(routes!(publication_status))
        .routes(routes!(publication_statuses))
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = PublicationRevision)]
pub(crate) struct RevisionResponse {
    pub(crate) id: String,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) number: u32,
    pub(crate) format: u32,
    #[schema(value_type = Object)]
    pub(crate) payload: Option<Value>,
    pub(crate) payload_hash: String,
    #[schema(value_type = String, format = DateTime)]
    pub(crate) draft_version: DateTime<Utc>,
    pub(crate) note: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) created_by: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub(crate) created_at: DateTime<Utc>,
}

impl RevisionResponse {
    pub(crate) fn of(revision: Revision, with_payload: bool) -> Self {
        Self {
            id: revision.id.to_string(),
            record_type: revision.record.record_type.as_str().to_owned(),
            record_id: revision.record.record_id.to_string(),
            number: revision.number,
            format: revision.format,
            payload: with_payload.then_some(revision.payload),
            payload_hash: revision.payload_hash,
            draft_version: revision.draft_version,
            note: revision.note,
            title: revision.title,
            created_by: revision.created_by,
            created_at: revision.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = PublicationRequest)]
pub(crate) struct RequestResponse {
    pub(crate) id: String,
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    #[schema(example = "activate")]
    pub(crate) action: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) generation: u64,
    #[schema(example = "completed")]
    pub(crate) status: String,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) run_at: Option<DateTime<Utc>>,
    pub(crate) requested_by: Option<String>,
    pub(crate) reason_code: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub(crate) created_at: DateTime<Utc>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

impl From<PublicationRequest> for RequestResponse {
    fn from(request: PublicationRequest) -> Self {
        Self {
            id: request.id.to_string(),
            record_type: request.record.record_type.as_str().to_owned(),
            record_id: request.record.record_id.to_string(),
            action: request.action.as_str().to_owned(),
            revision_id: request.revision_id.map(|id| id.to_string()),
            generation: request.generation,
            status: request.status.as_str().to_owned(),
            run_at: request.run_at,
            requested_by: request.requested_by,
            reason_code: request.reason_code,
            created_at: request.created_at,
            completed_at: request.completed_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = PublicationStatus)]
pub(crate) struct StatusResponse {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) generation: u64,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) first_published_at: Option<DateTime<Utc>>,
    pub(crate) active_revision: Option<RevisionResponse>,
    pub(crate) requests: Vec<RequestResponse>,
}

impl From<StatusView> for StatusResponse {
    fn from(view: StatusView) -> Self {
        Self {
            record_type: view.record.record_type.as_str().to_owned(),
            record_id: view.record.record_id.to_string(),
            generation: view
                .subject
                .as_ref()
                .map_or(0, |subject| subject.generation),
            first_published_at: view.subject.and_then(|subject| subject.first_published_at),
            active_revision: view
                .active
                .map(|revision| RevisionResponse::of(revision, false)),
            requests: view.requests.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct RecordQuery {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaptureRequest {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) expected_version: Option<DateTime<Utc>>,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestoreRequest {
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) expected_version: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublishRequest {
    pub(crate) revision_id: String,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) at: Option<DateTime<Utc>>,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WithdrawRequest {
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) idempotency_key: String,
}

#[utoipa::path(
    get,
    path = "/publication/revisions",
    tag = TAG,
    operation_id = "publication_revisions_list",
    security(("session_cookie" = [])),
    params(RecordQuery, PaginationQuery),
    description = "Revisions of a publishable record, newest first, without payloads; `title` is the record's title field in each revision. Needs read access to the record.",
    responses(
        (status = 200, description = "Revisions", body = Page<RevisionResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Record not found or not readable (`publication_record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The record type does not take part in publication (`publication_not_publishable`)", body = ErrorEnvelope),
    )
)]
async fn list_revisions(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(query): Query<RecordQuery>,
    pagination: Pagination,
) -> Result<Json<Page<RevisionResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let (items, total) = service::revisions(
        &ctx,
        &query.record_type,
        &query.record_id,
        u32::try_from(pagination.limit()).unwrap_or(u32::MAX),
        u32::try_from(pagination.offset()).unwrap_or(u32::MAX),
    )
    .await?;
    Ok(Json(Page::new(
        items
            .into_iter()
            .map(|revision| RevisionResponse::of(revision, false))
            .collect(),
        total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/publication/revisions",
    tag = TAG,
    operation_id = "publication_revisions_capture",
    security(("session_cookie" = [])),
    request_body = CaptureRequest,
    description = "Captures the saved draft fields of a record as an immutable revision (`publication.revision_captured`). `expected_version` is the `updated_at` of the saved draft; a newer save answers 409. Needs update access to the record.",
    responses(
        (status = 201, description = "Captured revision", body = Data<RevisionResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`publication_record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The draft changed since `expected_version` or the definition is invalid (`publication_definition_invalid`)", body = ErrorEnvelope),
        (status = 413, description = "The content is larger than 2 MiB (`publication_payload_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body or record type not publishable (`publication_not_publishable`)", body = ErrorEnvelope),
    )
)]
async fn capture_revision(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CaptureRequest>,
) -> Result<(StatusCode, Json<Data<RevisionResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let revision = service::capture(
        &ctx,
        CaptureInput {
            record_type: request.record_type,
            record_id: request.record_id,
            expected_version: request.expected_version,
            note: request.note,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(RevisionResponse::of(revision, true))),
    ))
}

#[utoipa::path(
    get,
    path = "/publication/revisions/{revision_id}",
    tag = TAG,
    operation_id = "publication_revisions_get",
    security(("session_cookie" = [])),
    params(("revision_id" = String, Path, description = "Revision id")),
    description = "One revision with its payload. Needs read access to the record.",
    responses(
        (status = 200, description = "Revision", body = Data<RevisionResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Revision not found (`publication_revision_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_revision(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(revision_id): Path<String>,
) -> Result<Json<Data<RevisionResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let revision = service::revision(&ctx, &revision_id).await?;
    Ok(Json(Data::new(RevisionResponse::of(revision, true))))
}

#[utoipa::path(
    post,
    path = "/publication/revisions/{revision_id}/restore",
    tag = TAG,
    operation_id = "publication_revisions_restore",
    security(("session_cookie" = [])),
    params(("revision_id" = String, Path, description = "Revision id")),
    request_body = RestoreRequest,
    description = "Copies a revision back into the draft fields as an ordinary edit. The published content does not change until a new revision is published.",
    responses(
        (status = 200, description = "Updated record", body = Data<Object>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing update permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Revision not found (`publication_revision_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The draft changed since `expected_version`", body = ErrorEnvelope),
    )
)]
async fn restore_revision(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(revision_id): Path<String>,
    JsonBody(request): JsonBody<RestoreRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let record = service::restore(
        &ctx,
        RestoreInput {
            revision_id,
            expected_version: request.expected_version,
        },
    )
    .await?;
    Ok(Json(Data::new(record)))
}

#[utoipa::path(
    post,
    path = "/publication/requests",
    tag = TAG,
    operation_id = "publication_requests_create",
    security(("session_cookie" = [])),
    request_body = PublishRequest,
    description = "Publishes a revision now, or at `at` (UTC), replacing every open request of the record. Approval policies of the moment `publication.activate` are checked against this revision; when one applies the request waits for approval (`awaiting_approval`). The status is `completed` when the revision became the published one. Repeating the same `idempotency_key` returns the first result; reusing it for another command answers 409. Needs the publish permission declared by the record type.",
    responses(
        (status = 201, description = "Publication request", body = Data<RequestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the publish permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Revision not found (`publication_revision_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Idempotency key reused (`publication_idempotency_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body", body = ErrorEnvelope),
    )
)]
async fn create_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<PublishRequest>,
) -> Result<(StatusCode, Json<Data<RequestResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let request = service::request(
        &ctx,
        RequestInput {
            revision_id: request.revision_id,
            at: request.at,
            idempotency_key: request.idempotency_key,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(request.into()))))
}

#[utoipa::path(
    post,
    path = "/publication/withdrawals",
    tag = TAG,
    operation_id = "publication_withdrawals_create",
    security(("session_cookie" = [])),
    request_body = WithdrawRequest,
    description = "Withdraws the published revision of a record and cancels its open requests, so an old schedule cannot publish it again. Needs the publish permission declared by the record type.",
    responses(
        (status = 201, description = "Withdrawal request", body = Data<RequestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the publish permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`publication_record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Idempotency key reused (`publication_idempotency_conflict`)", body = ErrorEnvelope),
    )
)]
async fn create_withdrawal(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<WithdrawRequest>,
) -> Result<(StatusCode, Json<Data<RequestResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let request = service::withdraw(
        &ctx,
        WithdrawInput {
            record_type: request.record_type,
            record_id: request.record_id,
            idempotency_key: request.idempotency_key,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(request.into()))))
}

#[utoipa::path(
    post,
    path = "/publication/requests/{request_id}/cancel",
    tag = TAG,
    operation_id = "publication_requests_cancel",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Publication request id")),
    description = "Cancels an open request (waiting for approval, scheduled or ready).",
    responses(
        (status = 200, description = "Cancelled request", body = Data<RequestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the publish permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`publication_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request is not open (`publication_request_not_open`)", body = ErrorEnvelope),
    )
)]
async fn cancel_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<Json<Data<RequestResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(
        service::cancel(&ctx, &request_id).await?.into(),
    )))
}

#[utoipa::path(
    post,
    path = "/publication/requests/{request_id}/retry",
    tag = TAG,
    operation_id = "publication_requests_retry",
    security(("session_cookie" = [])),
    params(("request_id" = String, Path, description = "Publication request id")),
    description = "Runs a failed request again with every check repeated.",
    responses(
        (status = 200, description = "Request after the new attempt", body = Data<RequestResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing the publish permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Request not found (`publication_request_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The request did not fail (`publication_request_not_failed`)", body = ErrorEnvelope),
    )
)]
async fn retry_request(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(request_id): Path<String>,
) -> Result<Json<Data<RequestResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(
        service::retry(&ctx, &request_id).await?.into(),
    )))
}

#[utoipa::path(
    get,
    path = "/publication/status",
    tag = TAG,
    operation_id = "publication_status_get",
    security(("session_cookie" = [])),
    params(RecordQuery),
    description = "Published revision, generation and the latest requests of a record.",
    responses(
        (status = 200, description = "Publication status", body = Data<StatusResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`publication_record_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The record type does not take part in publication (`publication_not_publishable`)", body = ErrorEnvelope),
    )
)]
async fn publication_status(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(query): Query<RecordQuery>,
) -> Result<Json<Data<StatusResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::status(&ctx, &query.record_type, &query.record_id).await?;
    Ok(Json(Data::new(view.into())))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct StatusesQuery {
    pub(crate) record_type: String,
    #[param(example = "0190a1b2-0000-7000-8000-000000000001,0190a1b2-0000-7000-8000-000000000002")]
    pub(crate) record_ids: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = PublicationRecordStatus)]
pub(crate) struct RecordStatusResponse {
    pub(crate) record_id: String,
    pub(crate) published: bool,
    pub(crate) open_request: Option<RequestResponse>,
}

#[utoipa::path(
    get,
    path = "/publication/statuses",
    tag = TAG,
    operation_id = "publication_statuses_list",
    security(("session_cookie" = [])),
    params(StatusesQuery),
    description = "Whether each record has a published revision and its open request (waiting for approval, scheduled or ready), for up to 100 records of one type. Records the caller cannot read are left out.",
    responses(
        (status = 200, description = "Publication status per readable record", body = Data<Vec<RecordStatusResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid ids or record type not publishable (`publication_not_publishable`)", body = ErrorEnvelope),
    )
)]
async fn publication_statuses(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(query): Query<StatusesQuery>,
) -> Result<Json<Data<Vec<RecordStatusResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let statuses = service::statuses(&ctx, &query.record_type, &query.record_ids).await?;
    Ok(Json(Data::new(
        statuses
            .into_iter()
            .map(|status| RecordStatusResponse {
                record_id: status.record_id.to_string(),
                published: status.published,
                open_request: status.open_request.map(Into::into),
            })
            .collect(),
    )))
}
