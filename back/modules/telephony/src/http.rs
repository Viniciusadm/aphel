use aphel_contract_crm::CrmRecordTypes;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    ActivityCallView, CallDirection, CallEventCommand, CallOutcome, CallRecordingView, CallReport,
    CallReportQuery, CallReportUser, CallSession, CallSessionItem, CallSessionStatus,
    CallSessionView, CreateCallOutcomeCommand, LogCallCommand, SessionCallCommand, SessionCounts,
    SessionItemStatus, StartCallCommand, StartCallSessionCommand, StartedCallView,
    UpdateCallCommand, UpdateCallOutcomeCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{
    Data, ErrorEnvelope, Page, double_option, timestamp as instant,
};
use aphel_sdk::platform::api::dto::files::UserReferenceResponse;
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::field_permissions::redacted;
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::tasks::TaskRecord;
use aphel_sdk::tasks::dto::ActivityResponse;

pub(crate) const TAG: &str = "telephony";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(log_call))
        .routes(routes!(update_call))
        .routes(routes!(call_recording))
        .routes(routes!(call_event))
        .routes(routes!(list_providers))
        .routes(routes!(start_call))
        .routes(routes!(call_report))
        .routes(routes!(list_outcomes, create_outcome))
        .routes(routes!(update_outcome))
        .routes(routes!(reorder_outcomes))
        .routes(routes!(list_sessions, start_session))
        .routes(routes!(get_session))
        .routes(routes!(session_items))
        .routes(routes!(session_call))
        .routes(routes!(session_skip))
        .routes(routes!(session_cursor))
        .routes(routes!(session_finish))
}

pub(crate) async fn activity_json(
    state: &AppState,
    scope: &AccessScope,
    record: TaskRecord,
) -> Result<Value, AppError> {
    redacted(state, scope, &RecordType::ACTIVITY, record.response).await
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = CallDirection)]
pub(crate) enum CallDirectionSchema {
    Inbound,
    Outbound,
}

impl From<CallDirection> for CallDirectionSchema {
    fn from(value: CallDirection) -> Self {
        match value {
            CallDirection::Inbound => Self::Inbound,
            CallDirection::Outbound => Self::Outbound,
        }
    }
}

