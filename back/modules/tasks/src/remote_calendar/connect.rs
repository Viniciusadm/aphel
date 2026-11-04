use axum::response::Response;
use chrono::Utc;
use serde_json::{Value, json};

use super::engine::refresh_token_name;
use super::jobs::enqueue_sync;
use super::model::{CalendarApi, CalendarConnectUrl, CalendarStatus, NO_REFRESH_TOKEN};
use super::store;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_kernel::errors::AppError;
use aphel_sdk::sso::IdTokenClaims;
use aphel_sdk::sso::errors::error_code;
use aphel_sdk::sso::login::{
    ACCESS_DENIED, INVALID_STATE, NOT_CONFIGURED, callback_url, credentials, exchange,
    failure_code, redirect_to,
};
use aphel_sdk::sso::oauth::{self, AuthorizationRequest};
use aphel_sdk::sso::settings;
use aphel_sdk::sso::states::{self, StoredState};
const PURPOSE_CALENDAR: &str = "calendar";
use aphel_sdk::sso::support;

pub(crate) const NOT_CONNECTED_CODE: &str = "calendar_not_connected";
pub(crate) const PERMISSION_DENIED: &str = "permission_denied";

fn page_with(api: &dyn CalendarApi, query: &str) -> String {
    format!("{}?{query}", api.spec().page_path())
}

fn error_page(api: &dyn CalendarApi, code: &str) -> Response {
    redirect_to(&page_with(api, &format!("calendar_error={code}")))
}

pub async fn status(ctx: &Ctx<'_>, api: &dyn CalendarApi) -> Result<CalendarStatus, AppError> {
    let spec = api.spec();
    let raw = support::settings(ctx, spec.key).await?;
    let configured = credentials(ctx, spec.key, &raw).await.is_ok();
    let user_id = support::scope_user(ctx.scope)?;
    let connection = store::find_connection(ctx.pool(), spec.key, user_id).await?;
    Ok(CalendarStatus {
        can_connect: support::can(ctx.scope, &spec.permission()),
        configured,
        connection: connection.map(|connection| connection.status()),
    })
}

pub async fn connect_url(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
) -> Result<CalendarConnectUrl, AppError> {
    let spec = api.spec();
    ctx.scope.require(&spec.permission())?;
    let user_id = support::scope_user(ctx.scope)?;
    let raw = support::settings(ctx, spec.key).await?;
    credentials(ctx, spec.key, &raw).await.map_err(|_| {
        AppError::validation_code(NOT_CONFIGURED, "configure the client id and secret first")
    })?;
    let client_id = settings::text(&raw, settings::CLIENT_ID).unwrap_or_default();
    let endpoints = api.endpoints(&raw).await?;
    let fresh = states::create(ctx, spec.key, PURPOSE_CALENDAR, Some(user_id), None).await?;
    let mut extra = api.consent_params();
    if let Some(email) = support::user_email(ctx, user_id).await? {
        extra.push(("login_hint", email));
    }
    let redirect_uri = callback_url(ctx, spec.key);
    Ok(CalendarConnectUrl {
        url: oauth::authorization_url(
            &endpoints.authorization_endpoint,
            &AuthorizationRequest {
                client_id: &client_id,
                redirect_uri: &redirect_uri,
                scopes: api.consent_scopes(),
                state: &fresh.state,
                nonce: &fresh.nonce,
                code_verifier: &fresh.code_verifier,
                extra,
            },
        ),
    })
}

fn account_email(claims: Option<&IdTokenClaims>) -> Option<String> {
    let claims = claims?;
    claims
        .text("email")
        .or_else(|| claims.text("preferred_username"))
        .filter(|value| value.contains('@'))
        .map(str::to_lowercase)
}

