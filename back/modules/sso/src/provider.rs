use async_trait::async_trait;
use serde_json::Value;

use crate::sso::login::{EMAIL_NOT_VERIFIED, NOT_CONFIGURED, failure};
use crate::sso::oauth::{self, IdTokenClaims, ProviderEndpoints};
use crate::sso::{IdentityProvider, settings};
use crate::{BUTTON_LABEL, DEFAULT_LABEL, DISCOVERY_URL, KEY};
use aphel_kernel::errors::AppError;

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

pub use aphel_sdk::sso::IdentityLogin;
