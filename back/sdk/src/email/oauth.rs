pub use super::{
    ALREADY_CONNECTED, MAIL_ACCESS_DENIED, MAIL_API_FAILED, MAIL_AUTHORIZATION_REVOKED,
    MAIL_CONNECT_FAILED, MAIL_CURSOR_EXPIRED, MAIL_INVALID_MESSAGE, MAIL_NO_REFRESH_TOKEN,
    MAIL_NOT_AUTHORIZED, MAIL_NOT_FOUND, MAIL_TOKEN_REJECTED, PURPOSE_MAIL,
};

use futures::StreamExt;
use serde_json::Value;

use super::mailbox::{MailProviderError, MailProviderErrorKind};
use crate::sso::IdentityProvider;
use crate::sso::errors::{is_authentication, is_internal, is_too_many_requests, is_unavailable};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

pub(crate) const FETCH_CONCURRENCY: usize = 4;

pub trait MailOAuth: IdentityProvider {
    fn mail_provider_key(&self) -> &'static str;

    fn mail_scopes(&self) -> &'static [&'static str];

    fn mail_consent_params(&self) -> Vec<(&'static str, String)>;

    fn mail_refresh_scopes(&self) -> Option<&'static str> {
        None
    }
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

use super::mailbox::{MailboxAccount, MailboxCredentials};
use axum::response::Response;
#[derive(Clone)]
pub struct MailAccess {
    pub token: String,
    pub refreshed: Option<MailboxCredentials>,
}
impl std::fmt::Debug for MailAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailAccess")
            .field("refreshed", &self.refreshed.is_some())
            .finish_non_exhaustive()
    }
}
#[async_trait::async_trait]
pub trait MailboxOAuth: Send + Sync {
    async fn start(&self, ctx: &Ctx<'_>, provider: &str) -> Response;
    async fn complete(
        &self,
        ctx: &Ctx<'_>,
        provider: &str,
        state: &crate::sso::StoredState,
        code: &str,
    ) -> Response;
    async fn acquire(
        &self,
        ctx: &Ctx<'_>,
        provider: &str,
        credentials: &MailboxCredentials,
        account: Option<&MailboxAccount>,
    ) -> Result<MailAccess, AppError>;
    async fn recover(
        &self,
        ctx: &Ctx<'_>,
        provider: &str,
        credentials: &MailboxCredentials,
        account: Option<&MailboxAccount>,
        prior: &MailAccess,
        error: AppError,
    ) -> Result<MailAccess, AppError>;
    async fn json(&self, request: reqwest::RequestBuilder) -> Result<Value, AppError>;
    async fn bytes(&self, request: reqwest::RequestBuilder) -> Result<Vec<u8>, AppError>;
}
fn execution() -> Result<std::sync::Arc<dyn MailboxOAuth>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn MailboxOAuth>()
        .ok_or_else(|| {
            AppError::unavailable_code("mailbox_oauth_unavailable", "mailbox OAuth is not enabled")
        })
}
fn unavailable_page() -> Response {
    crate::sso::login::redirect_to("/account/mailboxes?error=mailbox_oauth_unavailable")
}
pub fn connect_path(module: &str) -> String {
    format!("{}/mail/connect", crate::sso::support::api_path(module))
}
pub async fn start(ctx: &Ctx<'_>, provider: &dyn MailOAuth) -> Response {
    match execution() {
        Ok(service) => service.start(ctx, provider.mail_provider_key()).await,
        Err(_) => unavailable_page(),
    }
}
pub async fn complete(
    ctx: &Ctx<'_>,
    provider: &dyn MailOAuth,
    state: &crate::sso::StoredState,
    code: &str,
) -> Response {
    match execution() {
        Ok(service) => {
            service
                .complete(ctx, provider.mail_provider_key(), state, code)
                .await
        }
        Err(_) => unavailable_page(),
    }
}
pub async fn json(request: reqwest::RequestBuilder) -> Result<Value, AppError> {
    execution()?.json(request).await
}
pub async fn bytes(request: reqwest::RequestBuilder) -> Result<Vec<u8>, AppError> {
    execution()?.bytes(request).await
}
pub fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
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
    run(ctx, provider, credentials, None, operation).await
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
    run(
        ctx,
        provider,
        &account.credentials,
        Some(account),
        operation,
    )
    .await
}
async fn run<T, F, Fut>(
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
    let service = execution()?;
    let lease = service
        .acquire(ctx, provider.mail_provider_key(), credentials, account)
        .await?;
    match operation(lease.token.clone()).await {
        Ok(value) => Ok((value, lease.refreshed)),
        Err(error) => {
            let retry = service
                .recover(
                    ctx,
                    provider.mail_provider_key(),
                    credentials,
                    account,
                    &lease,
                    error,
                )
                .await?;
            let value = operation(retry.token).await?;
            Ok((value, retry.refreshed))
        }
    }
}
