pub mod errors;
pub mod http;
mod protocol;
pub(crate) mod provider;
pub mod sso;
pub mod support;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;

use aphel_kernel::api::sso::LoginProvider;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, SettingField, SettingKind};

pub use provider::{IdentityLogin, OidcProvider};

pub use aphel_sdk::sso::KEY;
pub(crate) const DISCOVERY_URL: &str = "discovery_url";
pub(crate) const BUTTON_LABEL: &str = "button_label";
pub(crate) const DEFAULT_LABEL: &str = "Login corporativo";

pub struct SsoModule;

#[async_trait]
impl Module for SsoModule {
    fn manifest(&self) -> Manifest {
        let mut manifest =
            Manifest::new(KEY, "Single sign-on (OIDC)", env!("CARGO_PKG_VERSION"))
                .description(
                    "Single sign-on with any corporate OpenID Connect provider (Okta, Keycloak, Auth0, Entra ID…)",
                )
                .requires_aphel("0.1.0")
                .owned_tables(&["sso_oauth_states"])
                .setting(
                    SettingField::new(DISCOVERY_URL, "Emissor (issuer) ou URL de descoberta", SettingKind::Url)
                        .required()
                        .help("Ex.: https://empresa.okta.com ou https://sso.empresa.com/realms/main/.well-known/openid-configuration"),
                )
                .setting(
                    SettingField::new(BUTTON_LABEL, "Rótulo do botão", SettingKind::Text)
                        .default_value(json!(DEFAULT_LABEL))
                        .help("Texto exibido no botão da tela de login"),
                );
        for field in sso::settings::login_setting_fields() {
            manifest = manifest.setting(field);
        }
        manifest.migration(Migration::new(
            1,
            "create oauth states",
            include_str!("../migrations/0001_oauth_states.sql"),
        ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::sso::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::sso::login::IdentityFlows>(Arc::new(protocol::Protocol));
        app.provide::<dyn aphel_sdk::sso::states::OAuthStates>(Arc::new(protocol::Protocol));
        app.provide::<dyn aphel_sdk::sso::oauth::OAuthProtocol>(Arc::new(protocol::Protocol));
        app.openapi_tag(
            http::TAG,
            "Single sign-on with any corporate OpenID Connect provider",
        )
        .routes(|_| http::router())
        .public_routes(|_| OpenApiRouter::new().merge(http::public_router()))
        .contribute::<dyn LoginProvider>(Arc::new(IdentityLogin(OidcProvider)));
        Ok(())
    }

    fn validate_settings(&self, settings: &Value) -> Vec<FieldViolation> {
        let mut violations = sso::settings::validate_login_settings(settings);
        if let Some(url) = sso::settings::text(settings, DISCOVERY_URL)
            && sso::oauth::discovery_url(&url).is_none()
        {
            violations.push(FieldViolation::new(
                DISCOVERY_URL,
                "use um endereço https:// do emissor",
            ));
        }
        violations
    }
}
