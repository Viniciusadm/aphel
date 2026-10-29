use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::push::{
    CreatePushSubscriptionRequest, PushPublicKeyResponse, RemovePushSubscriptionRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody, SessionContext};
use crate::api::state::AppState;
use crate::application::notifications::push::PushUseCases;
use crate::errors::AppError;
use crate::infrastructure::notifications::push::MySqlPushPorts;

pub const TAG: &str = "notifications";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(public_key))
        .routes(routes!(subscribe))
        .routes(routes!(unsubscribe))
}

fn use_cases(state: &AppState) -> Result<PushUseCases<MySqlPushPorts>, AppError> {
    Ok(PushUseCases::new(MySqlPushPorts::new(
        state.db()?,
        state.secrets.clone(),
    )))
}

#[utoipa::path(
    get,
    path = "/notifications/push/public-key",
    tag = TAG,
    operation_id = "notifications_push_public_key",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "VAPID public key of the installation (uncompressed P-256 point in base64url) to pass as `applicationServerKey` to `PushManager.subscribe`. The key pair is generated on first use and kept encrypted with `APP_ENCRYPTION_KEY`; the private key never leaves the server", body = Data<PushPublicKeyResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured, so web push is unavailable (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn public_key(
    State(state): State<AppState>,
    _session: SessionContext,
) -> Result<Json<Data<PushPublicKeyResponse>>, ApiError> {
    let public_key = use_cases(&state)?.public_key().await?;
    Ok(Json(Data::new(PushPublicKeyResponse { public_key })))
}

#[utoipa::path(
    post,
    path = "/notifications/push/subscriptions",
    tag = TAG,
    operation_id = "notifications_push_subscribe",
    security(("session_cookie" = [])),
    request_body = CreatePushSubscriptionRequest,
    responses(
        (status = 204, description = "Push subscription of this device saved (the endpoint is unique: subscribing again moves it to the current user and session). It is removed when the session ends (sign out, ending sessions, password change) or when the push service answers 404/410"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `X-Requested-With` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Endpoint is not an https URL of a known push service, or invalid `keys.p256dh`/`keys.auth` (`validation_error`)", body = ErrorEnvelope),
        (status = 503, description = "`APP_ENCRYPTION_KEY` is not configured, so web push is unavailable (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn subscribe(
    State(state): State<AppState>,
    session: SessionContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreatePushSubscriptionRequest>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .subscribe(
            &session.user().id,
            session.session_id(),
            request.into_input(client.0.user_agent),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/notifications/push/subscriptions/unsubscribe",
    tag = TAG,
    operation_id = "notifications_push_unsubscribe",
    security(("session_cookie" = [])),
    request_body = RemovePushSubscriptionRequest,
    responses(
        (status = 204, description = "Push subscription of the current user with this endpoint removed (also when there was none)"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Called with an API token (`api_token_not_allowed`) or missing `X-Requested-With` (`csrf_header_missing`)", body = ErrorEnvelope),
        (status = 422, description = "Missing endpoint (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn unsubscribe(
    State(state): State<AppState>,
    session: SessionContext,
    JsonBody(request): JsonBody<RemovePushSubscriptionRequest>,
) -> Result<StatusCode, ApiError> {
    use_cases(&state)?
        .unsubscribe(&session.user().id, &request.endpoint)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
