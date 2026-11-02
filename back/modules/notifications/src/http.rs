use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::application::NotificationUseCases;
use crate::dto::{
    MarkAllReadResponse, NotificationPageResponse, NotificationPreferencesResponse,
    NotificationsQueryParams, UnreadCountResponse, UpdateNotificationPreferencesRequest,
};
use crate::infrastructure::MySqlNotificationPorts;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub const TAG: &str = "notifications";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_notifications))
        .routes(routes!(unread_count))
        .routes(routes!(mark_all_read))
        .routes(routes!(mark_read))
        .routes(routes!(get_preferences, update_preferences))
}

fn use_cases(state: &AppState) -> Result<NotificationUseCases<MySqlNotificationPorts>, AppError> {
    Ok(NotificationUseCases::new(MySqlNotificationPorts::new(
        state.db()?,
        state.record_types.builtin(),
        state.jobs.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/notifications",
    tag = TAG,
    operation_id = "notifications_list",
    security(("session_cookie" = [])),
    params(NotificationsQueryParams),
    responses(
        (status = 200, description = "In-app notifications of the user, newest first, paginated by cursor: `{ data, meta: { next_cursor } }`; pass `meta.next_cursor` back as `cursor` for the next page (null on the last one). `unread=true` keeps only the unread ones; `per_page` from 1 to 100, default 25. The unread total comes from `GET /notifications/unread-count`", body = NotificationPageResponse),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `unread`, `cursor` or `per_page` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_notifications(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<NotificationsQueryParams>, QueryRejection>,
) -> Result<Json<NotificationPageResponse>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let per_page = params.per_page()?;
    let page = use_cases(&state)?
        .list(
            &tenant.scope,
            params.filter(),
            params.cursor.as_deref(),
            per_page,
        )
        .await?;
    Ok(Json(page.into()))
}

#[utoipa::path(
    get,
    path = "/notifications/unread-count",
    tag = TAG,
    operation_id = "notifications_unread_count",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Number of unread in-app notifications", body = Data<UnreadCountResponse>),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
    )
)]
async fn unread_count(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<UnreadCountResponse>>, ApiError> {
    let count = use_cases(&state)?.unread_count(&tenant.scope).await?;
    Ok(Json(Data::new(UnreadCountResponse { count })))
}

#[utoipa::path(
    post,
    path = "/notifications/read-all",
    tag = TAG,
    operation_id = "notifications_mark_all_read",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Every unread notification of the user was marked as read (personal state: no event or audit entry)", body = Data<MarkAllReadResponse>),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `X-Requested-With` (`csrf_header_missing`)", body = ErrorEnvelope),
    )
)]
async fn mark_all_read(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<MarkAllReadResponse>>, ApiError> {
    let updated = use_cases(&state)?.mark_all_read(&tenant.scope).await?;
    Ok(Json(Data::new(MarkAllReadResponse { updated })))
}

#[utoipa::path(
    post,
    path = "/notifications/{notification_id}/read",
    tag = TAG,
    operation_id = "notifications_mark_read",
    security(("session_cookie" = [])),
    params(("notification_id" = String, Path, description = "Notification id")),
    responses(
        (status = 204, description = "Notification marked as read (idempotent)"),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `X-Requested-With` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 404, description = "Notification not found or of another user (`notification_not_found`)", body = ErrorEnvelope),
    )
)]
async fn mark_read(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(notification_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .mark_read(&tenant.scope, &notification_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/notifications/preferences",
    tag = TAG,
    operation_id = "notifications_preferences_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Effective preference of every supported kind × channel pair (stored choice or default)", body = Data<NotificationPreferencesResponse>),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
    )
)]
async fn get_preferences(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<NotificationPreferencesResponse>>, ApiError> {
    let preferences = use_cases(&state)?.preferences(&tenant.scope).await?;
    Ok(Json(Data::new(preferences.into())))
}

#[utoipa::path(
    put,
    path = "/notifications/preferences",
    tag = TAG,
    operation_id = "notifications_preferences_update",
    security(("session_cookie" = [])),
    request_body = UpdateNotificationPreferencesRequest,
    responses(
        (status = 200, description = "Preferences saved (only the pairs sent change); returns every effective preference", body = Data<NotificationPreferencesResponse>),
        (status = 401, description = "Missing or invalid session (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `X-Requested-With` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown kind or channel not supported by the kind (`validation_error` on `preferences.<index>.kind|channel`)", body = ErrorEnvelope),
    )
)]
async fn update_preferences(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<UpdateNotificationPreferencesRequest>,
) -> Result<Json<Data<NotificationPreferencesResponse>>, ApiError> {
    let preferences = use_cases(&state)?
        .update_preferences(&tenant.scope, request.into_updates())
        .await?;
    Ok(Json(Data::new(preferences.into())))
}