async fn finish_connection(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    state: &StoredState,
    code: &str,
) -> Result<(), AppError> {
    let spec = api.spec();
    let user_id = state
        .user_id
        .ok_or_else(|| AppError::validation_code(INVALID_STATE, "invalid state"))?;
    let scope = support::scope_for_user(ctx, spec.key, user_id)
        .await?
        .ok_or_else(|| AppError::forbidden_code(PERMISSION_DENIED, "not a member"))?;
    scope.require(&spec.permission())?;
    let user_ctx = ctx.with_scope(&scope);
    let exchanged = exchange(&user_ctx, api, state, code).await?;
    let refresh_token = exchanged
        .tokens
        .refresh_token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            AppError::validation_code(
                NO_REFRESH_TOKEN,
                "the provider did not grant offline access",
            )
        })?;
    let email = account_email(exchanged.claims.as_ref());
    let before = store::find_connection(ctx.pool(), spec.key, user_id).await?;
    let actor = Actor::from_scope(&scope);
    let mut tx = user_ctx.begin().await?;
    support::store_secret(
        &user_ctx,
        &mut tx,
        spec.key,
        &refresh_token_name(user_id),
        refresh_token,
    )
    .await?;
    store::upsert_connection(&mut tx, spec.key, user_id, email.as_deref()).await?;
    user_ctx
        .events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), spec.connected_event, Utc::now()).with_payload(json!({
                "user_id": user_id,
                "account_email": email,
            })),
        )
        .await?;
    user_ctx
        .audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, spec.connected_event, Utc::now()).with_changes(vec![
                FieldChange::new(
                    "account_email",
                    before
                        .and_then(|connection| connection.account_email)
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                    json!(email),
                ),
                FieldChange::new("refresh_token", Value::Null, json!("[redacted]")),
            ]),
        )
        .await?;
    tx.commit().await?;
    enqueue_sync(ctx, spec.key, user_id).await
}

pub async fn complete_connection(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    state: &StoredState,
    code: &str,
) -> Response {
    if code.is_empty() {
        return error_page(api, ACCESS_DENIED);
    }
    match finish_connection(ctx, api, state, code).await {
        Ok(()) => redirect_to(&page_with(api, "calendar=connected")),
        Err(error) => {
            let code = error_code(&error);
            tracing::info!(module = api.spec().key, code, "calendar connection refused");
            let code = match code {
                NO_REFRESH_TOKEN => NO_REFRESH_TOKEN,
                PERMISSION_DENIED => PERMISSION_DENIED,
                _ => failure_code(&error),
            };
            error_page(api, code)
        }
    }
}

pub async fn disconnect(ctx: &Ctx<'_>, api: &dyn CalendarApi) -> Result<(), AppError> {
    let spec = api.spec();
    let user_id = support::scope_user(ctx.scope)?;
    let Some(connection) = store::find_connection(ctx.pool(), spec.key, user_id).await? else {
        return Err(AppError::not_found_code(
            NOT_CONNECTED_CODE,
            "the calendar is not connected",
        ));
    };
    let actor = Actor::from_scope(ctx.scope);
    let mut tx = ctx.begin().await?;
    support::remove_secret(ctx, &mut tx, spec.key, &refresh_token_name(user_id)).await?;
    store::delete_connection(&mut tx, spec.key, user_id).await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), spec.disconnected_event, Utc::now()).with_payload(
                json!({
                    "user_id": user_id,
                    "account_email": connection.account_email,
                }),
            ),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, spec.disconnected_event, Utc::now()).with_changes(vec![
                FieldChange::new(
                    "account_email",
                    json!(connection.account_email),
                    Value::Null,
                ),
                FieldChange::new("refresh_token", json!("[redacted]"), Value::Null),
            ]),
        )
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn sync_now(ctx: &Ctx<'_>, api: &dyn CalendarApi) -> Result<(), AppError> {
    let spec = api.spec();
    ctx.scope.require(&spec.permission())?;
    let user_id = support::scope_user(ctx.scope)?;
    let exists = store::find_connection(ctx.pool(), spec.key, user_id)
        .await?
        .is_some();
    if !exists {
        return Err(AppError::not_found_code(
            NOT_CONNECTED_CODE,
            "the calendar is not connected",
        ));
    }
    enqueue_sync(ctx, spec.key, user_id).await
}
