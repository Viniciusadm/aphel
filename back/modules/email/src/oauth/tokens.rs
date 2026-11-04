use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::Value;

use super::{MAIL_AUTHORIZATION_REVOKED, MAIL_NOT_AUTHORIZED, MAIL_TOKEN_REJECTED, MailOAuth};
use crate::{MailboxAccount, MailboxCredentials};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::sso::errors::error_code;
use aphel_sdk::sso::login::credentials as client_credentials;
use aphel_sdk::sso::oauth::{self, INVALID_GRANT, TokenResponse};
use aphel_sdk::sso::support;

pub(crate) const ACCESS_TOKEN: &str = "access_token";
pub(crate) const REFRESH_TOKEN: &str = "refresh_token";
pub(crate) const EXPIRES_AT: &str = "expires_at";

const DEFAULT_LIFETIME_SECONDS: i64 = 3600;
const EXPIRY_MARGIN_SECONDS: i64 = 120;

pub(crate) fn credentials_from(
    tokens: &TokenResponse,
    previous: Option<&MailboxCredentials>,
) -> MailboxCredentials {
    let mut map = previous
        .map(|credentials| credentials.0.clone())
        .unwrap_or_default();
    map.insert(
        ACCESS_TOKEN.to_owned(),
        Value::String(tokens.access_token.clone()),
    );
    if let Some(refresh_token) = tokens
        .refresh_token
        .as_deref()
        .filter(|token| !token.is_empty())
    {
        map.insert(
            REFRESH_TOKEN.to_owned(),
            Value::String(refresh_token.to_owned()),
        );
    }
    let lifetime = tokens
        .expires_in
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_LIFETIME_SECONDS);
    map.insert(
        EXPIRES_AT.to_owned(),
        Value::String(
            (Utc::now() + Duration::seconds(lifetime)).to_rfc3339_opts(SecondsFormat::Secs, true),
        ),
    );
    MailboxCredentials(map)
}

pub(crate) fn expires_at(credentials: &MailboxCredentials) -> Option<DateTime<Utc>> {
    match credentials.0.get(EXPIRES_AT)? {
        Value::String(text) => DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|instant| instant.with_timezone(&Utc)),
        Value::Number(number) => number
            .as_i64()
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
        _ => None,
    }
}

fn usable_access_token(credentials: &MailboxCredentials) -> Option<String> {
    let token = credentials
        .get(ACCESS_TOKEN)
        .filter(|token| !token.is_empty())?;
    let expires_at = expires_at(credentials)?;
    (expires_at > Utc::now() + Duration::seconds(EXPIRY_MARGIN_SECONDS)).then(|| token.to_owned())
}

fn access_token_of(credentials: &MailboxCredentials) -> Result<String, AppError> {
    credentials
        .get(ACCESS_TOKEN)
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::unauthorized_code(MAIL_NOT_AUTHORIZED, "the mailbox has no access token")
        })
}

pub(crate) async fn refresh(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    credentials: &MailboxCredentials,
) -> Result<MailboxCredentials, AppError> {
    let refresh_token = credentials
        .get(REFRESH_TOKEN)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            AppError::unauthorized_code(
                MAIL_NOT_AUTHORIZED,
                "the mailbox has no refresh token; connect it again",
            )
        })?;
    let key = provider.key();
    let raw = support::settings(ctx, key).await?;
    let client = client_credentials(ctx, key, &raw).await?;
    let endpoints = provider.endpoints(&raw).await?;
    let tokens = oauth::refresh_access_token(
        &endpoints.token_endpoint,
        &client,
        refresh_token,
        provider.mail_refresh_scopes(),
    )
    .await
    .map_err(|error| {
        if error_code(&error) == INVALID_GRANT {
            AppError::unauthorized_code(
                MAIL_AUTHORIZATION_REVOKED,
                "the mailbox authorization was revoked or expired; connect it again",
            )
        } else {
            error
        }
    })?;
    Ok(credentials_from(&tokens, Some(credentials)))
}

pub async fn with_access<T, F, Fut>(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    credentials: &MailboxCredentials,
    operation: F,
) -> Result<(T, Option<MailboxCredentials>), AppError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    run_with_access(ctx, provider, credentials, None, operation).await
}

pub async fn with_account_access<T, F, Fut>(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    account: &MailboxAccount,
    operation: F,
) -> Result<(T, Option<MailboxCredentials>), AppError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    run_with_access(
        ctx,
        provider,
        &account.credentials,
        Some(account),
        operation,
    )
    .await
}

async fn refresh_and_persist(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    credentials: &MailboxCredentials,
    account: Option<&MailboxAccount>,
) -> Result<MailboxCredentials, AppError> {
    let fresh = refresh(ctx, provider, credentials).await?;
    if let Some(account) = account {
        account.persist_refreshed(&fresh).await;
    }
    Ok(fresh)
}

async fn run_with_access<T, F, Fut>(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    credentials: &MailboxCredentials,
    account: Option<&MailboxAccount>,
    operation: F,
) -> Result<(T, Option<MailboxCredentials>), AppError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let (token, refreshed) = match usable_access_token(credentials) {
        Some(token) => (token, None),
        None => {
            let fresh = refresh_and_persist(ctx, provider, credentials, account).await?;
            (access_token_of(&fresh)?, Some(fresh))
        }
    };
    match operation(token).await {
        Err(error) if error_code(&error) == MAIL_TOKEN_REJECTED && refreshed.is_none() => {
            let fresh = refresh_and_persist(ctx, provider, credentials, account).await?;
            let value = operation(access_token_of(&fresh)?).await?;
            Ok((value, Some(fresh)))
        }
        result => result.map(|value| (value, refreshed)),
    }
}
