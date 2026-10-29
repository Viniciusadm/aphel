use async_trait::async_trait;
use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::domain::{
    ActivityLocation, ActivityLocationInput, ActivityPriority, ActivityRecord,
    ActivityScheduleInput, ActivityStatus, ActivityTypeSummary, ActivityTypeView, CalendarQuery,
    CreateActivityCommand, CreateActivityTypeCommand, InboxGroup, InboxGroupKey, InboxListing,
    InboxQuery, LinkedDealValue, LinkedRecord, ListActivitiesQuery, RecordLinkInput,
    UpdateActivityCommand, UpdateActivityTypeCommand,
};
use crate::recurrence::{
    ActivityRecurrenceView, ApplyTo, RecurrenceFrequency, RecurrenceRuleInput,
};
use crate::{service, types};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{
    Data, ErrorEnvelope, Page, double_option, timestamp as instant,
};
use aphel_kernel::api::dto::files::UserReferenceResponse;
use aphel_kernel::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::field_permissions::{redacted, redacted_items};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, RecordTypeHandler, optional_record, values_request};
use chrono::Weekday;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_activities, create_activity))
        .routes(routes!(activity_inbox))
        .routes(routes!(get_activity, update_activity, delete_activity))
        .routes(routes!(complete_activity))
        .routes(routes!(reopen_activity))
        .routes(routes!(list_activity_types, create_activity_type))
        .routes(routes!(reorder_activity_types))
        .routes(routes!(get_activity_type, update_activity_type))
        .routes(routes!(activate_activity_type))
        .routes(routes!(deactivate_activity_type))
        .routes(routes!(get_calendar))
}

pub(crate) fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub(crate) const TAG: &str = "activities";

fn query_rejection(rejection: QueryRejection) -> AppError {
    AppError::Validation(rejection.body_text())
}