impl From<CallDirectionSchema> for CallDirection {
    fn from(value: CallDirectionSchema) -> Self {
        match value {
            CallDirectionSchema::Inbound => Self::Inbound,
            CallDirectionSchema::Outbound => Self::Outbound,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallOutcomeSummaryResponse {
    #[schema(example = "answered")]
    pub(crate) key: String,
    #[schema(example = "Answered")]
    pub(crate) name: Option<String>,
    pub(crate) is_connected: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActivityCallResponse {
    pub(crate) direction: CallDirectionSchema,
    pub(crate) outcome: CallOutcomeSummaryResponse,
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) has_recording: bool,
    pub(crate) updated_at: String,
}

impl From<ActivityCallView> for ActivityCallResponse {
    fn from(call: ActivityCallView) -> Self {
        let has_recording = call.recording_url.is_some() || call.provider_call_id.is_some();
        Self {
            direction: call.direction.into(),
            outcome: CallOutcomeSummaryResponse {
                key: call.outcome.key,
                name: call.outcome.name,
                is_connected: call.outcome.is_connected,
            },
            duration_seconds: call.duration_seconds,
            phone_number: call.phone_number,
            recording_url: call.recording_url,
            provider: call.provider,
            provider_call_id: call.provider_call_id,
            session_id: call.session_id.map(|id| id.to_string()),
            has_recording,
            updated_at: instant(call.updated_at),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub(crate) struct CallActivityResponse {
    #[serde(flatten)]
    pub(crate) activity: ActivityResponse,
    pub(crate) call: Option<ActivityCallResponse>,
}

#[utoipa::path(
    post,
    path = "/telephony",
    tag = TAG,
    operation_id = "calls_log",
    security(("session_cookie" = [])),
    request_body = LogCallRequest,
    responses(
        (status = 201, description = "Call logged as a done activity of the `call` type with the call details in `call` (`activity.created` and `call.logged`). `direction` defaults to `outbound`; `outcome_key` is one of the active call outcomes of the organization; `duration_seconds` (0–86400) also fills `duration_minutes`, rounded up; `notes` become the description; `occurred_at` is the completion time (default now); `title` defaults to `<call type> · <outcome>`. `provider` + `provider_call_id` tie the call to a telephony module (unique)", body = Data<CallActivityResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, unknown or inactive outcome, unreadable record (`validation_error`) or no call activity type (`call_activity_type_missing`)", body = ErrorEnvelope),
    )
)]
async fn log_call(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<LogCallRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let record = service::log(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            activity_json(&state, &tenant.scope, record).await?,
        )),
    ))
}

#[utoipa::path(
    patch,
    path = "/telephony/{activity_id}",
    tag = TAG,
    operation_id = "calls_update",
    security(("session_cookie" = [])),
    params(
        ("activity_id" = String, Path, description = "Id of the call activity"),
        ("If-Match" = Option<String>, Header, description = "`call.updated_at` as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateCallRequest,
    responses(
        (status = 200, description = "Call details updated (`call.updated` with `changes`); only the sent fields change, `null` clears duration, phone number or recording", body = Data<CallActivityResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.update` for this activity (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`) or it is not a logged call (`call_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The call changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or unknown outcome (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_call(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(activity_id): Path<String>,
    JsonBody(request): JsonBody<UpdateCallRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(
        activity_json(&state, &tenant.scope, record).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/telephony/{activity_id}/recording",
    tag = TAG,
    operation_id = "calls_recording",
    security(("session_cookie" = [])),
    params(("activity_id" = String, Path, description = "Id of the call activity")),
    responses(
        (status = 200, description = "Address of the call recording: asked to the telephony provider of the call when it is enabled (it may expire at `expires_at`), otherwise the stored `recording_url`", body = Data<CallRecordingResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`), not a call (`call_not_found`) or without recording (`call_recording_not_found`)", body = ErrorEnvelope),
    )
)]
async fn call_recording(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(activity_id): Path<String>,
) -> Result<Json<Data<CallRecordingResponse>>, ApiError> {
    let recording =
        service::recording(&Ctx::from_state(&state, &tenant.scope).await?, &activity_id).await?;
    Ok(Json(Data::new(recording.into())))
}

#[utoipa::path(
    post,
    path = "/telephony/events",
    tag = TAG,
    operation_id = "calls_events",
    security(("session_cookie" = [])),
    request_body = CallEventRequest,
    responses(
        (status = 200, description = "Call event of a telephony module (`provider` is `<module>.<provider>`, enabled for the organization). The first event of a `provider_call_id` logs the call as the acting user (`call.logged`; without `outcome_key` the first active outcome that is connected when `duration_seconds` > 0, or not connected otherwise); the next ones update its details (`call.updated`). Modules send it with `report_call_event`", body = Data<CallActivityResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (or `tasks.update` for an existing call) (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or disabled telephony provider (`telephony_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn call_event(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CallEventRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = service::record_event(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        activity_json(&state, &tenant.scope, record).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/telephony/providers",
    tag = TAG,
    operation_id = "calls_providers",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Telephony providers of the modules enabled for the organization. Without any, the web app dials with `tel:`", body = Data<Vec<TelephonyProviderResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_providers(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<TelephonyProviderResponse>>>, ApiError> {
    let providers = service::providers(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        providers
            .into_iter()
            .map(|(key, label)| TelephonyProviderResponse { key, label })
            .collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/telephony/start",
    tag = TAG,
    operation_id = "calls_start",
    security(("session_cookie" = [])),
    request_body = StartCallRequest,
    responses(
        (status = 200, description = "Call started by the telephony provider (click to call). Nothing is logged yet: the provider reports the call with `POST /telephony/events` using the returned `provider_call_id`, or the user logs it with `POST /telephony`", body = Data<StartedCallResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or disabled telephony provider (`telephony_provider_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Missing phone number or unreadable record (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn start_call(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<StartCallRequest>,
) -> Result<Json<Data<StartedCallResponse>>, ApiError> {
    let started = service::start(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into(),
    )
    .await?;
    Ok(Json(Data::new(started.into())))
}

#[utoipa::path(
    get,
    path = "/telephony/report",
    tag = TAG,
    operation_id = "calls_report",
    security(("session_cookie" = [])),
    params(CallReportParams),
    responses(
        (status = 200, description = "Logged calls per owner and outcome in the period (`from` inclusive, `to` exclusive; dates `YYYY-MM-DD` in the organization time zone — `to` includes the whole day — or RFC 3339 instants; default the last 30 days, at most 366 days), only over the call tasks the user can read (`tasks.read` scope). `owner_id=me` keeps the user's calls. `connected` counts the outcomes marked as connected", body = Data<CallReportResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid period or owner (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn call_report(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<CallReportParams>,
) -> Result<Json<Data<CallReportResponse>>, ApiError> {
    let report = service::report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into(),
    )
    .await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    get,
    path = "/telephony/outcomes",
    tag = TAG,
    operation_id = "calls_outcomes_list",
    security(("session_cookie" = [])),
    params(CallOutcomeListParams),
    responses(
        (status = 200, description = "Call outcomes of the organization in their order (active only unless `include_inactive=true`). The system ones (`answered`, `voicemail`, `no_answer`, `wrong_number`) are created with the organization", body = Data<Vec<CallOutcomeResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` or `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_outcomes(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<CallOutcomeListParams>,
) -> Result<Json<Data<Vec<CallOutcomeResponse>>>, ApiError> {
    let outcomes = service::list_outcomes(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.include_inactive.unwrap_or(false),
    )
    .await?;
    Ok(Json(Data::new(
        outcomes.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/telephony/outcomes",
    tag = TAG,
    operation_id = "calls_outcomes_create",
    security(("session_cookie" = [])),
    request_body = CreateCallOutcomeRequest,
    responses(
        (status = 201, description = "Call outcome created at the end of the list (`call_outcome.created`); `key` defaults to one derived from the name. `is_connected` marks outcomes where the person was reached (counted as connected in the report)", body = Data<CallOutcomeResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another outcome uses the key (`call_outcome_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or key, or the limit of 50 outcomes was reached (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_outcome(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateCallOutcomeRequest>,
) -> Result<(StatusCode, Json<Data<CallOutcomeResponse>>), ApiError> {
    let outcome = service::create_outcome(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(outcome.into()))))
}

#[utoipa::path(
    patch,
    path = "/telephony/outcomes/{outcome_id}",
    tag = TAG,
    operation_id = "calls_outcomes_update",
    security(("session_cookie" = [])),
    params(
        ("outcome_id" = String, Path, description = "Call outcome id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the outcome as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateCallOutcomeRequest,
    responses(
        (status = 200, description = "Call outcome updated (`call_outcome.updated`); the key never changes. Inactive outcomes stay on the calls already logged but cannot be picked for new ones", body = Data<CallOutcomeResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Outcome not found (`call_outcome_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Deactivating the last active outcome (`last_active_call_outcome`) or the outcome changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_outcome(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(outcome_id): Path<String>,
    JsonBody(request): JsonBody<UpdateCallOutcomeRequest>,
) -> Result<Json<Data<CallOutcomeResponse>>, ApiError> {
    let outcome = service::update_outcome(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &outcome_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(outcome.into())))
}

#[utoipa::path(
    put,
    path = "/telephony/outcomes/order",
    tag = TAG,
    operation_id = "calls_outcomes_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderCallOutcomesRequest,
    responses(
        (status = 200, description = "Outcomes in the new order (`call_outcome.reordered`); `ids` lists every outcome, active or not, exactly once", body = Data<Vec<CallOutcomeResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The list does not match the outcomes of the organization (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_outcomes(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderCallOutcomesRequest>,
) -> Result<Json<Data<Vec<CallOutcomeResponse>>>, ApiError> {
    let outcomes = service::reorder_outcomes(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request.ids,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        outcomes.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/telephony/sessions",
    tag = TAG,
    operation_id = "calls_sessions_list",
    security(("session_cookie" = [])),
    params(CallSessionListParams, PaginationQuery),
    responses(
        (status = 200, description = "Call sessions of the user, newest first (`active=true` keeps the ones still in progress)", body = Page<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_sessions(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    Query(params): Query<CallSessionListParams>,
) -> Result<Json<Page<CallSessionResponse>>, ApiError> {
    let listing = service::list_sessions(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.active.unwrap_or(false),
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/telephony/sessions",
    tag = TAG,
    operation_id = "calls_sessions_start",
    security(("session_cookie" = [])),
    request_body = StartCallSessionRequest,
    responses(
        (status = 201, description = "Call session started from a saved view of people visible to the user (`call_session.started`): the people matching the view filter and sort under the user's read scope (at most 500) become the queue, kept for the session. `script` defaults to the `call_script` text in the view settings; `name` to the view name", body = Data<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` or `people.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "View not found, not visible or not of people (`call_session_view_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The view has no people (`call_session_empty`) or invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn start_session(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<StartCallSessionRequest>,
) -> Result<(StatusCode, Json<Data<CallSessionResponse>>), ApiError> {
    let session = service::start_session(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(session.into()))))
}

#[utoipa::path(
    get,
    path = "/telephony/sessions/{session_id}",
    tag = TAG,
    operation_id = "calls_sessions_get",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id")),
    responses(
        (status = 200, description = "Session with counts and the `current` contact of the queue (null once completed)", body = Data<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found or of another user (`call_session_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_session(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(session_id): Path<String>,
) -> Result<Json<Data<CallSessionResponse>>, ApiError> {
    let session =
        service::get_session(&Ctx::from_state(&state, &tenant.scope).await?, &session_id).await?;
    Ok(Json(Data::new(session.into())))
}

#[utoipa::path(
    get,
    path = "/telephony/sessions/{session_id}/items",
    tag = TAG,
    operation_id = "calls_sessions_items",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id"), PaginationQuery),
    responses(
        (status = 200, description = "Queue of the session in order, with the status of each contact (`pending`, `logged`, `skipped`) and the last call logged for it", body = Page<CallSessionItemResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found or of another user (`call_session_not_found`)", body = ErrorEnvelope),
    )
)]
async fn session_items(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    Path(session_id): Path<String>,
) -> Result<Json<Page<CallSessionItemResponse>>, ApiError> {
    let listing = service::session_items(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &session_id,
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/telephony/sessions/{session_id}/calls",
    tag = TAG,
    operation_id = "calls_sessions_log",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id")),
    request_body = SessionCallRequest,
    responses(
        (status = 201, description = "Call logged for the contact at `position` (default the current one), in the same transaction that marks it as `logged` and moves the session to the next pending contact (after it, then from the start). When no pending contact remains the session is completed (`call_session.completed`)", body = Data<SessionCallResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found (`call_session_not_found`) or no contact at the position (`call_session_item_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The session is completed (`call_session_completed`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, unknown outcome or the contact is no longer readable (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn session_call(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(session_id): Path<String>,
    JsonBody(request): JsonBody<SessionCallRequest>,
) -> Result<(StatusCode, Json<Data<SessionCallResponse>>), ApiError> {
    let (record, session) = service::log_in_session(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &session_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(SessionCallResponse {
            activity: activity_json(&state, &tenant.scope, record).await?,
            session: session.into(),
        })),
    ))
}

#[utoipa::path(
    post,
    path = "/telephony/sessions/{session_id}/skip",
    tag = TAG,
    operation_id = "calls_sessions_skip",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id")),
    request_body = SessionPositionRequest,
    responses(
        (status = 200, description = "Contact at `position` (default the current one) marked as `skipped` when still pending, and the session moves to the next pending contact; completed when none remains", body = Data<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found (`call_session_not_found`) or no contact at the position (`call_session_item_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The session is completed (`call_session_completed`)", body = ErrorEnvelope),
    )
)]
async fn session_skip(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(session_id): Path<String>,
    JsonBody(request): JsonBody<SessionPositionRequest>,
) -> Result<Json<Data<CallSessionResponse>>, ApiError> {
    let session = service::skip_in_session(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &session_id,
        request.position,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(session.into())))
}

#[utoipa::path(
    post,
    path = "/telephony/sessions/{session_id}/cursor",
    tag = TAG,
    operation_id = "calls_sessions_cursor",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id")),
    request_body = SessionCursorRequest,
    responses(
        (status = 200, description = "Session moved to the contact at `position` (go back or jump ahead); a contact already logged can be called again", body = Data<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found (`call_session_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The session is completed (`call_session_completed`)", body = ErrorEnvelope),
        (status = 422, description = "Position outside the queue (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn session_cursor(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(session_id): Path<String>,
    JsonBody(request): JsonBody<SessionCursorRequest>,
) -> Result<Json<Data<CallSessionResponse>>, ApiError> {
    let session = service::move_session(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &session_id,
        request.position,
    )
    .await?;
    Ok(Json(Data::new(session.into())))
}

#[utoipa::path(
    post,
    path = "/telephony/sessions/{session_id}/finish",
    tag = TAG,
    operation_id = "calls_sessions_finish",
    security(("session_cookie" = [])),
    params(("session_id" = String, Path, description = "Call session id")),
    responses(
        (status = 200, description = "Session completed before the end of the queue (`call_session.completed`)", body = Data<CallSessionResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `tasks.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Session not found (`call_session_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The session is already completed (`call_session_completed`)", body = ErrorEnvelope),
    )
)]
async fn session_finish(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(session_id): Path<String>,
) -> Result<Json<Data<CallSessionResponse>>, ApiError> {
    let session = service::finish_session(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &session_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(session.into())))
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = CallSessionStatus)]
pub(crate) enum CallSessionStatusSchema {
    Active,
    Completed,
}

impl From<CallSessionStatus> for CallSessionStatusSchema {
    fn from(value: CallSessionStatus) -> Self {
        match value {
            CallSessionStatus::Active => Self::Active,
            CallSessionStatus::Completed => Self::Completed,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = CallSessionItemStatus)]
pub(crate) enum CallSessionItemStatusSchema {
    Pending,
    Logged,
    Skipped,
}

impl From<SessionItemStatus> for CallSessionItemStatusSchema {
    fn from(value: SessionItemStatus) -> Self {
        match value {
            SessionItemStatus::Pending => Self::Pending,
            SessionItemStatus::Logged => Self::Logged,
            SessionItemStatus::Skipped => Self::Skipped,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallOutcomeResponse {
    pub(crate) id: String,
    #[schema(example = "answered")]
    pub(crate) key: String,
    #[schema(example = "Answered")]
    pub(crate) name: String,
    pub(crate) is_connected: bool,
    pub(crate) is_system: bool,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<CallOutcome> for CallOutcomeResponse {
    fn from(outcome: CallOutcome) -> Self {
        Self {
            id: outcome.id.to_string(),
            key: outcome.key,
            name: outcome.name,
            is_connected: outcome.is_connected,
            is_system: outcome.is_system,
            is_active: outcome.is_active,
            position: outcome.position,
            created_at: instant(outcome.created_at),
            updated_at: instant(outcome.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CallOutcomeListParams {
    pub(crate) include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateCallOutcomeRequest {
    #[schema(example = "callback_requested")]
    pub(crate) key: Option<String>,
    #[schema(example = "Requested callback")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) is_connected: bool,
}

impl CreateCallOutcomeRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateCallOutcomeCommand {
        CreateCallOutcomeCommand {
            key: self.key,
            name: self.name,
            is_connected: self.is_connected,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateCallOutcomeRequest {
    pub(crate) name: Option<String>,
    pub(crate) is_connected: Option<bool>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateCallOutcomeRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> UpdateCallOutcomeCommand {
        UpdateCallOutcomeCommand {
            name: self.name,
            is_connected: self.is_connected,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReorderCallOutcomesRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogCallRequest {
    #[schema(example = "people.person")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) direction: Option<CallDirectionSchema>,
    #[schema(example = "answered")]
    pub(crate) outcome_key: String,
    #[schema(example = 95)]
    pub(crate) duration_seconds: Option<i64>,
    #[schema(example = "+55 11 99999-0000")]
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) occurred_at: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
}

impl LogCallRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> LogCallCommand {
        LogCallCommand {
            record_type: self.record_type,
            record_id: self.record_id,
            direction: self.direction.map(Into::into),
            outcome_key: Some(self.outcome_key),
            duration_seconds: self.duration_seconds,
            phone_number: self.phone_number,
            recording_url: self.recording_url,
            title: self.title,
            notes: self.notes,
            occurred_at: self.occurred_at,
            provider: self.provider,
            provider_call_id: self.provider_call_id,
            custom_fields: self.custom_fields,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateCallRequest {
    pub(crate) direction: Option<CallDirectionSchema>,
    pub(crate) outcome_key: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>)]
    pub(crate) duration_seconds: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) phone_number: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) recording_url: Option<Option<String>>,
}

impl UpdateCallRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> UpdateCallCommand {
        UpdateCallCommand {
            direction: self.direction.map(Into::into),
            outcome_key: self.outcome_key,
            duration_seconds: self.duration_seconds,
            phone_number: self.phone_number,
            recording_url: self.recording_url,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CallEventRequest {
    #[schema(example = "voip.twilio")]
    pub(crate) provider: String,
    pub(crate) provider_call_id: String,
    pub(crate) direction: Option<CallDirectionSchema>,
    pub(crate) phone_number: Option<String>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) recording_url: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) occurred_at: Option<String>,
}

impl CallEventRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CallEventCommand {
        CallEventCommand {
            provider: self.provider,
            provider_call_id: self.provider_call_id,
            direction: self.direction.map(Into::into),
            phone_number: self.phone_number,
            outcome_key: self.outcome_key,
            duration_seconds: self.duration_seconds,
            recording_url: self.recording_url,
            record_type: self.record_type,
            record_id: self.record_id,
            occurred_at: self.occurred_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TelephonyProviderResponse {
    #[schema(example = "voip.twilio")]
    pub(crate) key: String,
    pub(crate) label: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartCallRequest {
    pub(crate) provider: String,
    pub(crate) phone_number: String,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}

impl From<StartCallRequest> for StartCallCommand {
    fn from(request: StartCallRequest) -> Self {
        Self {
            provider: request.provider,
            phone_number: request.phone_number,
            record_type: request.record_type,
            record_id: request.record_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct StartedCallResponse {
    pub(crate) provider: String,
    pub(crate) provider_call_id: String,
}

impl From<StartedCallView> for StartedCallResponse {
    fn from(view: StartedCallView) -> Self {
        Self {
            provider: view.provider,
            provider_call_id: view.provider_call_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallRecordingResponse {
    pub(crate) url: String,
    pub(crate) expires_at: Option<String>,
}

impl From<CallRecordingView> for CallRecordingResponse {
    fn from(view: CallRecordingView) -> Self {
        Self {
            url: view.url,
            expires_at: view.expires_at.map(instant),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CallReportParams {
    #[param(example = "2026-10-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-10-31")]
    pub(crate) to: Option<String>,
    #[param(example = "me")]
    pub(crate) owner_id: Option<String>,
}

impl From<CallReportParams> for CallReportQuery {
    fn from(params: CallReportParams) -> Self {
        Self {
            from: params.from,
            to: params.to,
            owner_id: params.owner_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallReportOutcomeCountResponse {
    pub(crate) outcome_key: String,
    pub(crate) calls: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallReportUserResponse {
    pub(crate) user: Option<UserReferenceResponse>,
    pub(crate) calls: u64,
    pub(crate) connected: u64,
    pub(crate) duration_seconds: u64,
    pub(crate) outcomes: Vec<CallReportOutcomeCountResponse>,
}

impl From<CallReportUser> for CallReportUserResponse {
    fn from(user: CallReportUser) -> Self {
        Self {
            user: user.user.map(Into::into),
            calls: user.calls,
            connected: user.connected,
            duration_seconds: user.duration_seconds,
            outcomes: user
                .outcomes
                .into_iter()
                .map(|count| CallReportOutcomeCountResponse {
                    outcome_key: count.outcome_key,
                    calls: count.calls,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallReportResponse {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) calls: u64,
    pub(crate) connected: u64,
    pub(crate) duration_seconds: u64,
    pub(crate) outcomes: Vec<CallOutcomeResponse>,
    pub(crate) users: Vec<CallReportUserResponse>,
}

impl From<CallReport> for CallReportResponse {
    fn from(report: CallReport) -> Self {
        Self {
            from: instant(report.from),
            to: instant(report.to),
            calls: report.calls,
            connected: report.connected,
            duration_seconds: report.duration_seconds,
            outcomes: report.outcomes.into_iter().map(Into::into).collect(),
            users: report.users.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartCallSessionRequest {
    pub(crate) view_id: String,
    pub(crate) name: Option<String>,
    pub(crate) script: Option<String>,
}

impl StartCallSessionRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> StartCallSessionCommand {
        StartCallSessionCommand {
            view_id: self.view_id,
            name: self.name,
            script: self.script,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CallSessionListParams {
    pub(crate) active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallSessionItemResponse {
    pub(crate) position: u32,
    pub(crate) contact_id: String,
    pub(crate) contact_name: Option<String>,
    pub(crate) status: CallSessionItemStatusSchema,
    pub(crate) activity_id: Option<String>,
    pub(crate) outcome_key: Option<String>,
    pub(crate) handled_at: Option<String>,
}

impl From<CallSessionItem> for CallSessionItemResponse {
    fn from(item: CallSessionItem) -> Self {
        Self {
            position: item.position,
            contact_id: item.contact_id.to_string(),
            contact_name: item.contact_name,
            status: item.status.into(),
            activity_id: item.activity_id.map(|id| id.to_string()),
            outcome_key: item.outcome_key,
            handled_at: item.handled_at.map(instant),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallSessionCountsResponse {
    pub(crate) pending: u32,
    pub(crate) logged: u32,
    pub(crate) skipped: u32,
}

impl From<SessionCounts> for CallSessionCountsResponse {
    fn from(counts: SessionCounts) -> Self {
        Self {
            pending: counts.pending,
            logged: counts.logged,
            skipped: counts.skipped,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CallSessionResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) view_id: Option<String>,
    pub(crate) script: Option<String>,
    pub(crate) status: CallSessionStatusSchema,
    pub(crate) position: u32,
    pub(crate) total: u32,
    pub(crate) counts: Option<CallSessionCountsResponse>,
    pub(crate) current: Option<CallSessionItemResponse>,
    pub(crate) completed_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<CallSession> for CallSessionResponse {
    fn from(session: CallSession) -> Self {
        Self {
            id: session.id.to_string(),
            name: session.name,
            view_id: session.view_id.map(|id| id.to_string()),
            script: session.script,
            status: session.status.into(),
            position: session.position,
            total: session.total,
            counts: None,
            current: None,
            completed_at: session.completed_at.map(instant),
            created_at: instant(session.created_at),
            updated_at: instant(session.updated_at),
        }
    }
}

impl From<CallSessionView> for CallSessionResponse {
    fn from(view: CallSessionView) -> Self {
        let mut response = Self::from(view.session);
        response.counts = Some(view.counts.into());
        response.current = view.current.map(Into::into);
        response
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionCallRequest {
    pub(crate) position: Option<u32>,
    pub(crate) direction: Option<CallDirectionSchema>,
    #[schema(example = "answered")]
    pub(crate) outcome_key: String,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) phone_number: Option<String>,
    pub(crate) recording_url: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) provider: Option<String>,
    pub(crate) provider_call_id: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
}

impl SessionCallRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SessionCallCommand {
        SessionCallCommand {
            position: self.position,
            direction: self.direction.map(Into::into),
            outcome_key: Some(self.outcome_key),
            duration_seconds: self.duration_seconds,
            phone_number: self.phone_number,
            recording_url: self.recording_url,
            notes: self.notes,
            provider: self.provider,
            provider_call_id: self.provider_call_id,
            custom_fields: self.custom_fields,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionPositionRequest {
    pub(crate) position: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionCursorRequest {
    pub(crate) position: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SessionCallResponse {
    #[schema(value_type = ActivityResponse)]
    pub(crate) activity: Value,
    pub(crate) session: CallSessionResponse,
}
