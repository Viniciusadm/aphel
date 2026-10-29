use std::sync::Arc;

use serde_json::Value;

use crate::KEY;
use crate::provider::{AiProvider, AnthropicProvider, OpenAiCompatibleProvider};
use aphel_kernel::application::secrets::SecretVault;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{module_secret_key, module_settings};

pub(crate) const NOT_CONFIGURED: &str = "ai_not_configured";
pub(crate) const FEATURE_DISABLED: &str = "ai_feature_disabled";

pub(crate) const SETTING_PROVIDER: &str = "provider";
pub(crate) const SETTING_BASE_URL: &str = "base_url";
pub(crate) const SETTING_MODEL: &str = "model";
pub(crate) const SETTING_API_KEY: &str = "api_key";
pub(crate) const SETTING_DAILY_LIMIT: &str = "daily_limit";
pub(crate) const SETTING_USER_HOURLY_LIMIT: &str = "user_hourly_limit";
pub(crate) const SETTING_SEND_PERSONAL_DATA: &str = "send_personal_data";

pub(crate) const DEFAULT_MODEL: &str = "claude-sonnet-5-5";
pub(crate) const DEFAULT_DAILY_LIMIT: u32 = 200;
pub(crate) const DEFAULT_USER_HOURLY_LIMIT: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Feature {
    Summary,
    NextActions,
    Compose,
    Query,
}

impl Feature {
    pub(crate) const ALL: [Feature; 4] = [
        Feature::Summary,
        Feature::NextActions,
        Feature::Compose,
        Feature::Query,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::NextActions => "next_actions",
            Self::Compose => "compose",
            Self::Query => "query",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|feature| feature.as_str() == value)
    }

    pub(crate) fn setting_key(&self) -> &'static str {
        match self {
            Self::Summary => "feature_summary",
            Self::NextActions => "feature_next_actions",
            Self::Compose => "feature_compose",
            Self::Query => "feature_query",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AiSettings {
    pub(crate) provider: String,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) daily_limit: u32,
    pub(crate) user_hourly_limit: u32,
    pub(crate) send_personal_data: bool,
    pub(crate) features: Vec<(Feature, bool)>,
}

fn text(settings: &Value, key: &str) -> String {
    settings
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_owned()
}

fn number(settings: &Value, key: &str, default: u32) -> u32 {
    settings
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| *value >= 1.0)
        .map(|value| value.min(f64::from(u32::MAX)) as u32)
        .unwrap_or(default)
}

fn flag(settings: &Value, key: &str) -> bool {
    settings.get(key).and_then(Value::as_bool).unwrap_or(true)
}

impl AiSettings {
    pub(crate) fn from_value(settings: &Value) -> Self {
        let provider = match text(settings, SETTING_PROVIDER).as_str() {
            OpenAiCompatibleProvider::NAME => OpenAiCompatibleProvider::NAME,
            _ => AnthropicProvider::NAME,
        }
        .to_owned();
        let base_url = match text(settings, SETTING_BASE_URL) {
            url if url.is_empty() && provider == OpenAiCompatibleProvider::NAME => {
                OpenAiCompatibleProvider::DEFAULT_BASE_URL.to_owned()
            }
            url if url.is_empty() => AnthropicProvider::DEFAULT_BASE_URL.to_owned(),
            url => url.trim_end_matches('/').to_owned(),
        };
        Self {
            provider,
            base_url,
            model: text(settings, SETTING_MODEL),
            daily_limit: number(settings, SETTING_DAILY_LIMIT, DEFAULT_DAILY_LIMIT),
            user_hourly_limit: number(
                settings,
                SETTING_USER_HOURLY_LIMIT,
                DEFAULT_USER_HOURLY_LIMIT,
            ),
            send_personal_data: flag(settings, SETTING_SEND_PERSONAL_DATA),
            features: Feature::ALL
                .into_iter()
                .map(|feature| (feature, flag(settings, feature.setting_key())))
                .collect(),
        }
    }

    pub(crate) async fn load(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        Ok(Self::from_value(&module_settings(ctx.pool(), KEY).await?))
    }

    pub(crate) fn feature_enabled(&self, feature: Feature) -> bool {
        self.features
            .iter()
            .any(|(candidate, enabled)| *candidate == feature && *enabled)
    }

    pub(crate) fn require_feature(&self, feature: Feature) -> Result<(), AppError> {
        if self.feature_enabled(feature) {
            Ok(())
        } else {
            Err(AppError::forbidden_code(
                FEATURE_DISABLED,
                format!("the {} feature is turned off", feature.as_str()),
            ))
        }
    }

    pub(crate) fn requires_key(&self) -> bool {
        self.provider == AnthropicProvider::NAME
            || self.base_url == OpenAiCompatibleProvider::DEFAULT_BASE_URL
    }
}

fn not_configured(message: &str) -> AppError {
    AppError::unavailable_code(NOT_CONFIGURED, message)
}

async fn api_key(ctx: &Ctx<'_>) -> Result<Option<String>, AppError> {
    let secret = ctx
        .secrets()?
        .reveal(
            &AccessScope::system(),
            &module_secret_key(KEY, SETTING_API_KEY),
        )
        .await?;
    Ok(secret.map(|secret| secret.expose().to_owned()))
}

pub(crate) fn has_valid_scheme(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

pub(crate) async fn provider(
    ctx: &Ctx<'_>,
    settings: &AiSettings,
) -> Result<Arc<dyn AiProvider>, AppError> {
    if settings.model.is_empty() {
        return Err(not_configured("no AI model is configured"));
    }
    if !has_valid_scheme(&settings.base_url) {
        return Err(not_configured("the AI provider URL must use http or https"));
    }
    let api_key = api_key(ctx).await?.filter(|key| !key.trim().is_empty());
    if settings.requires_key() && api_key.is_none() {
        return Err(not_configured(
            "no API key is configured for the AI provider",
        ));
    }
    Ok(if settings.provider == OpenAiCompatibleProvider::NAME {
        Arc::new(OpenAiCompatibleProvider::new(
            settings.base_url.clone(),
            settings.model.clone(),
            api_key,
        ))
    } else {
        Arc::new(AnthropicProvider::new(
            settings.base_url.clone(),
            settings.model.clone(),
            api_key.unwrap_or_default(),
        ))
    })
}

pub(crate) async fn is_configured(
    ctx: &Ctx<'_>,
    settings: &AiSettings,
) -> Result<(bool, bool), AppError> {
    let key_configured = api_key(ctx)
        .await?
        .is_some_and(|key| !key.trim().is_empty());
    let configured = !settings.model.is_empty()
        && has_valid_scheme(&settings.base_url)
        && (key_configured || !settings.requires_key());
    Ok((configured, key_configured))
}
