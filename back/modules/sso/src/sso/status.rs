use super::login::{IdentityProvider, callback_url};
use super::settings::{self, LoginSettings};
use crate::support;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

pub use aphel_sdk::sso::status::SsoLoginStatus;

pub async fn login_status(
    ctx: &Ctx<'_>,
    provider: &dyn IdentityProvider,
) -> Result<SsoLoginStatus, AppError> {
    let key = provider.key();
    let raw = support::settings(ctx, key).await?;
    let login = LoginSettings::from_value(&raw);
    let secret_configured = support::secret(ctx, key, settings::CLIENT_SECRET)
        .await?
        .is_some_and(|secret| !secret.trim().is_empty());
    Ok(SsoLoginStatus {
        provider: key.to_owned(),
        label: provider.label(&raw),
        callback_url: callback_url(ctx, key),
        configured: login.client_id.is_some() && secret_configured,
        domains: login.domains,
        require_sso: login.require_sso,
    })
}
