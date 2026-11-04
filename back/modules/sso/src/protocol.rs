use crate::sso::oauth;
use aphel_sdk::{
    AppError,
    sso::oauth::{ClientCredentials, OAuthProtocol, ProviderEndpoints, TokenResponse},
};
pub(crate) struct Protocol;
#[async_trait::async_trait]
impl OAuthProtocol for Protocol {
    fn http_client(&self) -> Result<reqwest::Client, AppError> {
        oauth::http_client()
    }
    async fn exchange_code(
        &self,
        endpoint: &str,
        credentials: &ClientCredentials,
        code: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> Result<TokenResponse, AppError> {
        oauth::exchange_code(endpoint, credentials, code, redirect_uri, verifier).await
    }
    async fn refresh_access_token(
        &self,
        endpoint: &str,
        credentials: &ClientCredentials,
        refresh_token: &str,
        scope: Option<&str>,
    ) -> Result<TokenResponse, AppError> {
        oauth::refresh_access_token(endpoint, credentials, refresh_token, scope).await
    }
    async fn discover(&self, url: &str) -> Result<ProviderEndpoints, AppError> {
        oauth::discover(url).await
    }
    async fn discover_cached(&self, url: &str) -> Result<ProviderEndpoints, AppError> {
        oauth::discover_cached(url).await
    }
}

#[async_trait::async_trait]
impl aphel_sdk::sso::states::OAuthStates for Protocol {
    async fn create(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        key: &str,
        purpose: &str,
        user_id: Option<uuid::Uuid>,
        redirect_path: Option<&str>,
    ) -> Result<aphel_sdk::sso::states::NewState, AppError> {
        crate::sso::states::create(ctx, key, purpose, user_id, redirect_path).await
    }
    async fn consume(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        key: &str,
        state: &str,
    ) -> Result<Option<aphel_sdk::sso::states::StoredState>, AppError> {
        crate::sso::states::consume(ctx, key, state).await
    }
}
