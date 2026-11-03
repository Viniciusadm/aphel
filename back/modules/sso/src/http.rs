use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::provider::OidcProvider;
use crate::sso::status::{SsoLoginStatus, login_status};
use crate::sso::{CallbackQuery, ClientContext, StartQuery, handle_callback, start_login};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::{ApiError, app_error};
use aphel_kernel::api::extractors::{AuthContext, ClientInfo};
use aphel_kernel::api::state::AppState;
use aphel_kernel::domain::access::AccessScope;

pub(crate) const TAG: &str = "sso";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(status))
}

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(callback))
}

#[utoipa::path(
    get,
    path = "/sso/status",
    tag = TAG,
    operation_id = "sso_status",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Single sign-on configuration summary (never the client secret) and the callback URL to register at the identity provider", body = Data<SsoLoginStatus>),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn status(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<SsoLoginStatus>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(login_status(&ctx, &OidcProvider).await?)))
}

#[utoipa::path(
    get,
    path = "/public/sso/login",
    tag = TAG,
    operation_id = "sso_public_login",
    params(
        StartQuery,
    ),
    responses(
        (status = 302, description = "Redirects to the identity provider (authorization code + PKCE). `login_hint` (the e-mail typed on the login screen) is optional and forwarded to the provider; `redirect` is accepted only as a relative path. On failure redirects to `/login?sso_error=<code>` (`no_account`, `sso_disabled`, `sso_not_configured`, `domain_not_allowed`, `invalid_request`, `provider_unavailable`, `rate_limited`)"),
    )
)]
async fn login(
    State(state): State<AppState>,
    client: ClientInfo,
    Query(query): Query<StartQuery>,
) -> Response {
    let scope = AccessScope::system();
    match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => start_login(&ctx, &OidcProvider, query, ClientContext::new(client)).await,
        Err(error) => app_error(error).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/public/sso/callback",
    tag = TAG,
    operation_id = "sso_public_callback",
    params(CallbackQuery),
    responses(
        (status = 302, description = "Validates the single-use `state`, exchanges the code, validates the id token (`iss`, `aud`, `exp`, `nonce`) and requires `email_verified`. Signs in the active member with that e-mail and redirects to the requested path (or `/login/two-factor`) with the session cookie; on failure redirects to `/login?sso_error=<code>` (`no_account`, `invalid_state`, `access_denied`, `email_not_verified`, `domain_not_allowed`, `token_exchange_failed`, `invalid_token`, `sso_failed`)"),
    )
)]
async fn callback(
    State(state): State<AppState>,
    client: ClientInfo,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let scope = AccessScope::system();
    match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => handle_callback(&ctx, &OidcProvider, query, ClientContext::new(client)).await,
        Err(error) => app_error(error).into_response(),
    }
}
