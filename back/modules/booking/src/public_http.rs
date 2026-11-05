use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::SlotWindow;
use crate::dto::{
    PublicBookingRequest, PublicBookingResponse, PublicCancelRequest, PublicMeetingTypeResponse,
    PublicRescheduleRequest, PublicSlotsParams, PublicSlotsResponse,
};
use crate::service::booking;
use crate::validation::parse_instant;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{ClientInfo, JsonBody};
use aphel_sdk::platform::api::rate_limit::SharedRateLimiter;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const TAG: &str = "booking";
const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);

struct BookingLimits {
    views: SharedRateLimiter,
    bookings_per_address: SharedRateLimiter,
    bookings_per_link: SharedRateLimiter,
    changes_per_address: SharedRateLimiter,
}

static LIMITS: LazyLock<BookingLimits> = LazyLock::new(|| BookingLimits {
    views: SharedRateLimiter::new("booking.views", 120, MINUTE),
    bookings_per_address: SharedRateLimiter::new("booking.bookings_per_address", 10, HOUR),
    bookings_per_link: SharedRateLimiter::new("booking.bookings_per_link", 200, HOUR),
    changes_per_address: SharedRateLimiter::new("booking.changes_per_address", 30, HOUR),
});

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(public_meeting_type, public_book))
        .routes(routes!(public_slots))
        .routes(routes!(public_manage))
        .routes(routes!(public_manage_slots))
        .routes(routes!(public_reschedule))
        .routes(routes!(public_cancel))
}

async fn limit(limiter: &SharedRateLimiter, key: &str) -> Result<(), AppError> {
    limiter
        .check(key, "too many attempts; wait a little and try again")
        .await
}

static SYSTEM_SCOPE: LazyLock<AccessScope> = LazyLock::new(AccessScope::system);

async fn ctx(state: &AppState) -> Result<Ctx<'static>, AppError> {
    Ctx::from_state(state, &SYSTEM_SCOPE).await
}

fn window(params: &PublicSlotsParams) -> Result<SlotWindow, AppError> {
    match (parse_instant(&params.from), parse_instant(&params.to)) {
        (Some(from), Some(to)) => Ok(SlotWindow { from, to }),
        _ => Err(AppError::invalid_fields(
            "invalid window",
            vec![FieldViolation::new(
                "from",
                "from and to must be RFC 3339 instants",
            )],
        )),
    }
}

