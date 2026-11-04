pub mod connect;
pub mod http;
pub mod mime;
pub mod tokens;

use futures::StreamExt;

use crate::{MailProviderError, MailProviderErrorKind};
use aphel_kernel::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::{AppState, runtime_state};
use aphel_sdk::sso::IdentityProvider;
use aphel_sdk::sso::errors::{
    is_authentication, is_internal, is_too_many_requests, is_unavailable,
};

pub const MAIL_API_FAILED: &str = "mail_api_failed";
pub(crate) const MAIL_TOKEN_REJECTED: &str = "mail_token_rejected";
pub(crate) const MAIL_ACCESS_DENIED: &str = "mail_access_denied";
pub const MAIL_NOT_FOUND: &str = "mail_not_found";
pub const MAIL_CURSOR_EXPIRED: &str = "mail_cursor_expired";
pub(crate) const MAIL_NOT_AUTHORIZED: &str = "mail_not_authorized";
pub(crate) const MAIL_AUTHORIZATION_REVOKED: &str = "mail_authorization_revoked";
pub(crate) const MAIL_NO_REFRESH_TOKEN: &str = "mail_no_refresh_token";
pub(crate) const MAIL_INVALID_MESSAGE: &str = "mail_invalid_message";
pub(crate) const MAIL_CONNECT_FAILED: &str = "mail_connect_failed";
pub(crate) const ALREADY_CONNECTED: &str = "already_connected";

pub(crate) const FETCH_CONCURRENCY: usize = 4;

pub trait MailOAuth: IdentityProvider {
    fn mail_provider_key(&self) -> &'static str;

    fn mail_scopes(&self) -> &'static [&'static str];

    fn mail_consent_params(&self) -> Vec<(&'static str, String)>;

    fn mail_refresh_scopes(&self) -> Option<&'static str> {
        None
    }
}

pub fn provider_state() -> Result<AppState, MailProviderError> {
    runtime_state().map_err(provider_error)
}

pub async fn provider_ctx<'a>(
    state: &AppState,
    scope: &'a AccessScope,
) -> Result<Ctx<'a>, MailProviderError> {
    Ctx::from_state(state, scope).await.map_err(provider_error)
}

pub fn provider_error(error: AppError) -> MailProviderError {
    let kind = if is_authentication(&error) {
        MailProviderErrorKind::Authentication
    } else if is_unavailable(&error) || is_too_many_requests(&error) {
        MailProviderErrorKind::Connection
    } else if is_internal(&error) {
        MailProviderErrorKind::Internal
    } else {
        MailProviderErrorKind::Rejected
    };
    MailProviderError::new(kind, error.client_message().to_owned())
}

pub async fn fetch_concurrently<T, F, Fut>(ids: Vec<String>, fetch: F) -> Result<Vec<T>, AppError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Option<T>, AppError>>,
{
    let results: Vec<Result<Option<T>, AppError>> =
        futures::stream::iter(ids.into_iter().map(fetch))
            .buffered(FETCH_CONCURRENCY)
            .collect()
            .await;
    let mut items = Vec::with_capacity(results.len());
    for result in results {
        if let Some(item) = result? {
            items.push(item);
        }
    }
    Ok(items)
}
