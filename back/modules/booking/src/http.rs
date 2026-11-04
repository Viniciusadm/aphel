use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{
    BookingAvailabilityRequest, BookingAvailabilityResponse, BookingContactFieldResponse,
    BookingReference, BookingUserParams, MeetingBookingCancelRequest, MeetingBookingListParams,
    MeetingBookingResponse, MeetingTypeRequest, MeetingTypeResponse,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "booking";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_availability, save_availability))
        .routes(routes!(contact_fields))
        .routes(routes!(distribution_rules))
        .routes(routes!(list_meeting_types, create_meeting_type))
        .routes(routes!(
            get_meeting_type,
            update_meeting_type,
            delete_meeting_type
        ))
        .routes(routes!(list_bookings))
        .routes(routes!(cancel_booking))
}

#[utoipa::path(
    get,
    path = "/booking/availability",
    tag = TAG,
    operation_id = "booking_availability_get",
    security(("session_cookie" = [])),
    params(BookingUserParams),
    responses(
        (status = 200, description = "Weekly hours, time zone, minimum notice, daily limit, buffers and booking horizon used by the booking links of the user (yours, or `user_id` when your `booking.manage` scope covers that member). Defaults (Monday to Friday 09:00–18:00, 4 h notice, 60 days) with `configured: false` until saved", body = Data<BookingAvailabilityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` or user outside your scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn get_availability(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<BookingUserParams>,
) -> Result<Json<Data<BookingAvailabilityResponse>>, ApiError> {
    let availability = service::availability(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.user_id.as_deref(),
    )
    .await?;
    Ok(Json(Data::new(availability.into())))
}

#[utoipa::path(
    put,
    path = "/booking/availability",
    tag = TAG,
    operation_id = "booking_availability_save",
    security(("session_cookie" = [])),
    params(
        BookingUserParams,
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the availability as read; fails with 409 when it changed since"),
    ),
    request_body = BookingAvailabilityRequest,
    responses(
        (status = 200, description = "Availability saved (`booking_availability.updated`); omitted properties stay unchanged and `weekly_hours` replaces the whole week (`weekday` 1 = Monday … 7 = Sunday, `HH:MM`, intervals of a day cannot overlap)", body = Data<BookingAvailabilityResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` or user outside your scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid time zone, hours or limits (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn save_availability(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Query(params): Query<BookingUserParams>,
    JsonBody(request): JsonBody<BookingAvailabilityRequest>,
) -> Result<Json<Data<BookingAvailabilityResponse>>, ApiError> {
    let availability = service::save_availability(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.user_id.as_deref(),
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(availability.into())))
}

#[utoipa::path(
    get,
    path = "/booking/contact-fields",
    tag = TAG,
    operation_id = "booking_contact_fields",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Contact fields that can receive the answer of a booking question (phone, mobile, job title, notes, city, state, country and active text-like custom fields). Answers fill the contact only when the booking creates it", body = Data<Vec<BookingContactFieldResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn contact_fields(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<BookingContactFieldResponse>>>, ApiError> {
    let fields = service::contact_fields(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        fields.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/booking/distribution-rules",
    tag = TAG,
    operation_id = "booking_distribution_rules",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Active lead distribution rules that a team booking link can use to pick the host", body = Data<Vec<BookingReference>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Team links need `booking.manage` for the team or the organization (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn distribution_rules(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<BookingReference>>>, ApiError> {
    let rules = service::distribution_rules(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        rules
            .into_iter()
            .map(|(id, name)| BookingReference {
                id: id.to_string(),
                name,
            })
            .collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/booking/meeting-types",
    tag = TAG,
    operation_id = "booking_meeting_types_list",
    security(("session_cookie" = [])),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Meeting types (booking links) whose owner is within your `booking.manage` scope, ordered by name", body = Page<MeetingTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_meeting_types(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
) -> Result<Json<Page<MeetingTypeResponse>>, ApiError> {
    let listing = service::list_meeting_types(
        &Ctx::from_state(&state, &tenant.scope).await?,
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
    path = "/booking/meeting-types",
    tag = TAG,
    operation_id = "booking_meeting_types_create",
    security(("session_cookie" = [])),
    request_body = MeetingTypeRequest,
    responses(
        (status = 201, description = "Meeting type created (`meeting_type.created`) and owned by you; its public page is `/book/{slug}` (`slug` derived from the name when omitted). `location_kind` is `none`, `video`, `address` or `phone` (value shown to the guest only after booking). `questions` (up to 10) may map the answer to a contact field (see `/booking/contact-fields`). `distribution_rule_id` turns it into a team link: the host is picked among the free members of the rule by its rotation. `spam_protection` enables the honeypot", body = Data<MeetingTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage`, or a team link without team/organization scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another link already uses the slug (`meeting_type_slug_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties, questions or rule (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_meeting_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<MeetingTypeRequest>,
) -> Result<(StatusCode, Json<Data<MeetingTypeResponse>>), ApiError> {
    let meeting_type = service::create_meeting_type(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, None),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(meeting_type.into()))))
}

#[utoipa::path(
    get,
    path = "/booking/meeting-types/{meeting_type_id}",
    tag = TAG,
    operation_id = "booking_meeting_types_get",
    security(("session_cookie" = [])),
    params(("meeting_type_id" = String, Path, description = "Meeting type id")),
    responses(
        (status = 200, description = "Meeting type", body = Data<MeetingTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Not found or outside your scope (`meeting_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_meeting_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(meeting_type_id): Path<String>,
) -> Result<Json<Data<MeetingTypeResponse>>, ApiError> {
    let meeting_type = service::get_meeting_type(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &meeting_type_id,
    )
    .await?;
    Ok(Json(Data::new(meeting_type.into())))
}

#[utoipa::path(
    patch,
    path = "/booking/meeting-types/{meeting_type_id}",
    tag = TAG,
    operation_id = "booking_meeting_types_update",
    security(("session_cookie" = [])),
    params(
        ("meeting_type_id" = String, Path, description = "Meeting type id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the meeting type as read; fails with 409 when it changed since"),
    ),
    request_body = MeetingTypeRequest,
    responses(
        (status = 200, description = "Meeting type updated (`meeting_type.updated`); omitted properties stay unchanged, `questions` replaces the list and `distribution_rule_id: null` turns a team link into a personal one. Existing bookings keep their time", body = Data<MeetingTypeResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Not found or outside your scope (`meeting_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Slug taken (`meeting_type_slug_taken`) or changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_meeting_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(meeting_type_id): Path<String>,
    JsonBody(request): JsonBody<MeetingTypeRequest>,
) -> Result<Json<Data<MeetingTypeResponse>>, ApiError> {
    let meeting_type = service::update_meeting_type(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &meeting_type_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(meeting_type.into())))
}

#[utoipa::path(
    delete,
    path = "/booking/meeting-types/{meeting_type_id}",
    tag = TAG,
    operation_id = "booking_meeting_types_delete",
    security(("session_cookie" = [])),
    params(("meeting_type_id" = String, Path, description = "Meeting type id")),
    responses(
        (status = 204, description = "Meeting type and its booking history deleted (`meeting_type.deleted`); the meeting tasks stay in the CRM"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Not found or outside your scope (`meeting_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_meeting_type(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(meeting_type_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete_meeting_type(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &meeting_type_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/booking/bookings",
    tag = TAG,
    operation_id = "booking_bookings_list",
    security(("session_cookie" = [])),
    params(MeetingBookingListParams, PaginationQuery),
    responses(
        (status = 200, description = "Meetings booked through booking links whose host is within your `booking.manage` scope; `upcoming=true` lists the ones not finished yet in chronological order, otherwise newest first", body = Page<MeetingBookingResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_bookings(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<MeetingBookingListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<MeetingBookingResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let listing = service::list_bookings(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_query(),
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
    path = "/booking/bookings/{booking_id}/cancel",
    tag = TAG,
    operation_id = "booking_bookings_cancel",
    security(("session_cookie" = [])),
    params(("booking_id" = String, Path, description = "Booking id")),
    request_body = MeetingBookingCancelRequest,
    responses(
        (status = 200, description = "Booking canceled by the host (`meeting.canceled`): the meeting activity is canceled and the guest receives the cancellation with an `.ics` (`METHOD:CANCEL`)", body = Data<MeetingBookingResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `booking.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Not found or outside your scope (`booking_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already canceled or in the past (`booking_not_changeable`)", body = ErrorEnvelope),
    )
)]
async fn cancel_booking(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(booking_id): Path<String>,
    JsonBody(request): JsonBody<MeetingBookingCancelRequest>,
) -> Result<Json<Data<MeetingBookingResponse>>, ApiError> {
    let booking = service::cancel_booking(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &booking_id,
        request.reason,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(booking.into())))
}