#[utoipa::path(
    get,
    path = "/public/book/{slug}",
    tag = TAG,
    operation_id = "booking_public_meeting_type",
    description = "Public data of an active booking link (no session): name, duration, kind of location, host name (null for team links), organization name, host time zone and the questions of the booking form.",
    params(("slug" = String, Path, description = "Slug of the booking link")),
    responses(
        (status = 200, description = "Booking link", body = Data<PublicMeetingTypeResponse>),
        (status = 404, description = "Unknown or inactive link (`meeting_type_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_meeting_type(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(slug): Path<String>,
) -> Result<Json<Data<PublicMeetingTypeResponse>>, ApiError> {
    limit(&LIMITS.views, client.ip_or_unknown()).await?;
    let meeting_type = booking::public_meeting_type(&ctx(&state).await?, &slug).await?;
    Ok(Json(Data::new(meeting_type.into())))
}

#[utoipa::path(
    get,
    path = "/public/book/{slug}/slots",
    tag = TAG,
    operation_id = "booking_public_slots",
    description = "Free start times of the link between `from` and `to` (RFC 3339, at most 42 days apart), no session. Only free slots are returned, never anything about the host's other commitments: open tasks with time and duration, other bookings, the connected Google/Microsoft calendars (busy times only), buffers, minimum notice, daily limit and horizon are applied before. For team links the slots of all eligible members are merged.",
    params(("slug" = String, Path, description = "Slug of the booking link"), PublicSlotsParams),
    responses(
        (status = 200, description = "Free slots", body = Data<PublicSlotsResponse>),
        (status = 404, description = "Unknown or inactive link (`meeting_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid window (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_slots(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(slug): Path<String>,
    Query(params): Query<PublicSlotsParams>,
) -> Result<Json<Data<PublicSlotsResponse>>, ApiError> {
    limit(&LIMITS.views, client.ip_or_unknown()).await?;
    let slots = booking::public_slots(&ctx(&state).await?, &slug, window(&params)?).await?;
    Ok(Json(Data::new(slots.into())))
}

#[utoipa::path(
    post,
    path = "/public/book/{slug}",
    tag = TAG,
    operation_id = "booking_public_book",
    description = "Books a free slot (no session, no `X-Requested-With` needed). Finds the contact by e-mail or creates it (owned by the host, with the mapped answers), creates a `meeting` activity linked to it and assigned to the host, sends the confirmation with an `.ics` invitation to the guest and the host, and publishes `meeting.booked`. Team links pick the host among the free members by the rotation of the distribution rule. `website` is a honeypot that must stay empty when the link has spam protection. The answer carries `manage_token`, used to reschedule or cancel.",
    params(("slug" = String, Path, description = "Slug of the booking link")),
    request_body = PublicBookingRequest,
    responses(
        (status = 201, description = "Booked", body = Data<PublicBookingResponse>),
        (status = 404, description = "Unknown or inactive link (`meeting_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The slot is no longer free (`booking_slot_unavailable`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid data (`validation_error`) or rejected as spam (`booking_rejected`)", body = ErrorEnvelope),
        (status = 429, description = "Too many bookings from this address or for this link (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_book(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<PublicBookingRequest>,
) -> Result<(StatusCode, Json<Data<PublicBookingResponse>>), ApiError> {
    limit(&LIMITS.bookings_per_address, client.ip_or_unknown()).await?;
    limit(&LIMITS.bookings_per_link, &slug.trim().to_lowercase()).await?;
    let booking = booking::book(
        &ctx(&state).await?,
        &slug,
        request.into_command(client.0.ip.clone()),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(PublicBookingResponse::from_public(
            booking,
            Utc::now(),
        ))),
    ))
}

#[utoipa::path(
    get,
    path = "/public/book/manage/{token}",
    tag = TAG,
    operation_id = "booking_public_booking",
    description = "Booking behind a signed management link (no session): meeting, time, host, location and status; `can_change` tells whether it can still be rescheduled or canceled.",
    params(("token" = String, Path, description = "Signed management token")),
    responses(
        (status = 200, description = "Booking", body = Data<PublicBookingResponse>),
        (status = 404, description = "Invalid token (`booking_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_manage(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
) -> Result<Json<Data<PublicBookingResponse>>, ApiError> {
    limit(&LIMITS.views, client.ip_or_unknown()).await?;
    let booking = booking::manage(&ctx(&state).await?, &token).await?;
    Ok(Json(Data::new(PublicBookingResponse::from_public(
        booking,
        Utc::now(),
    ))))
}

#[utoipa::path(
    get,
    path = "/public/book/manage/{token}/slots",
    tag = TAG,
    operation_id = "booking_public_booking_slots",
    description = "Free slots of the same host to reschedule the booking (its own time counts as free). Same rules as the slots of the link.",
    params(("token" = String, Path, description = "Signed management token"), PublicSlotsParams),
    responses(
        (status = 200, description = "Free slots", body = Data<PublicSlotsResponse>),
        (status = 404, description = "Invalid token (`booking_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Canceled or past booking (`booking_not_changeable`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid window (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_manage_slots(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
    Query(params): Query<PublicSlotsParams>,
) -> Result<Json<Data<PublicSlotsResponse>>, ApiError> {
    limit(&LIMITS.views, client.ip_or_unknown()).await?;
    let slots = booking::manage_slots(&ctx(&state).await?, &token, window(&params)?).await?;
    Ok(Json(Data::new(slots.into())))
}

#[utoipa::path(
    post,
    path = "/public/book/manage/{token}/reschedule",
    tag = TAG,
    operation_id = "booking_public_reschedule",
    description = "Moves the booking to another free slot of the same host (no session): updates the meeting activity, sends the updated `.ics` to the guest and the host and publishes `meeting.rescheduled`.",
    params(("token" = String, Path, description = "Signed management token")),
    request_body = PublicRescheduleRequest,
    responses(
        (status = 200, description = "Rescheduled", body = Data<PublicBookingResponse>),
        (status = 404, description = "Invalid token (`booking_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Slot no longer free (`booking_slot_unavailable`) or booking canceled/past (`booking_not_changeable`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid time (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "Too many changes from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_reschedule(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
    JsonBody(request): JsonBody<PublicRescheduleRequest>,
) -> Result<Json<Data<PublicBookingResponse>>, ApiError> {
    limit(&LIMITS.changes_per_address, client.ip_or_unknown()).await?;
    let booking = booking::reschedule(
        &ctx(&state).await?,
        &token,
        &request.starts_at,
        client.0.ip.clone(),
    )
    .await?;
    Ok(Json(Data::new(PublicBookingResponse::from_public(
        booking,
        Utc::now(),
    ))))
}

#[utoipa::path(
    post,
    path = "/public/book/manage/{token}/cancel",
    tag = TAG,
    operation_id = "booking_public_cancel",
    description = "Cancels the booking (no session): cancels the meeting activity, sends the cancellation `.ics` (`METHOD:CANCEL`) to the guest and the host and publishes `meeting.canceled`.",
    params(("token" = String, Path, description = "Signed management token")),
    request_body = PublicCancelRequest,
    responses(
        (status = 200, description = "Canceled", body = Data<PublicBookingResponse>),
        (status = 404, description = "Invalid token (`booking_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already canceled or past (`booking_not_changeable`)", body = ErrorEnvelope),
        (status = 429, description = "Too many changes from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_cancel(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
    JsonBody(request): JsonBody<PublicCancelRequest>,
) -> Result<Json<Data<PublicBookingResponse>>, ApiError> {
    limit(&LIMITS.changes_per_address, client.ip_or_unknown()).await?;
    let booking = booking::cancel_with_token(
        &ctx(&state).await?,
        &token,
        request.reason,
        client.0.ip.clone(),
    )
    .await?;
    Ok(Json(Data::new(PublicBookingResponse::from_public(
        booking,
        Utc::now(),
    ))))
}
