use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_outbound::{PreviewEmailRequest, PreviewEmailResponse, SendEmailRequest};
use crate::dto::email_sync::EmailMessageResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "emails";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(send_email))
        .routes(routes!(preview_email))
        .routes(routes!(cancel_email))
}

#[utoipa::path(
    post,
    path = "/emails",
    tag = TAG,
    operation_id = "emails_send",
    description = "Queues an e-mail from one of the user's own mailboxes (`mailbox_id`, or the first active one). `record` links the conversation to a contact, company, deal or custom object (read access required) and fills the `{{contact.first_name}}`-style variables of `subject`/`body_html`/`template_id`; recipients that are people are linked too. `reply_to_message_id` keeps the conversation (`In-Reply-To`, `References`, `Re:` subject). `attachment_file_ids` are files the user can download (`/api/v1/files`). Every e-mail is stored as `scheduled` and delivered by the worker (`email.send_scheduled`); a future `scheduled_at` delays delivery. `tracking` asks for open/click tracking when the organization allows it (default from `/email-settings/tracking`); it is always off when a recipient objected. Queue acceptance publishes `email.scheduled`; delivery publishes `email.sent` once per linked record.",
    security(("session_cookie" = [])),
    request_body = SendEmailRequest,
    responses(
        (status = 201, description = "E-mail queued (`status = scheduled`)", body = Data<EmailMessageResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` or read access to the record (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown mailbox, replied message or template (`mailbox_not_found`, `email_not_found`, `email_template_not_found`, `file_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid input (`validation_error` with `details.fields`), no mailbox (`no_mailbox_available`), disconnected mailbox (`mailbox_unavailable`), attachments over 25 MB (`email_attachments_too_large`) or a provider failure (`mail_provider_authentication`, `mail_provider_unavailable`, `mail_provider_rejected`, `mail_provider_unsupported`); a failed e-mail stays in the conversation with `status = failed`", body = ErrorEnvelope),
    )
)]
async fn send_email(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<SendEmailRequest>,
) -> Result<(StatusCode, Json<Data<EmailMessageResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let email = service::compose::send(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(email.into()))))
}

#[utoipa::path(
    post,
    path = "/emails/preview",
    tag = TAG,
    operation_id = "emails_preview",
    description = "Renders a template (`template_id`) or the given `subject`/`body_html` with the variables of `record` and of the sender (`{{user.display_name}}`, `{{user.email}}`). Values are HTML-escaped in the body; hidden fields and unreadable related records become missing variables (rendered empty and listed in `missing_variables`). The HTML is sanitized (no script, iframe, event handlers or `javascript:` URLs).",
    security(("session_cookie" = [])),
    request_body = PreviewEmailRequest,
    responses(
        (status = 200, description = "Rendered subject, HTML and plain text", body = Data<PreviewEmailResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` or read access to the record (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`email_template_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid record reference (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn preview_email(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<PreviewEmailRequest>,
) -> Result<Json<Data<PreviewEmailResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let rendered = service::templates::preview(&ctx, request.into()).await?;
    Ok(Json(Data::new(rendered.into())))
}

#[utoipa::path(
    post,
    path = "/emails/{email_id}/cancel",
    tag = TAG,
    operation_id = "emails_cancel",
    description = "Cancels a scheduled e-mail of the user's mailbox before it is sent (`email.cancelled` with audit).",
    security(("session_cookie" = [])),
    params(("email_id" = String, Path, description = "E-mail message id")),
    responses(
        (status = 200, description = "E-mail with `status = cancelled`", body = Data<EmailMessageResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown e-mail or e-mail of another user (`email_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The e-mail is no longer scheduled (`email_not_cancellable`)", body = ErrorEnvelope),
    )
)]
async fn cancel_email(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(email_id): Path<String>,
) -> Result<Json<Data<EmailMessageResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let email = service::compose::cancel(&ctx, &email_id, client.0.ip).await?;
    Ok(Json(Data::new(email.into())))
}
