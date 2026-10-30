use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::{CompleteOnboardingRequest, OnboardingResponse, SaveOnboardingAnswersRequest};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "onboarding";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_onboarding))
        .routes(routes!(save_answers))
        .routes(routes!(complete_onboarding))
        .routes(routes!(skip_onboarding))
        .routes(routes!(reopen_onboarding))
}

#[utoipa::path(
    get,
    path = "/onboarding",
    tag = TAG,
    operation_id = "onboarding_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "State of the initial setup assistant of the organization (`pending` when it was never answered), the saved answers and the template recommended for them (`null` until a business type is answered). When the assistant was never answered, `answers` are the business answers given during installation, so they are not asked again. Recommended and alternative keys refer to `GET /config-packages/templates`; `reasons` are codes for the interface to translate", body = Data<OnboardingResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn get_onboarding(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<OnboardingResponse>>, ApiError> {
    let view = service::get(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    put,
    path = "/onboarding/answers",
    tag = TAG,
    operation_id = "onboarding_save_answers",
    security(("session_cookie" = [])),
    request_body = SaveOnboardingAnswersRequest,
    responses(
        (status = 200, description = "Answers saved as a draft (no domain event) and the recommendation recalculated", body = Data<OnboardingResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The assistant is not pending (`onboarding_not_pending`); reopen it first", body = ErrorEnvelope),
        (status = 422, description = "Unknown business type, offering (`offering.N`) or team size", body = ErrorEnvelope),
    )
)]
async fn save_answers(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<SaveOnboardingAnswersRequest>,
) -> Result<Json<Data<OnboardingResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::save_answers(&ctx, request.into()).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/onboarding/complete",
    tag = TAG,
    operation_id = "onboarding_complete",
    security(("session_cookie" = [])),
    request_body = CompleteOnboardingRequest,
    responses(
        (status = 200, description = "Assistant completed. The template is applied beforehand with `POST /config-packages/apply`; this call only records the result: `template_key` (`null` when the organization starts from scratch) and the `install_id` returned by the installation. When only `install_id` is sent, the template comes from the installation. Publishes `onboarding.completed`", body = Data<OnboardingResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The assistant is not pending (`onboarding_not_pending`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown template, installation not found in the organization or template different from the installed one (`template_key`, `install_id`)", body = ErrorEnvelope),
    )
)]
async fn complete_onboarding(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CompleteOnboardingRequest>,
) -> Result<Json<Data<OnboardingResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::complete(&ctx, request.into_command(client.0.ip)).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/onboarding/skip",
    tag = TAG,
    operation_id = "onboarding_skip",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Assistant skipped; it can be reopened in the settings. Publishes `onboarding.skipped`", body = Data<OnboardingResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The assistant is not pending (`onboarding_not_pending`)", body = ErrorEnvelope),
    )
)]
async fn skip_onboarding(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
) -> Result<Json<Data<OnboardingResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::skip(&ctx, client.0.ip).await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    post,
    path = "/onboarding/reopen",
    tag = TAG,
    operation_id = "onboarding_reopen",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Assistant back to `pending`, keeping the previous answers and clearing the previous result. Publishes `onboarding.reopened` (no change when it was already pending)", body = Data<OnboardingResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn reopen_onboarding(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
) -> Result<Json<Data<OnboardingResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let view = service::reopen(&ctx, client.0.ip).await?;
    Ok(Json(Data::new(view.into())))
}
