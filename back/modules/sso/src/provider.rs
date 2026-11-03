use async_trait::async_trait;
use serde_json::Value;

use crate::sso::login::{EMAIL_NOT_VERIFIED, NOT_CONFIGURED, failure};
use crate::sso::oauth::{self, IdTokenClaims, ProviderEndpoints};
use crate::sso::{IdentityProvider, discover_options, settings};
use crate::{BUTTON_LABEL, DEFAULT_LABEL, DISCOVERY_URL, KEY};
use aphel_kernel::api::sso::{LoginOption, LoginProvider};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

pub struct OidcProvider;

#[async_trait]
impl IdentityProvider for OidcProvider {
    fn key(&self) -> &'static str {
        KEY
    }

    fn label(&self, settings: &Value) -> String {
        settings::text(settings, BUTTON_LABEL).unwrap_or_else(|| DEFAULT_LABEL.to_owned())
    }

    async fn endpoints(&self, settings: &Value) -> Result<ProviderEndpoints, AppError> {
        let url = settings::text(settings, DISCOVERY_URL)
            .and_then(|value| oauth::discovery_url(&value))
            .ok_or_else(|| failure(NOT_CONFIGURED, "the identity provider is not configured"))?;
        oauth::discover_cached(&url).await
    }

    fn verified_email(
        &self,
        claims: &IdTokenClaims,
        _settings: &Value,
    ) -> Result<String, AppError> {
        if !claims.flag("email_verified") {
            return Err(failure(
                EMAIL_NOT_VERIFIED,
                "the identity provider did not confirm the e-mail",
            ));
        }
        claims
            .text("email")
            .map(str::to_owned)
            .ok_or_else(|| failure(EMAIL_NOT_VERIFIED, "the id token has no e-mail"))
    }
}

pub struct IdentityLogin<P: IdentityProvider>(pub P);

#[async_trait]
impl<P: IdentityProvider + 'static> LoginProvider for IdentityLogin<P> {
    fn key(&self) -> &'static str {
        self.0.key()
    }

    async fn discover(&self, state: &AppState, email: &str) -> Result<Vec<LoginOption>, AppError> {
        discover_options(state, &self.0, email).await
    }
}
