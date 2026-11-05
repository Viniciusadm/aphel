use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::provider::MicrosoftProvider;
use aphel_kernel::Ctx;
use aphel_sdk::email::oauth as mail_connect;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::{ApiError, app_error};
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::sso::status::{SsoLoginStatus, login_status};
use aphel_sdk::sso::{CallbackQuery, ClientContext, StartQuery, handle_callback, start_login};
use aphel_sdk::tasks::remote_calendar as connect;
use aphel_sdk::tasks::remote_calendar::{CalendarConnectUrl, CalendarStatus};

pub(crate) const TAG: &str = "microsoft";

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MicrosoftStatus {
    pub(crate) login: SsoLoginStatus,
    pub(crate) calendar: CalendarStatus,
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(connect_calendar))
        .routes(routes!(disconnect_calendar))
        .routes(routes!(sync_calendar))
        .routes(routes!(connect_mail))
}

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(callback))
}

#[utoipa::path(
    get,
    path = "/microsoft/status",
    tag = TAG,
    operation_id = "microsoft_status",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Login configuration summary (never the client secret), the callback URL to register at Microsoft Entra ID, and the calendar connection of the current user", body = Data<MicrosoftStatus>),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn status(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<MicrosoftStatus>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(MicrosoftStatus {
        login: login_status(&ctx, &MicrosoftProvider).await?,
        calendar: connect::status(&ctx, &MicrosoftProvider).await?,
    })))
}

#[utoipa::path(
    post,
    path = "/microsoft/calendar/connect",
    tag = TAG,
    operation_id = "microsoft_calendar_connect",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Microsoft consent URL (Calendars.ReadWrite, offline_access) bound to the current user; open it in the browser", body = Data<CalendarConnectUrl>),
        (status = 403, description = "Missing `microsoft.calendar_sync` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Client id or secret not configured (`sso_not_configured`)", body = ErrorEnvelope),
    )
)]
async fn connect_calendar(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<CalendarConnectUrl>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    Ok(Json(Data::new(
        connect::connect_url(&ctx, &MicrosoftProvider).await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/microsoft/calendar/connection",
    tag = TAG,
    operation_id = "microsoft_calendar_disconnect",
    security(("session_cookie" = [])),
    responses(
        (status = 204, description = "Removes the stored refresh token and the connection of the current user (the meetings already synced stay in the CRM). Publishes `microsoft.calendar_disconnected`"),
        (status = 404, description = "No calendar connected (`calendar_not_connected`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn disconnect_calendar(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    connect::disconnect(&ctx, &MicrosoftProvider).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/microsoft/calendar/sync",
    tag = TAG,
    operation_id = "microsoft_calendar_sync",
    security(("session_cookie" = [])),
    responses(
        (status = 202, description = "Queues a synchronization of the current user's calendar"),
        (status = 403, description = "Missing `microsoft.calendar_sync` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "No calendar connected (`calendar_not_connected`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn sync_calendar(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    connect::sync_now(&ctx, &MicrosoftProvider).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/microsoft/mail/connect",
    tag = TAG,
    operation_id = "microsoft_mail_connect",
    security(("session_cookie" = [])),
    responses(
        (status = 302, description = "Browser navigation (not fetch): redirects to the Microsoft consent screen for the Outlook mailbox of the current user (Mail.Read, Mail.Send, User.Read, offline_access). The callback exchanges the code and creates the mailbox through `POST /api/v1/mailboxes` as the user (provider `microsoft.outlook`), then redirects to `/account/mailboxes?connected=microsoft.outlook` or `/account/mailboxes?error=<code>` (`sso_not_configured`, `module_disabled`, `access_denied`, `mail_no_refresh_token`, `token_exchange_failed`, `provider_unavailable` or the code answered by the mailbox API)"),
    )
)]
async fn connect_mail(State(state): State<AppState>, tenant: AuthContext) -> Response {
    match Ctx::from_state(&state, &tenant.scope).await {
        Ok(ctx) => mail_connect::start(&ctx, &MicrosoftProvider).await,
        Err(error) => app_error(error).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/public/microsoft/login",
    tag = TAG,
    operation_id = "microsoft_public_login",
    params(
        StartQuery,
    ),
    responses(
        (status = 302, description = "Redirects to Microsoft (authorization code + PKCE). `login_hint` (the e-mail typed on the login screen) is optional and forwarded to the provider; `redirect` is accepted only as a relative path. On failure redirects to `/login?sso_error=<code>` (`no_account`, `sso_disabled`, `sso_not_configured`, `domain_not_allowed`, `invalid_request`, `rate_limited`)"),
    )
)]
async fn login(
    State(state): State<AppState>,
    client: ClientInfo,
    Query(query): Query<StartQuery>,
) -> Response {
    if let Err(error) = state
        .rate_limits
        .sso
        .check(
            client.0.ip.as_deref().unwrap_or("unknown"),
            "too many sign-in attempts",
        )
        .await
    {
        return app_error(error).into_response();
    }
    let scope = AccessScope::system();
    match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => start_login(&ctx, &MicrosoftProvider, query, ClientContext::new(client)).await,
        Err(error) => app_error(error).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/public/microsoft/callback",
    tag = TAG,
    operation_id = "microsoft_public_callback",
    params(CallbackQuery),
    responses(
        (status = 302, description = "OAuth callback for both login and calendar connection (register this URL at Microsoft Entra ID). Login: validates the single-use `state`, the id token (`iss`, `aud`, `exp`, `nonce`) and `email_verified`, signs in the active member and redirects with the session cookie (or to `/login/two-factor`); failures go to `/login?sso_error=<code>`. Calendar: stores the refresh token encrypted and redirects to `/microsoft?calendar=connected` (or `?calendar_error=<code>`). Mailbox: creates the Outlook mailbox and redirects to `/account/mailboxes?connected=microsoft.outlook` (or `?error=<code>`)"),
    )
)]
async fn callback(
    State(state): State<AppState>,
    client: ClientInfo,
    Query(query): Query<CallbackQuery>,
) -> Response {
    if let Err(error) = state
        .rate_limits
        .sso
        .check(
            client.0.ip.as_deref().unwrap_or("unknown"),
            "too many sign-in attempts",
        )
        .await
    {
        return app_error(error).into_response();
    }
    let scope = AccessScope::system();
    match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => {
            handle_callback(&ctx, &MicrosoftProvider, query, ClientContext::new(client)).await
        }
        Err(error) => app_error(error).into_response(),
    }
}
