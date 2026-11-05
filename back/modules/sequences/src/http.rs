use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::EnrollCommand;
use crate::EnrollmentFilter;
use crate::domain::EnrollmentSource;
use crate::dto::{
    EnrollContactRequest, SequenceEnrollmentListParams, SequenceEnrollmentResponse,
    SequenceEnrollmentStatusParams, SequenceListParams, SequenceMetricsResponse, SequenceRequest,
    SequenceResponse,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "sequences";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_sequences, create_sequence))
        .routes(routes!(get_sequence, update_sequence, delete_sequence))
        .routes(routes!(sequence_metrics))
        .routes(routes!(list_sequence_enrollments, enroll_contact))
        .routes(routes!(list_enrollments))
        .routes(routes!(pause_enrollment))
        .routes(routes!(resume_enrollment))
        .routes(routes!(exit_enrollment))
}

fn invalid_query(message: String) -> ApiError {
    AppError::Validation(message).into()
}

#[utoipa::path(
    get,
    path = "/sequences",
    tag = TAG,
    operation_id = "sequences_list",
    description = "Sequences (cadences) of the organization ordered by name, with their steps and enrollment counts. Needs `sequences.read` (counts follow its own/team/organization scope over who enrolled) or `sequences.manage`.",
    security(("session_cookie" = [])),
    params(SequenceListParams, PaginationQuery),
    responses(
        (status = 200, description = "Sequences", body = Page<SequenceResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_sequences(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<SequenceListParams>,
    pagination: Pagination,
) -> Result<Json<Page<SequenceResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list(&ctx, params.into_query(), window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/sequences",
    tag = TAG,
    operation_id = "sequences_create",
    description = "Creates a sequence (`sequence.created`). Steps run in order: `auto_email` (sent from the mailbox of whoever enrolled the contact, inside the sending window and the daily limit of that mailbox, never two on the same day for the same contact), `manual_email` (an `email` task for the enroller with the rendered draft), `call`, `linkedin` and `task` (tasks for the enroller) and `wait` (`wait_days` business days in the organization timezone). E-mail steps take `template_id` (an organization template) or `subject` + `body_html` with `{{contact.first_name}}`-style variables; later e-mails reply in the same conversation. `window_start`/`window_end` are `HH:MM` in the organization timezone (default 08:00–18:00, business days); `daily_limit` is the maximum of sequence e-mails per mailbox per day (default 50). Needs `sequences.manage`.",
    security(("session_cookie" = [])),
    request_body = SequenceRequest,
    responses(
        (status = 201, description = "Sequence created", body = Data<SequenceResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another sequence already uses the key (`sequence_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or steps (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn create_sequence(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<SequenceRequest>,
) -> Result<(StatusCode, Json<Data<SequenceResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let sequence = service::create(&ctx, request.into_command(client.0.ip, None)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(sequence.into()))))
}

#[utoipa::path(
    get,
    path = "/sequences/{sequence_id}",
    tag = TAG,
    operation_id = "sequences_get",
    description = "Sequence with steps and enrollment counts. Needs `sequences.read` or `sequences.manage`.",
    security(("session_cookie" = [])),
    params(("sequence_id" = String, Path, description = "Sequence id")),
    responses(
        (status = 200, description = "Sequence", body = Data<SequenceResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Sequence not found (`sequence_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_sequence(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(sequence_id): Path<String>,
) -> Result<Json<Data<SequenceResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let sequence = service::get(&ctx, &sequence_id).await?;
    Ok(Json(Data::new(sequence.into())))
}

#[utoipa::path(
    patch,
    path = "/sequences/{sequence_id}",
    tag = TAG,
    operation_id = "sequences_update",
    description = "Updates a sequence (`sequence.updated`); omitted properties stay unchanged and `steps` replaces the whole list (send the `id` of a step to keep it and its metrics). Active enrollments continue from their current position. `is_active: false` holds every enrollment until the sequence is active again. Needs `sequences.manage`.",
    security(("session_cookie" = [])),
    params(
        ("sequence_id" = String, Path, description = "Sequence id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the sequence as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = SequenceRequest,
    responses(
        (status = 200, description = "Sequence updated", body = Data<SequenceResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Sequence not found (`sequence_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`sequence_key_taken`) or the sequence changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_sequence(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(sequence_id): Path<String>,
    JsonBody(request): JsonBody<SequenceRequest>,
) -> Result<Json<Data<SequenceResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let sequence = service::update(
        &ctx,
        &sequence_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(sequence.into())))
}

#[utoipa::path(
    delete,
    path = "/sequences/{sequence_id}",
    tag = TAG,
    operation_id = "sequences_delete",
    description = "Deletes a sequence with its enrollments and step history (`sequence.deleted`). E-mails and tasks already created stay. Needs `sequences.manage`.",
    security(("session_cookie" = [])),
    params(("sequence_id" = String, Path, description = "Sequence id")),
    responses(
        (status = 204, description = "Sequence deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Sequence not found (`sequence_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_sequence(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(sequence_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::delete(&ctx, &sequence_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/sequences/{sequence_id}/metrics",
    tag = TAG,
    operation_id = "sequences_metrics",
    description = "Enrollment counts and metrics per step: `executed` (steps run), `sent` (e-mails sent or tasks created), `opened` (tracked e-mails opened), `replied` (replies received in the conversation), `bounced`, `failed` and `exits` (enrollments that left the sequence right after the step). Counts follow the `sequences.read` scope over who enrolled.",
    security(("session_cookie" = [])),
    params(("sequence_id" = String, Path, description = "Sequence id")),
    responses(
        (status = 200, description = "Metrics", body = Data<SequenceMetricsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Sequence not found (`sequence_not_found`)", body = ErrorEnvelope),
    )
)]
async fn sequence_metrics(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(sequence_id): Path<String>,
) -> Result<Json<Data<SequenceMetricsResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let metrics = service::metrics(&ctx, &sequence_id).await?;
    Ok(Json(Data::new(metrics.into())))
}

#[utoipa::path(
    get,
    path = "/sequences/{sequence_id}/enrollments",
    tag = TAG,
    operation_id = "sequences_enrollments_list",
    description = "Enrollments of the sequence, newest first. Needs `sequences.read` (own/team/organization over who enrolled).",
    security(("session_cookie" = [])),
    params(
        ("sequence_id" = String, Path, description = "Sequence id"),
        SequenceEnrollmentStatusParams,
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Enrollments", body = Page<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Sequence not found (`sequence_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid status or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_sequence_enrollments(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(sequence_id): Path<String>,
    params: Result<Query<SequenceEnrollmentStatusParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<SequenceEnrollmentResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| invalid_query(rejection.body_text()))?;
    let status = params.status.map(Into::into);
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let sequence = service::get(&ctx, &sequence_id).await?;
    let listing = service::list_enrollments(
        &ctx,
        EnrollmentFilter {
            sequence_id: Some(sequence.sequence.id),
            status,
            ..EnrollmentFilter::default()
        },
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
    path = "/sequences/{sequence_id}/enrollments",
    tag = TAG,
    operation_id = "sequences_enroll",
    description = "Enrolls a contact (`sequence.enrolled`). E-mails go out from `mailbox_id` (one of the user's mailboxes) or the user's first connected mailbox. The contact leaves the sequence automatically when they reply, book a meeting, a deal of theirs is won or lost, an e-mail bounces, they object to tracking or they are deleted (`sequence.exited` with `exit_reason`); after the last step the enrollment is `completed` (`sequence.completed`). Needs `sequences.enroll` and read access to the contact.",
    security(("session_cookie" = [])),
    params(("sequence_id" = String, Path, description = "Sequence id")),
    request_body = EnrollContactRequest,
    responses(
        (status = 201, description = "Contact enrolled", body = Data<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.enroll` or read access to the contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown sequence, contact or mailbox (`sequence_not_found`, `contact_not_found`, `mailbox_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The contact is already active or paused in the sequence (`sequence_already_enrolled`)", body = ErrorEnvelope),
        (status = 422, description = "Inactive or empty sequence (`sequence_inactive`, `sequence_empty`), contact without e-mail (`sequence_contact_without_email`), contact that objected (`sequence_contact_objected`) or no mailbox to send from (`no_mailbox_available`)", body = ErrorEnvelope),
    )
)]
async fn enroll_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(sequence_id): Path<String>,
    JsonBody(request): JsonBody<EnrollContactRequest>,
) -> Result<(StatusCode, Json<Data<SequenceEnrollmentResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let enrollment = service::enroll(
        &ctx,
        EnrollCommand {
            sequence_id,
            contact_id: request.contact_id,
            mailbox_id: request.mailbox_id,
            source: EnrollmentSource::Manual,
            enroll_as: None,
            ip: client.0.ip,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(enrollment.into()))))
}

#[utoipa::path(
    get,
    path = "/sequence-enrollments",
    tag = TAG,
    operation_id = "sequence_enrollments_list",
    description = "Enrollments filtered by `contact_id` (needs read access to the contact), `sequence_id` and `status`, newest first. Needs `sequences.read` (own/team/organization over who enrolled).",
    security(("session_cookie" = [])),
    params(SequenceEnrollmentListParams, PaginationQuery),
    responses(
        (status = 200, description = "Enrollments", body = Page<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.read` or read access to the contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filters or pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_enrollments(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<SequenceEnrollmentListParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<SequenceEnrollmentResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| invalid_query(rejection.body_text()))?;
    let filter = params.into_filter().map_err(invalid_query)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let listing = service::list_enrollments(&ctx, filter, window(pagination)).await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/sequence-enrollments/{enrollment_id}/pause",
    tag = TAG,
    operation_id = "sequence_enrollments_pause",
    description = "Pauses an active enrollment (`sequence_enrollment.paused`); no step runs until it is resumed. Needs `sequences.enroll` (own/team/organization over who enrolled).",
    security(("session_cookie" = [])),
    params(("enrollment_id" = String, Path, description = "Enrollment id")),
    responses(
        (status = 200, description = "Enrollment paused", body = Data<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Enrollment not found (`sequence_enrollment_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The enrollment is not active (`sequence_enrollment_finished`)", body = ErrorEnvelope),
    )
)]
async fn pause_enrollment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(enrollment_id): Path<String>,
) -> Result<Json<Data<SequenceEnrollmentResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let enrollment = service::pause(&ctx, &enrollment_id, client.0.ip).await?;
    Ok(Json(Data::new(enrollment.into())))
}

#[utoipa::path(
    post,
    path = "/sequence-enrollments/{enrollment_id}/resume",
    tag = TAG,
    operation_id = "sequence_enrollments_resume",
    description = "Resumes a paused enrollment (`sequence_enrollment.resumed`); the pending step runs on the next cycle, respecting the sending window and limits. Needs `sequences.enroll`.",
    security(("session_cookie" = [])),
    params(("enrollment_id" = String, Path, description = "Enrollment id")),
    responses(
        (status = 200, description = "Enrollment resumed", body = Data<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Enrollment not found (`sequence_enrollment_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The enrollment is not paused (`sequence_enrollment_finished`)", body = ErrorEnvelope),
    )
)]
async fn resume_enrollment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(enrollment_id): Path<String>,
) -> Result<Json<Data<SequenceEnrollmentResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let enrollment = service::resume(&ctx, &enrollment_id, client.0.ip).await?;
    Ok(Json(Data::new(enrollment.into())))
}

#[utoipa::path(
    post,
    path = "/sequence-enrollments/{enrollment_id}/exit",
    tag = TAG,
    operation_id = "sequence_enrollments_exit",
    description = "Removes the contact from the sequence (`sequence.exited` with `exit_reason = removed`). Needs `sequences.enroll`.",
    security(("session_cookie" = [])),
    params(("enrollment_id" = String, Path, description = "Enrollment id")),
    responses(
        (status = 200, description = "Enrollment exited", body = Data<SequenceEnrollmentResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Enrollment not found (`sequence_enrollment_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The enrollment already finished (`sequence_enrollment_finished`)", body = ErrorEnvelope),
    )
)]
async fn exit_enrollment(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(enrollment_id): Path<String>,
) -> Result<Json<Data<SequenceEnrollmentResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let enrollment = service::remove(&ctx, &enrollment_id, client.0.ip).await?;
    Ok(Json(Data::new(enrollment.into())))
}
