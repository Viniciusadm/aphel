use serde_json::{Value, json};

use aphel_kernel::errors::FieldViolation;
use aphel_kernel::modules::{SettingField, SettingKind};

pub(crate) const CLIENT_ID: &str = "client_id";
pub(crate) const CLIENT_SECRET: &str = "client_secret";
pub(crate) const DOMAINS: &str = "domains";
pub(crate) const REQUIRE_SSO: &str = "require_sso";

pub(crate) const MAX_DOMAINS: usize = 50;

pub fn text(settings: &Value, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(crate) fn flag(settings: &Value, key: &str) -> bool {
    settings.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn parse_domains(value: &str) -> Vec<String> {
    let mut domains: Vec<String> = value
        .split([',', ';', '\n', ' ', '\t'])
        .map(|domain| domain.trim().trim_start_matches('@').to_ascii_lowercase())
        .filter(|domain| !domain.is_empty())
        .collect();
    domains.sort();
    domains.dedup();
    domains
}

pub(crate) fn is_valid_domain(domain: &str) -> bool {
    domain.len() <= 253
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
}

pub(crate) fn email_domain(email: &str) -> Option<String> {
    let (local, domain) = email.trim().rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    Some(domain.to_ascii_lowercase())
}

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

pub fn login_setting_fields() -> Vec<SettingField> {
    vec![
        SettingField::new(CLIENT_ID, "Client ID", SettingKind::Text)
            .required()
            .help("Identificador do aplicativo cadastrado no provedor"),
        SettingField::new(CLIENT_SECRET, "Client secret", SettingKind::Secret)
            .required()
            .help("Segredo do aplicativo; fica cifrado e nunca é exibido de novo"),
        SettingField::new(DOMAINS, "Domínios de e-mail", SettingKind::LongText).help(
            "Domínios que entram pelo login único, separados por vírgula (ex.: empresa.com.br). Vazio = o botão não aparece no login",
        ),
        SettingField::new(REQUIRE_SSO, "Exigir login único", SettingKind::Boolean)
            .default_value(json!(false))
            .help("Recusa o login por senha de e-mails desses domínios"),
    ]
}

pub fn validate_login_settings(settings: &Value) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    let domains = text(settings, DOMAINS)
        .map(|value| parse_domains(&value))
        .unwrap_or_default();
    if domains.len() > MAX_DOMAINS {
        violations.push(FieldViolation::new(
            DOMAINS,
            format!("informe no máximo {MAX_DOMAINS} domínios"),
        ));
    }
    if let Some(invalid) = domains.iter().find(|domain| !is_valid_domain(domain)) {
        violations.push(FieldViolation::new(
            DOMAINS,
            format!("domínio inválido: {invalid}"),
        ));
    }
    if flag(settings, REQUIRE_SSO) && domains.is_empty() {
        violations.push(FieldViolation::new(
            REQUIRE_SSO,
            "informe os domínios antes de exigir o login único",
        ));
    }
    violations
}
