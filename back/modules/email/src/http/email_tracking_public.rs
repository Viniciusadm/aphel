use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::service;
use crate::tracking::tracking_scope;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::ErrorEnvelope;
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::AuthContext;
use aphel_kernel::api::state::AppState;

pub(crate) const TAG: &str = "email tracking";

const PIXEL: [u8; 43] = [
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0xff, 0x21, 0xf9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3b,
];

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(track_open))
        .routes(routes!(track_click))
}

fn pixel_response() -> Response {
    let mut response = (StatusCode::OK, PIXEL.to_vec()).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/gif"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

#[utoipa::path(
    get,
    path = "/e/o/{token}",
    tag = TAG,
    operation_id = "email_tracking_open",
    description = "Public open-tracking pixel embedded in tracked e-mails (no session). The token is an HMAC-signed reference to the message, without personal data. Always answers a 1×1 GIF; only valid tokens of tracked messages record the open (`opened_at`, `open_count`; `email.opened` on the first one). Requests by the mailbox owner signed in to Aphel and messages to addresses that objected to tracking are ignored.",
    params(("token" = String, Path, description = "Signed tracking token")),
    responses((status = 200, description = "Transparent 1×1 GIF", content_type = "image/gif")),
)]
async fn track_open(
    State(state): State<AppState>,
    Path(token): Path<String>,
    auth: Result<AuthContext, ApiError>,
) -> Response {
    let viewer = auth.ok().map(|auth| auth.user.id.clone());
    let scope = tracking_scope();
    match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => {
            if let Err(error) = service::tracking::record_open(&ctx, &token, viewer.as_ref()).await
            {
                tracing::warn!(%error, "failed to record an e-mail open");
            }
        }
        Err(error) => tracing::warn!(%error, "e-mail tracking is unavailable"),
    }
    pixel_response()
}

#[utoipa::path(
    get,
    path = "/e/c/{token}",
    tag = TAG,
    operation_id = "email_tracking_click",
    description = "Public click-tracking redirect for links of tracked e-mails (no session). The signed token points to a link stored when the e-mail was sent, so only URLs that were in the sent body are reachable (no open redirect). Records the click (`clicked_at`, `click_count`; `email.clicked` on the first one) unless it comes from the mailbox owner or the recipient objected to tracking.",
    params(("token" = String, Path, description = "Signed tracking token")),
    responses(
        (status = 302, description = "Redirect to the original link"),
        (status = 404, description = "Invalid token or unknown link (`email_tracking_link_not_found`)", body = ErrorEnvelope),
    ),
)]
async fn track_click(
    State(state): State<AppState>,
    Path(token): Path<String>,
    auth: Result<AuthContext, ApiError>,
) -> Result<Response, ApiError> {
    let viewer = auth.ok().map(|auth| auth.user.id.clone());
    let scope = tracking_scope();
    let ctx = Ctx::from_state(&state, &scope).await?;
    let url = service::tracking::record_click(&ctx, &token, viewer.as_ref()).await?;
    let location = HeaderValue::from_str(&url).map_err(|_| {
        aphel_kernel::errors::AppError::Internal("stored link is not a valid header".to_owned())
    })?;
    let mut response = StatusCode::FOUND.into_response();
    let headers = response.headers_mut();
    headers.insert(header::LOCATION, location);
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    Ok(response)
}
