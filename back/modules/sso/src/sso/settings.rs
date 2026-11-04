pub use aphel_sdk::sso::settings::*;
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSettings {
    pub(crate) client_id: Option<String>,
    pub domains: Vec<String>,
    pub(crate) require_sso: bool,
}

impl LoginSettings {
    pub fn from_value(settings: &Value) -> Self {
        Self {
            client_id: text(settings, CLIENT_ID),
            domains: text(settings, DOMAINS)
                .map(|value| parse_domains(&value))
                .unwrap_or_default(),
            require_sso: flag(settings, REQUIRE_SSO),
        }
    }

    pub fn matches_email(&self, email: &str) -> bool {
        email_domain(email).is_some_and(|domain| self.domains.contains(&domain))
    }
}
