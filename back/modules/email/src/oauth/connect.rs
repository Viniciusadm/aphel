use axum::response::Response;
use serde_json::json;

use super::tokens::credentials_from;
use super::{ALREADY_CONNECTED, MAIL_CONNECT_FAILED, MAIL_NO_REFRESH_TOKEN, MailOAuth};
use crate::mailboxes::ConnectMailboxCommand;
use crate::service::mailboxes;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::MODULE_DISABLED;
use aphel_sdk::sso::errors::{error_code, is_conflict, is_unavailable};
use aphel_sdk::sso::login::{
    ACCESS_DENIED, INVALID_STATE, NOT_CONFIGURED, callback_url, credentials, exchange, redirect_to,
};
use aphel_sdk::sso::oauth::{self, AuthorizationRequest};
use aphel_sdk::sso::settings;
use aphel_sdk::sso::states::{self, StoredState};
use aphel_sdk::sso::support;

pub const PURPOSE_MAIL: &str = "mail";
pub(crate) const MAILBOXES_PAGE: &str = "/account/mailboxes";
pub(crate) const PERMISSION_DENIED: &str = "permission_denied";

pub fn connect_path(module: &str) -> String {
    format!("{}/mail/connect", support::api_path(module))
}

fn page_with(key: &str, value: &str) -> String {
    format!(
        "{MAILBOXES_PAGE}?{}",
        form_urlencoded::Serializer::new(String::new())
            .append_pair(key, value)
            .finish()
    )
}

fn error_page(code: &str) -> Response {
    redirect_to(&page_with("error", code))
}

fn redirect_code(error: &AppError) -> &'static str {
    if is_unavailable(error) {
        return oauth::PROVIDER_UNAVAILABLE;
    }
    if is_conflict(error) {
        return ALREADY_CONNECTED;
    }
    let code = error_code(error);
    if code == MODULE_DISABLED {
        return MODULE_DISABLED;
    }
    let safe = !code.is_empty()
        && code.len() <= 64
        && code
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_');
    if safe { code } else { MAIL_CONNECT_FAILED }
}

async fn authorization_url(ctx: &Ctx<'_>, provider: &dyn MailOAuth) -> Result<String, AppError> {
    let key = provider.key();
    let user_id = support::scope_user(ctx.scope)?;
    let raw = support::settings(ctx, key).await?;
    credentials(ctx, key, &raw).await.map_err(|_| {
        AppError::validation_code(NOT_CONFIGURED, "configure the client id and secret first")
    })?;
    let client_id = settings::text(&raw, settings::CLIENT_ID).unwrap_or_default();
    let endpoints = provider.endpoints(&raw).await?;
    let fresh = states::create(ctx, key, PURPOSE_MAIL, Some(user_id), None).await?;
    let mut extra = provider.mail_consent_params();
    if let Some(email) = support::user_email(ctx, user_id).await? {
        extra.push(("login_hint", email));
    }
    let redirect_uri = callback_url(ctx, key);
    Ok(oauth::authorization_url(
        &endpoints.authorization_endpoint,
        &AuthorizationRequest {
            client_id: &client_id,
            redirect_uri: &redirect_uri,
            scopes: provider.mail_scopes(),
            state: &fresh.state,
            nonce: &fresh.nonce,
            code_verifier: &fresh.code_verifier,
            extra,
        },
    ))
}

pub async fn start(ctx: &Ctx<'_>, provider: &dyn MailOAuth) -> Response {
    match authorization_url(ctx, provider).await {
        Ok(url) => redirect_to(&url),
        Err(error) => {
            tracing::info!(
                module = provider.key(),
                code = error_code(&error),
                "mailbox connection start refused"
            );
            error_page(redirect_code(&error))
        }
    }
}

async fn finish(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    state: &StoredState,
    code: &str,
) -> Result<(), AppError> {
    let user_id = state
        .user_id
        .ok_or_else(|| AppError::validation_code(INVALID_STATE, "invalid state"))?;
    let scope = support::scope_for_user(ctx, provider.key(), user_id)
        .await?
        .ok_or_else(|| AppError::forbidden_code(PERMISSION_DENIED, "not a member"))?;
    let user_ctx = ctx.with_scope(&scope);
    let exchanged = exchange(&user_ctx, provider, state, code).await?;
    if exchanged
        .tokens
        .refresh_token
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err(AppError::validation_code(
            MAIL_NO_REFRESH_TOKEN,
            "the provider did not grant offline access",
        ));
    }
    let credentials = credentials_from(&exchanged.tokens, None);
    mailboxes::create(
        &user_ctx,
        ConnectMailboxCommand {
            provider: provider.mail_provider_key().to_owned(),
            settings: Some(json!({})),
            credentials: credentials.0,
            ..ConnectMailboxCommand::default()
        },
    )
    .await
    .map(|_| ())
}

pub async fn complete(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    state: &StoredState,
    code: &str,
) -> Response {
    if code.is_empty() {
        return error_page(ACCESS_DENIED);
    }
    match finish(ctx, provider, state, code).await {
        Ok(()) => redirect_to(&page_with("connected", provider.mail_provider_key())),
        Err(error) => {
            tracing::info!(
                module = provider.key(),
                code = error_code(&error),
                "mailbox connection refused"
            );
            error_page(redirect_code(&error))
        }
    }
}
