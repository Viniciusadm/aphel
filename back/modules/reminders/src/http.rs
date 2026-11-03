use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    CreateReminderCommand, ListRemindersQuery, ReminderStatus, ReminderView, UpdateReminderCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{
    Data, ErrorEnvelope, Page, double_option, timestamp as instant,
};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, IfMatch, JsonBody, Pagination, PaginationQuery};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::errors::AppError;
use tasks::http::LinkedRecordResponse;

pub(crate) const TAG: &str = "reminders";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_reminders, create_reminder))
        .routes(routes!(get_reminder, update_reminder, delete_reminder))
}

#[utoipa::path(
    get,
    path = "/reminders",
    tag = TAG,
    operation_id = "reminders_list",
    security(("session_cookie" = [])),
    params(ReminderListParams, PaginationQuery),
    responses(
        (status = 200, description = "The user's own reminders (reminders are personal), sorted by `remind_at`. Filters: `status` (`pending`, `sent`, `dismissed`) and `record_type` + `record_id`", body = Page<ReminderResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_reminders(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    params: Result<Query<ReminderListParams>, QueryRejection>,
) -> Result<Json<Page<ReminderResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        ListRemindersQuery {
            status: params.status,
            record_type: params.record_type,
            record_id: params.record_id,
            window: ListWindow {
                offset: pagination.offset(),
                limit: pagination.limit(),
            },
        },
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
    path = "/reminders",
    tag = TAG,
    operation_id = "reminders_create",
    security(("session_cookie" = [])),
    request_body = CreateReminderRequest,
    responses(
        (status = 201, description = "Personal reminder created as `pending`; at `remind_at` the user gets a `reminder` notification (in-app and e-mail, following the notification preferences). A reminder is linked to a record (`record_type` + `record_id`, which the user must be able to read) or has a `note`", body = Data<ReminderResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or unreadable record (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_reminder(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateReminderRequest>,
) -> Result<(StatusCode, Json<Data<ReminderResponse>>), ApiError> {
    let reminder = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(reminder.into()))))
}

#[utoipa::path(
    get,
    path = "/reminders/{reminder_id}",
    tag = TAG,
    operation_id = "reminders_get",
    security(("session_cookie" = [])),
    params(("reminder_id" = String, Path, description = "Reminder id")),
    responses(
        (status = 200, description = "One of the user's reminders", body = Data<ReminderResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Reminder not found or owned by someone else (`reminder_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_reminder(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(reminder_id): Path<String>,
) -> Result<Json<Data<ReminderResponse>>, ApiError> {
    let reminder =
        service::get(&Ctx::from_state(&state, &tenant.scope).await?, &reminder_id).await?;
    Ok(Json(Data::new(reminder.into())))
}

#[utoipa::path(
    patch,
    path = "/reminders/{reminder_id}",
    tag = TAG,
    operation_id = "reminders_update",
    security(("session_cookie" = [])),
    params(
        ("reminder_id" = String, Path, description = "Reminder id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the reminder as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateReminderRequest,
    responses(
        (status = 200, description = "Reminder updated; only the sent fields change. A new `remind_at` or `status=pending` re-arms it; `status=dismissed` silences it", body = Data<ReminderResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Reminder not found (`reminder_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The reminder changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or unreadable record (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_reminder(
    State(state): State<AppState>,
    tenant: AuthContext,
    IfMatch(expected_updated_at): IfMatch,
    Path(reminder_id): Path<String>,
    JsonBody(request): JsonBody<UpdateReminderRequest>,
) -> Result<Json<Data<ReminderResponse>>, ApiError> {
    let reminder = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &reminder_id,
        request.into_command(expected_updated_at),
    )
    .await?;
    Ok(Json(Data::new(reminder.into())))
}

#[utoipa::path(
    delete,
    path = "/reminders/{reminder_id}",
    tag = TAG,
    operation_id = "reminders_delete",
    security(("session_cookie" = [])),
    params(("reminder_id" = String, Path, description = "Reminder id")),
    responses(
        (status = 204, description = "Reminder deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 404, description = "Reminder not found (`reminder_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_reminder(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(reminder_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(&Ctx::from_state(&state, &tenant.scope).await?, &reminder_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ReminderStatus)]
pub(crate) enum ReminderStatusSchema {
    Pending,
    Sent,
    Dismissed,
}

impl From<ReminderStatus> for ReminderStatusSchema {
    fn from(status: ReminderStatus) -> Self {
        match status {
            ReminderStatus::Pending => Self::Pending,
            ReminderStatus::Sent => Self::Sent,
            ReminderStatus::Dismissed => Self::Dismissed,
        }
    }
}

impl From<ReminderStatusSchema> for ReminderStatus {
    fn from(status: ReminderStatusSchema) -> Self {
        match status {
            ReminderStatusSchema::Pending => Self::Pending,
            ReminderStatusSchema::Sent => Self::Sent,
            ReminderStatusSchema::Dismissed => Self::Dismissed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReminderResponse {
    pub(crate) id: String,
    #[schema(example = "2026-10-22T13:00:00Z")]
    pub(crate) remind_at: String,
    pub(crate) note: Option<String>,
    pub(crate) status: ReminderStatusSchema,
    pub(crate) sent_at: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) record: Option<LinkedRecordResponse>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<ReminderView> for ReminderResponse {
    fn from(view: ReminderView) -> Self {
        Self {
            id: view.id.to_string(),
            remind_at: instant(view.remind_at),
            note: view.note,
            status: view.status.into(),
            sent_at: view.sent_at.map(instant),
            record_type: view
                .record
                .as_ref()
                .map(|linked| linked.record.record_type.as_str().to_owned()),
            record_id: view
                .record
                .as_ref()
                .map(|linked| linked.record.record_id.to_string()),
            record: view.record.map(Into::into),
            created_at: instant(view.created_at),
            updated_at: instant(view.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateReminderRequest {
    #[schema(example = "2026-10-22T13:00:00Z")]
    pub(crate) remind_at: String,
    #[schema(example = "Follow up on the proposal")]
    pub(crate) note: Option<String>,
    #[schema(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}

impl From<CreateReminderRequest> for CreateReminderCommand {
    fn from(request: CreateReminderRequest) -> Self {
        Self {
            remind_at: request.remind_at,
            note: request.note,
            record_type: request.record_type,
            record_id: request.record_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateReminderRequest {
    pub(crate) remind_at: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) note: Option<Option<String>>,
    pub(crate) status: Option<ReminderStatusSchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) record_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) record_id: Option<Option<String>>,
}

impl UpdateReminderRequest {
    pub(crate) fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateReminderCommand {
        UpdateReminderCommand {
            remind_at: self.remind_at,
            note: self.note,
            status: self.status.map(Into::into),
            record_type: self.record_type,
            record_id: self.record_id,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ReminderListParams {
    #[param(example = "pending")]
    pub(crate) status: Option<String>,
    #[param(example = "deals.deal")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}
