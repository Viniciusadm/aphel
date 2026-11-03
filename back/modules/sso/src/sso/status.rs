use serde::Serialize;
use utoipa::ToSchema;

use super::login::{IdentityProvider, callback_url};
use super::settings::{self, LoginSettings};
use crate::support;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SsoLoginStatus {
    pub(crate) provider: String,
    pub(crate) label: String,
    #[schema(example = "https://crm.example.com/api/v1/public/google/callback")]
    pub(crate) callback_url: String,
    pub(crate) configured: bool,
    pub(crate) domains: Vec<String>,
    pub(crate) require_sso: bool,
}

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
