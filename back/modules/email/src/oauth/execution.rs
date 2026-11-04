use super::{connect, http, tokens};
use aphel_sdk::{
    AppError, Ctx,
    email::{
        mailbox::{MailboxAccount, MailboxCredentials},
        oauth::{MailAccess, MailOAuth, MailboxOAuth},
    },
};
use axum::response::Response;
use reqwest::RequestBuilder;
use std::sync::Arc;
pub(crate) struct Execution;
fn required(ctx: &Ctx<'_>, key: &str) -> Result<Arc<dyn MailOAuth>, AppError> {
    let sources = ctx.app().all_contributions::<dyn MailOAuth>();
    let mut matching = sources.into_iter().filter(|(owner, provider)| {
        ctx.app().is_enabled(owner) && provider.mail_provider_key() == key
    });
    let Some((_, provider)) = matching.next() else {
        return Err(AppError::unavailable_code(
            "mail_oauth_provider_unavailable",
            "mail OAuth provider is not enabled",
        ));
    };
    if matching.next().is_some() {
        return Err(AppError::Configuration(format!(
            "ambiguous mail OAuth provider {key}"
        )));
    }
    Ok(provider)
}
fn unavailable() -> Response {
    aphel_sdk::sso::login::redirect_to("/account/mailboxes?error=mail_oauth_provider_unavailable")
}
#[async_trait::async_trait]
impl MailboxOAuth for Execution {
    async fn start(&self, ctx: &Ctx<'_>, key: &str) -> Response {
        match required(ctx, key) {
            Ok(provider) => connect::start(ctx, provider.as_ref()).await,
            Err(_) => unavailable(),
        }
    }
    async fn complete(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        state: &aphel_sdk::sso::StoredState,
        code: &str,
    ) -> Response {
        match required(ctx, key) {
            Ok(provider) => connect::complete(ctx, provider.as_ref(), state, code).await,
            Err(_) => unavailable(),
        }
    }
    async fn acquire(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        credentials: &MailboxCredentials,
        account: Option<&MailboxAccount>,
    ) -> Result<MailAccess, AppError> {
        tokens::acquire(ctx, required(ctx, key)?.as_ref(), credentials, account).await
    }
    async fn recover(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
        credentials: &MailboxCredentials,
        account: Option<&MailboxAccount>,
        prior: &MailAccess,
        error: AppError,
    ) -> Result<MailAccess, AppError> {
        tokens::recover(
            ctx,
            required(ctx, key)?.as_ref(),
            credentials,
            account,
            prior,
            error,
        )
        .await
    }
    async fn json(&self, request: RequestBuilder) -> Result<serde_json::Value, AppError> {
        http::json(request).await
    }
    async fn bytes(&self, request: RequestBuilder) -> Result<Vec<u8>, AppError> {
        http::bytes(request).await
    }
}