#[utoipa::path(
    get,
    path = "/activities",
    tag = TAG,
    operation_id = "activities_list",
    security(("session_cookie" = [])),
    params(ActivityListParams, RecordQueryParams, PaginationQuery),
    responses(
        (status = 200, description = "Activities visible to the user. `record_type` + `record_id` keep the activities of a record (main link or additional links); `my_open=true` keeps the open activities owned by the user, sorted by due date unless `sort` is sent (otherwise newest first). Besides `filter` as JSON, simple conditions go as `filter[field][operator]=value` (`filter[status][eq]=open`, `filter[due_at][lt]=2026-01-01`), combined with `and`. `activity_type`, `owner`, `record` and `links` are always embedded", body = Page<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` or read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, record reference or pagination (`validation_error` with `details.fields`); unknown record type (`unsupported_record_type`)", body = ErrorEnvelope),
    )
)]
async fn list_activities(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    params: Result<Query<ActivityListParams>, QueryRejection>,
    RecordListQuery(query): RecordListQuery,
) -> Result<Json<Page<Value>>, ApiError> {
    let Query(params) = params.map_err(query_rejection)?;
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        ListActivitiesQuery {
            record_type: params.record_type,
            record_id: params.record_id,
            my_open: params.my_open.unwrap_or(false),
            query,
        },
    )
    .await?;
    let items: Vec<ActivityResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &RecordType::ACTIVITY, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/activities",
    tag = TAG,
    operation_id = "activities_create",
    security(("session_cookie" = [])),
    request_body = CreateActivityRequest,
    responses(
        (status = 201, description = "Activity created. Without `activity_type_id` the active `task` type (or the first active type) is used. Without `owner_id` the user is the owner; `status=done` logs an activity that already happened (`completed_at` defaults to now). `reminder_minutes_before` notifies the owner that many minutes before `due_at` (omitted: 60; `null`: no reminder). `recurrence` turns an open activity with `due_at` into the first occurrence of a series (daily, weekly on `by_weekday`, monthly on `month_day` — the last day of shorter months —, yearly; `interval`, and `until` or `count`), computed in the organization time zone; completing an occurrence creates the next one. `location` (`latitude` −90..90, `longitude` −180..180, optional `accuracy_m` in meters and `captured_at`, default now, never in the future) records where the activity happened, as in a visit check-in; it is stored with 6 decimal places and answered with a `map_url` (also flattened as `location_*`). Enabled modules may add keys to the answer, such as `call` from the calls module", body = Data<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields, inactive type, unknown owner, unreadable linked record or invalid custom fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateActivityRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let (command, schedule) = request.into_parts(client.0.ip);
    let activity = service::create_with(
        &Ctx::from_state(&state, &tenant.scope).await?,
        command,
        schedule,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted(
                &state,
                &tenant.scope,
                &RecordType::ACTIVITY,
                ActivityResponse::from(activity),
            )
            .await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/activities/{activity_id}",
    tag = TAG,
    operation_id = "activities_get",
    security(("session_cookie" = [])),
    params(("activity_id" = String, Path, description = "Activity id")),
    responses(
        (status = 200, description = "The activity with its custom fields", body = Data<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found or outside the user's scope (`activity_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(activity_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let activity =
        service::get(&Ctx::from_state(&state, &tenant.scope).await?, &activity_id).await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/activities/{activity_id}",
    tag = TAG,
    operation_id = "activities_update",
    security(("session_cookie" = [])),
    params(
        ("activity_id" = String, Path, description = "Activity id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the activity as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateActivityRequest,
    responses(
        (status = 200, description = "Activity updated; only the sent fields change. Changing `due_at` or `reminder_minutes_before`, or reopening, re-arms the reminder. On an occurrence of a recurring series, `apply_to=this` (default) changes only this occurrence and `apply_to=following` also updates the series template (title, type, owner, priority, description, duration, reminder, links and, when `due_at` changes, the anchor of the schedule) used by the next occurrences. `recurrence` replaces the rule from this occurrence on (or starts a series); `recurrence: null` ends it. Completing an occurrence (`status=done`) creates the next one once. `location` replaces the check-in location; `location: null` removes it", body = Data<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.update` for this activity (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The activity changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or custom fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(activity_id): Path<String>,
    JsonBody(request): JsonBody<UpdateActivityRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let activity = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/activities/{activity_id}",
    tag = TAG,
    operation_id = "activities_delete",
    security(("session_cookie" = [])),
    params(("activity_id" = String, Path, description = "Activity id")),
    responses(
        (status = 204, description = "Activity deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.delete` for this activity (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(activity_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/activities/{activity_id}/complete",
    tag = TAG,
    operation_id = "activities_complete",
    security(("session_cookie" = [])),
    params(("activity_id" = String, Path, description = "Activity id")),
    responses(
        (status = 200, description = "Activity marked as done now", body = Data<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.update` for this activity (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The activity is already done (`activity_already_completed`)", body = ErrorEnvelope),
    )
)]
async fn complete_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(activity_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let activity = service::complete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/activities/{activity_id}/reopen",
    tag = TAG,
    operation_id = "activities_reopen",
    security(("session_cookie" = [])),
    params(("activity_id" = String, Path, description = "Activity id")),
    responses(
        (status = 200, description = "Activity open again", body = Data<ActivityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.update` for this activity (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity not found (`activity_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The activity is already open (`activity_already_open`)", body = ErrorEnvelope),
    )
)]
async fn reopen_activity(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(activity_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let activity = service::reopen(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await?,
    )))
}

#[utoipa::path(
    get,
    path = "/activities/inbox",
    tag = TAG,
    operation_id = "activities_inbox",
    security(("session_cookie" = [])),
    params(InboxParams),
    responses(
        (status = 200, description = "Open activities in four groups by due date in the organization time zone (`overdue`, `today`, `upcoming`, `no_due`), at most 500. `owner=me` (default) keeps the user's activities, `owner=team` the ones owned by members of `team_id` (or of the user's teams) and `owner=all` everything the user can read. `sort=due` (default), `priority` (urgent first) or `value` (amount of the linked deal, largest first), always followed by due date. `linked_deal` is the main or first linked deal the user can read (needs `deals.read`); its `amount` is `null` when the field is hidden from the user (`deal_amounts_visible=false`)", body = Data<InboxResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `owner` or `sort` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn activity_inbox(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<InboxParams>, QueryRejection>,
) -> Result<Json<Data<InboxResponse>>, ApiError> {
    let Query(params) = params.map_err(query_rejection)?;
    let mut listing = service::inbox(
        &Ctx::from_state(&state, &tenant.scope).await?,
        InboxQuery {
            owner: params.owner,
            team_id: params.team_id,
            sort: params.sort,
        },
    )
    .await?;
    let mut groups = Vec::new();
    for group in std::mem::take(&mut listing.groups) {
        let (key, items) = inbox_group_items(group);
        groups.push(InboxGroupResponse {
            key,
            items: redacted_items(&state, &tenant.scope, &RecordType::ACTIVITY, items).await?,
        });
    }
    Ok(Json(Data::new(InboxResponse::new(&listing, groups))))
}

pub(crate) const TYPES_TAG: &str = "activity_types";

#[utoipa::path(
    get,
    path = "/activity-types",
    tag = TYPES_TAG,
    operation_id = "activity_types_list",
    security(("session_cookie" = [])),
    params(ActivityTypeListParams, PaginationQuery),
    responses(
        (status = 200, description = "Activity types ordered by position; inactive ones only with `include_inactive=true`", body = Page<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `include_inactive` or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_activity_types(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<ActivityTypeListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<ActivityTypeResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = types::search(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.include_inactive.unwrap_or(false),
        params.search.as_deref(),
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
    get,
    path = "/activity-types/{activity_type_id}",
    tag = TYPES_TAG,
    operation_id = "activity_types_get",
    security(("session_cookie" = [])),
    params(("activity_type_id" = String, Path, description = "Activity type id")),
    responses(
        (status = 200, description = "The activity type", body = Data<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` and `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity type not found (`activity_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_activity_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(activity_type_id): Path<String>,
) -> Result<Json<Data<ActivityTypeResponse>>, ApiError> {
    let view = types::get(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_type_id,
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/activity-types",
    tag = TYPES_TAG,
    operation_id = "activity_types_create",
    security(("session_cookie" = [])),
    request_body = CreateActivityTypeRequest,
    responses(
        (status = 201, description = "Activity type created at the end of the list; the key is derived from the name when omitted", body = Data<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`activity_type_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, key, icon or color (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_activity_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateActivityTypeRequest>,
) -> Result<(StatusCode, Json<Data<ActivityTypeResponse>>), ApiError> {
    let view = types::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(view.into()))))
}

#[utoipa::path(
    patch,
    path = "/activity-types/{activity_type_id}",
    tag = TYPES_TAG,
    operation_id = "activity_types_update",
    security(("session_cookie" = [])),
    params(
        ("activity_type_id" = String, Path, description = "Activity type id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the activity type as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateActivityTypeRequest,
    responses(
        (status = 200, description = "Activity type renamed or restyled; the key never changes", body = Data<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity type not found (`activity_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The activity type changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, icon or color (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_activity_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected_updated_at): IfMatch,
    Path(activity_type_id): Path<String>,
    JsonBody(request): JsonBody<UpdateActivityTypeRequest>,
) -> Result<Json<Data<ActivityTypeResponse>>, ApiError> {
    let view = types::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_type_id,
        request.into_command(client.0.ip, expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/activity-types/{activity_type_id}/activate",
    tag = TYPES_TAG,
    operation_id = "activity_types_activate",
    security(("session_cookie" = [])),
    params(("activity_type_id" = String, Path, description = "Activity type id")),
    responses(
        (status = 200, description = "Activity type active again", body = Data<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity type not found (`activity_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn activate_activity_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(activity_type_id): Path<String>,
) -> Result<Json<Data<ActivityTypeResponse>>, ApiError> {
    let view = types::set_active(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_type_id,
        true,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/activity-types/{activity_type_id}/deactivate",
    tag = TYPES_TAG,
    operation_id = "activity_types_deactivate",
    security(("session_cookie" = [])),
    params(("activity_type_id" = String, Path, description = "Activity type id")),
    responses(
        (status = 200, description = "Activity type hidden from new activities; existing activities keep it", body = Data<ActivityTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Activity type not found (`activity_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "It is the last active type (`last_active_activity_type`)", body = ErrorEnvelope),
    )
)]
async fn deactivate_activity_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(activity_type_id): Path<String>,
) -> Result<Json<Data<ActivityTypeResponse>>, ApiError> {
    let view = types::set_active(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &activity_type_id,
        false,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/activity-types/order",
    tag = TYPES_TAG,
    operation_id = "activity_types_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderActivityTypesRequest,
    responses(
        (status = 200, description = "Every activity type, active or not, in the new order", body = Data<Vec<ActivityTypeResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The ids do not list every activity type exactly once (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_activity_types(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderActivityTypesRequest>,
) -> Result<Json<Data<Vec<ActivityTypeResponse>>>, ApiError> {
    let types = types::reorder(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &request.ids,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(types.into_iter().map(Into::into).collect())))
}

pub(crate) const CALENDAR_TAG: &str = "calendar";

#[utoipa::path(
    get,
    path = "/calendar",
    tag = CALENDAR_TAG,
    operation_id = "calendar_get",
    security(("session_cookie" = [])),
    params(CalendarParams),
    responses(
        (status = 200, description = "Activities the user can read with `due_at` in `[from, to)` (at most 62 days), sorted by due date, at most 1000 (`truncated` when there are more). Optional filters: `owner_id`, `team_id` (owner is a member of the team), `activity_type_id` and `status`. Meetings carry `duration_minutes`. Field permissions apply: hidden fields are removed and filtering by a hidden field answers 422 `field_not_accessible`", body = Data<CalendarResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `activities.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Missing or invalid range, range longer than 62 days or invalid filter (`validation_error`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn get_calendar(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<CalendarParams>, QueryRejection>,
) -> Result<Json<Data<CalendarResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::calendar(
        &Ctx::from_state(&state, &tenant.scope).await?,
        CalendarQuery {
            from: params.from,
            to: params.to,
            owner_id: params.owner_id,
            team_id: params.team_id,
            activity_type_id: params.activity_type_id,
            status: params.status,
        },
    )
    .await?;
    let items: Vec<ActivityResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Data::new(CalendarResponse {
        from: instant(listing.from),
        to: instant(listing.to),
        timezone: listing.timezone,
        total: listing.total,
        truncated: listing.truncated,
        items: redacted_items(&state, &tenant.scope, &RecordType::ACTIVITY, items).await?,
    })))
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ActivityStatus)]
pub(crate) enum ActivityStatusSchema {
    Open,
    Done,
    Canceled,
}

impl From<ActivityStatus> for ActivityStatusSchema {
    fn from(value: ActivityStatus) -> Self {
        match value {
            ActivityStatus::Open => Self::Open,
            ActivityStatus::Done => Self::Done,
            ActivityStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<ActivityStatusSchema> for ActivityStatus {
    fn from(value: ActivityStatusSchema) -> Self {
        match value {
            ActivityStatusSchema::Open => Self::Open,
            ActivityStatusSchema::Done => Self::Done,
            ActivityStatusSchema::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ActivityPriority)]
pub(crate) enum ActivityPrioritySchema {
    Low,
    Normal,
    High,
    Urgent,
}

impl From<ActivityPriority> for ActivityPrioritySchema {
    fn from(value: ActivityPriority) -> Self {
        match value {
            ActivityPriority::Low => Self::Low,
            ActivityPriority::Normal => Self::Normal,
            ActivityPriority::High => Self::High,
            ActivityPriority::Urgent => Self::Urgent,
        }
    }
}

impl From<ActivityPrioritySchema> for ActivityPriority {
    fn from(value: ActivityPrioritySchema) -> Self {
        match value {
            ActivityPrioritySchema::Low => Self::Low,
            ActivityPrioritySchema::Normal => Self::Normal,
            ActivityPrioritySchema::High => Self::High,
            ActivityPrioritySchema::Urgent => Self::Urgent,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RecurrenceFrequency)]
pub(crate) enum RecurrenceFrequencySchema {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl From<RecurrenceFrequency> for RecurrenceFrequencySchema {
    fn from(value: RecurrenceFrequency) -> Self {
        match value {
            RecurrenceFrequency::Daily => Self::Daily,
            RecurrenceFrequency::Weekly => Self::Weekly,
            RecurrenceFrequency::Monthly => Self::Monthly,
            RecurrenceFrequency::Yearly => Self::Yearly,
        }
    }
}

impl From<RecurrenceFrequencySchema> for RecurrenceFrequency {
    fn from(value: RecurrenceFrequencySchema) -> Self {
        match value {
            RecurrenceFrequencySchema::Daily => Self::Daily,
            RecurrenceFrequencySchema::Weekly => Self::Weekly,
            RecurrenceFrequencySchema::Monthly => Self::Monthly,
            RecurrenceFrequencySchema::Yearly => Self::Yearly,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RecurrenceWeekday)]
pub(crate) enum RecurrenceWeekdaySchema {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

impl From<Weekday> for RecurrenceWeekdaySchema {
    fn from(value: Weekday) -> Self {
        match value {
            Weekday::Mon => Self::Mo,
            Weekday::Tue => Self::Tu,
            Weekday::Wed => Self::We,
            Weekday::Thu => Self::Th,
            Weekday::Fri => Self::Fr,
            Weekday::Sat => Self::Sa,
            Weekday::Sun => Self::Su,
        }
    }
}

impl From<RecurrenceWeekdaySchema> for Weekday {
    fn from(value: RecurrenceWeekdaySchema) -> Self {
        match value {
            RecurrenceWeekdaySchema::Mo => Self::Mon,
            RecurrenceWeekdaySchema::Tu => Self::Tue,
            RecurrenceWeekdaySchema::We => Self::Wed,
            RecurrenceWeekdaySchema::Th => Self::Thu,
            RecurrenceWeekdaySchema::Fr => Self::Fri,
            RecurrenceWeekdaySchema::Sa => Self::Sat,
            RecurrenceWeekdaySchema::Su => Self::Sun,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RecurrenceApplyTo)]
pub(crate) enum RecurrenceApplyToSchema {
    This,
    Following,
}

impl From<RecurrenceApplyToSchema> for ApplyTo {
    fn from(value: RecurrenceApplyToSchema) -> Self {
        match value {
            RecurrenceApplyToSchema::This => Self::This,
            RecurrenceApplyToSchema::Following => Self::Following,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecurrenceRuleRequest {
    pub(crate) frequency: RecurrenceFrequencySchema,
    #[schema(example = 1)]
    pub(crate) interval: Option<i64>,
    pub(crate) by_weekday: Option<Vec<RecurrenceWeekdaySchema>>,
    #[schema(example = 31)]
    pub(crate) month_day: Option<i64>,
    #[schema(example = "2026-12-31")]
    pub(crate) until: Option<String>,
    pub(crate) count: Option<i64>,
}

impl From<RecurrenceRuleRequest> for RecurrenceRuleInput {
    fn from(request: RecurrenceRuleRequest) -> Self {
        Self {
            frequency: request.frequency.into(),
            interval: request.interval,
            by_weekday: request
                .by_weekday
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            month_day: request.month_day,
            until: request.until,
            count: request.count,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActivityRecurrenceResponse {
    pub(crate) series_id: String,
    pub(crate) index: u32,
    pub(crate) frequency: RecurrenceFrequencySchema,
    pub(crate) interval: u32,
    pub(crate) by_weekday: Vec<RecurrenceWeekdaySchema>,
    pub(crate) month_day: Option<u32>,
    pub(crate) until: Option<String>,
    pub(crate) count: Option<u32>,
    #[schema(example = "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31")]
    pub(crate) rrule: String,
    pub(crate) ended: bool,
}

impl From<ActivityRecurrenceView> for ActivityRecurrenceResponse {
    fn from(view: ActivityRecurrenceView) -> Self {
        Self {
            series_id: view.series_id.to_string(),
            index: view.index,
            frequency: view.rule.frequency.into(),
            interval: view.rule.interval,
            by_weekday: view.rule.weekdays.iter().copied().map(Into::into).collect(),
            month_day: view.rule.month_day,
            until: view.rule.until.map(|date| date.to_string()),
            count: view.rule.count,
            rrule: view.rule.to_rrule(),
            ended: view.ended,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkedRecordResponse {
    #[schema(example = "deal")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) title: Option<String>,
}

impl From<LinkedRecord> for LinkedRecordResponse {
    fn from(linked: LinkedRecord) -> Self {
        Self {
            record_type: linked.record.record_type.as_str().to_owned(),
            record_id: linked.record.record_id.to_string(),
            title: linked.title,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActivityLocationResponse {
    #[schema(example = -23.55052)]
    pub(crate) latitude: f64,
    #[schema(example = -46.633308)]
    pub(crate) longitude: f64,
    #[schema(example = 12)]
    pub(crate) accuracy_m: Option<u32>,
    pub(crate) captured_at: String,
    #[schema(
        example = "https://www.openstreetmap.org/?mlat=-23.55052&mlon=-46.633308#map=17/-23.55052/-46.633308"
    )]
    pub(crate) map_url: String,
}

impl From<ActivityLocation> for ActivityLocationResponse {
    fn from(location: ActivityLocation) -> Self {
        Self {
            latitude: location.latitude.try_into().unwrap_or_default(),
            longitude: location.longitude.try_into().unwrap_or_default(),
            accuracy_m: location.accuracy_m,
            captured_at: instant(location.captured_at),
            map_url: location.map_url(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityLocationRequest {
    #[schema(example = -23.55052)]
    pub(crate) latitude: f64,
    #[schema(example = -46.633308)]
    pub(crate) longitude: f64,
    #[schema(example = 12.5)]
    pub(crate) accuracy_m: Option<f64>,
    #[schema(example = "2026-03-20T15:00:00Z")]
    pub(crate) captured_at: Option<String>,
}

impl From<ActivityLocationRequest> for ActivityLocationInput {
    fn from(request: ActivityLocationRequest) -> Self {
        Self {
            latitude: request.latitude,
            longitude: request.longitude,
            accuracy_m: request.accuracy_m,
            captured_at: request.captured_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityResponse {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) activity_type_id: String,
    pub(crate) status: ActivityStatusSchema,
    pub(crate) priority: ActivityPrioritySchema,
    pub(crate) due_at: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) duration_minutes: Option<u32>,
    pub(crate) description: Option<String>,
    pub(crate) location: Option<ActivityLocationResponse>,
    #[schema(example = -23.55052)]
    pub(crate) location_latitude: Option<f64>,
    #[schema(example = -46.633308)]
    pub(crate) location_longitude: Option<f64>,
    #[schema(example = 12)]
    pub(crate) location_accuracy_m: Option<u32>,
    pub(crate) location_captured_at: Option<String>,
    pub(crate) reminder_minutes_before: Option<u32>,
    pub(crate) recurrence: Option<ActivityRecurrenceResponse>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) activity_type: ActivityTypeSummaryResponse,
    pub(crate) owner: Option<UserReferenceResponse>,
    pub(crate) record: Option<LinkedRecordResponse>,
    pub(crate) links: Vec<LinkedRecordResponse>,
}

impl From<ActivityRecord> for ActivityResponse {
    fn from(record: ActivityRecord) -> Self {
        let activity = record.activity;
        let location: Option<ActivityLocationResponse> = activity.location.map(Into::into);
        Self {
            id: activity.id.to_string(),
            title: activity.title,
            activity_type_id: activity.activity_type.id.to_string(),
            status: activity.status.into(),
            priority: activity.priority.into(),
            due_at: activity.due_at.map(instant),
            completed_at: activity.completed_at.map(instant),
            owner_id: activity
                .owner
                .as_ref()
                .map(|owner| owner.id.as_str().to_owned()),
            created_by: activity
                .created_by
                .as_ref()
                .map(|user| user.id.as_str().to_owned()),
            record_type: activity
                .record
                .as_ref()
                .map(|linked| linked.record.record_type.as_str().to_owned()),
            record_id: activity
                .record
                .as_ref()
                .map(|linked| linked.record.record_id.to_string()),
            duration_minutes: activity.duration_minutes,
            description: activity.description,
            location_latitude: location.as_ref().map(|location| location.latitude),
            location_longitude: location.as_ref().map(|location| location.longitude),
            location_accuracy_m: location.as_ref().and_then(|location| location.accuracy_m),
            location_captured_at: location
                .as_ref()
                .map(|location| location.captured_at.clone()),
            location,
            reminder_minutes_before: activity.reminder_minutes_before,
            recurrence: activity.recurrence.map(Into::into),
            created_at: instant(activity.created_at),
            updated_at: instant(activity.updated_at),
            custom_fields: record.custom_fields,
            activity_type: activity.activity_type.into(),
            owner: activity.owner.map(Into::into),
            record: activity.record.map(Into::into),
            links: activity.links.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordLinkRequest {
    #[schema(example = "contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

impl From<RecordLinkRequest> for RecordLinkInput {
    fn from(request: RecordLinkRequest) -> Self {
        Self {
            record_type: request.record_type,
            record_id: request.record_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateActivityRequest {
    #[schema(example = "Call Ana")]
    pub(crate) title: String,
    pub(crate) activity_type_id: Option<String>,
    pub(crate) status: Option<ActivityStatusSchema>,
    pub(crate) priority: Option<ActivityPrioritySchema>,
    #[schema(example = "2026-03-20T15:00:00Z")]
    pub(crate) due_at: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) owner_id: Option<String>,
    #[schema(example = "contact")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) links: Option<Vec<RecordLinkRequest>>,
    pub(crate) duration_minutes: Option<i64>,
    pub(crate) description: Option<String>,
    pub(crate) location: Option<ActivityLocationRequest>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 15)]
    pub(crate) reminder_minutes_before: Option<Option<i64>>,
    pub(crate) recurrence: Option<RecurrenceRuleRequest>,
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
}

impl CreateActivityRequest {
    pub(crate) fn into_parts(
        self,
        ip: Option<String>,
    ) -> (CreateActivityCommand, ActivityScheduleInput) {
        let schedule = ActivityScheduleInput {
            reminder_minutes_before: self.reminder_minutes_before,
            recurrence: self.recurrence.map(Into::into),
        };
        let command = CreateActivityCommand {
            title: self.title,
            activity_type_id: self.activity_type_id.unwrap_or_default(),
            status: self
                .status
                .map(|status| ActivityStatus::from(status).as_str().to_owned()),
            priority: self
                .priority
                .map(|priority| ActivityPriority::from(priority).as_str().to_owned()),
            due_at: self.due_at,
            completed_at: self.completed_at,
            owner_id: self.owner_id,
            record_type: self.record_type,
            record_id: self.record_id,
            links: self
                .links
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            duration_minutes: self.duration_minutes,
            description: self.description,
            location: self.location.map(Into::into),
            custom_fields: self.custom_fields.unwrap_or_default(),
            ip,
        };
        (command, schedule)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateActivityRequest {
    pub(crate) title: Option<String>,
    pub(crate) activity_type_id: Option<String>,
    pub(crate) status: Option<ActivityStatusSchema>,
    pub(crate) priority: Option<ActivityPrioritySchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) due_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) completed_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) owner_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) record_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) record_id: Option<Option<String>>,
    pub(crate) links: Option<Vec<RecordLinkRequest>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>)]
    pub(crate) duration_minutes: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<ActivityLocationRequest>)]
    pub(crate) location: Option<Option<ActivityLocationRequest>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i64>, example = 15)]
    pub(crate) reminder_minutes_before: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<RecurrenceRuleRequest>)]
    pub(crate) recurrence: Option<Option<RecurrenceRuleRequest>>,
    pub(crate) apply_to: Option<RecurrenceApplyToSchema>,
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
}

impl UpdateActivityRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateActivityCommand {
        UpdateActivityCommand {
            title: self.title,
            activity_type_id: self.activity_type_id,
            status: self
                .status
                .map(|status| ActivityStatus::from(status).as_str().to_owned()),
            priority: self
                .priority
                .map(|priority| ActivityPriority::from(priority).as_str().to_owned()),
            due_at: self.due_at,
            completed_at: self.completed_at,
            owner_id: self.owner_id,
            record_type: self.record_type,
            record_id: self.record_id,
            links: self
                .links
                .map(|links| links.into_iter().map(Into::into).collect()),
            duration_minutes: self.duration_minutes,
            description: self.description,
            location: self.location.map(|location| location.map(Into::into)),
            custom_fields: self.custom_fields.unwrap_or_default(),
            reminder_minutes_before: self.reminder_minutes_before,
            recurrence: self.recurrence.map(|recurrence| recurrence.map(Into::into)),
            apply_to: self.apply_to.map(Into::into),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ActivityListParams {
    #[param(example = "contact")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) my_open: Option<bool>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct InboxParams {
    #[param(example = "me")]
    pub(crate) owner: Option<String>,
    pub(crate) team_id: Option<String>,
    #[param(example = "due")]
    pub(crate) sort: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LinkedDealValueResponse {
    pub(crate) deal_id: String,
    pub(crate) title: String,
    #[schema(example = "15000.00")]
    pub(crate) amount: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
}

impl From<LinkedDealValue> for LinkedDealValueResponse {
    fn from(value: LinkedDealValue) -> Self {
        Self {
            deal_id: value.deal_id.to_string(),
            title: value.title,
            amount: value.amount.map(money),
            currency: value.currency,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct InboxItemResponse {
    #[serde(flatten)]
    pub(crate) activity: ActivityResponse,
    pub(crate) linked_deal: Option<LinkedDealValueResponse>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = InboxGroupKey)]
pub(crate) enum InboxGroupKeySchema {
    Overdue,
    Today,
    Upcoming,
    NoDue,
}

impl From<InboxGroupKey> for InboxGroupKeySchema {
    fn from(value: InboxGroupKey) -> Self {
        match value {
            InboxGroupKey::Overdue => Self::Overdue,
            InboxGroupKey::Today => Self::Today,
            InboxGroupKey::Upcoming => Self::Upcoming,
            InboxGroupKey::NoDue => Self::NoDue,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct InboxGroupResponse {
    pub(crate) key: InboxGroupKeySchema,
    #[schema(value_type = Vec<InboxItemResponse>)]
    pub(crate) items: Vec<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct InboxResponse {
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) total: u64,
    pub(crate) truncated: bool,
    pub(crate) deal_amounts_visible: bool,
    pub(crate) groups: Vec<InboxGroupResponse>,
}

impl InboxResponse {
    pub(crate) fn new(listing: &InboxListing, groups: Vec<InboxGroupResponse>) -> Self {
        Self {
            timezone: listing.timezone.clone(),
            total: listing.total,
            truncated: listing.truncated,
            deal_amounts_visible: listing.deal_amounts_visible,
            groups,
        }
    }
}

pub(crate) fn inbox_group_items(
    group: InboxGroup,
) -> (InboxGroupKeySchema, Vec<InboxItemResponse>) {
    (
        group.key.into(),
        group
            .items
            .into_iter()
            .map(|item| InboxItemResponse {
                activity: item.record.into(),
                linked_deal: item.deal.map(Into::into),
            })
            .collect(),
    )
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActivityTypeResponse {
    pub(crate) id: String,
    #[schema(example = "call")]
    pub(crate) key: String,
    #[schema(example = "Call")]
    pub(crate) name: String,
    #[schema(example = "phone")]
    pub(crate) icon: Option<String>,
    #[schema(example = "#1f6feb")]
    pub(crate) color: Option<String>,
    pub(crate) is_system: bool,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<ActivityTypeView> for ActivityTypeResponse {
    fn from(view: ActivityTypeView) -> Self {
        Self {
            id: view.id.to_string(),
            key: view.key,
            name: view.name,
            icon: view.icon,
            color: view.color,
            is_system: view.is_system,
            is_active: view.is_active,
            position: view.position,
            created_at: instant(view.created_at),
            updated_at: instant(view.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ActivityTypeSummaryResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
    pub(crate) color: Option<String>,
}

impl From<ActivityTypeSummary> for ActivityTypeSummaryResponse {
    fn from(summary: ActivityTypeSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            key: summary.key,
            name: summary.name,
            icon: summary.icon,
            color: summary.color,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ActivityTypeListParams {
    pub(crate) include_inactive: Option<bool>,
    #[param(example = "follow")]
    pub(crate) search: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateActivityTypeRequest {
    #[schema(example = "follow_up")]
    pub(crate) key: Option<String>,
    #[schema(example = "Follow-up")]
    pub(crate) name: String,
    #[schema(example = "repeat")]
    pub(crate) icon: Option<String>,
    #[schema(example = "#8250df")]
    pub(crate) color: Option<String>,
}

impl CreateActivityTypeRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateActivityTypeCommand {
        CreateActivityTypeCommand {
            key: self.key,
            name: self.name,
            icon: self.icon,
            color: self.color,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateActivityTypeRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) icon: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) color: Option<Option<String>>,
}

impl UpdateActivityTypeRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateActivityTypeCommand {
        UpdateActivityTypeCommand {
            name: self.name,
            icon: self.icon,
            color: self.color,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReorderActivityTypesRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CalendarParams {
    #[param(example = "2026-10-01T03:00:00Z")]
    pub(crate) from: String,
    #[param(example = "2026-11-01T03:00:00Z")]
    pub(crate) to: String,
    pub(crate) owner_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) activity_type_id: Option<String>,
    #[param(example = "open")]
    pub(crate) status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CalendarResponse {
    pub(crate) from: String,
    pub(crate) to: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    pub(crate) total: u64,
    pub(crate) truncated: bool,
    #[schema(value_type = Vec<ActivityResponse>)]
    pub(crate) items: Vec<Value>,
}

pub(crate) struct ActivityHandler;

#[async_trait]
impl RecordTypeHandler for ActivityHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        "activity"
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let (command, schedule) = values_request::<CreateActivityRequest>(values)?.into_parts(None);
        let activity = service::create_with(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            command,
            schedule,
        )
        .await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
        )
        .await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let command = values_request::<UpdateActivityRequest>(values)?.into_command(None, None);
        let activity = service::update(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            command,
        )
        .await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::ACTIVITY,
            ActivityResponse::from(activity),
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
        let context = Ctx::from_state(ctx.state, ctx.scope).await?;
        let Some(activity) = optional_record(service::get(&context, &id.to_string()).await)? else {
            return Ok(None);
        };
        Ok(Some(
            redacted(
                ctx.state,
                ctx.scope,
                &RecordType::ACTIVITY,
                ActivityResponse::from(activity),
            )
            .await?,
        ))
    }
}
