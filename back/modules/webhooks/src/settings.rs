use aphel_kernel::config::OutboundHttpConfig;
use aphel_kernel::errors::FieldViolation;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfig, OrganizationConfigExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookDeliverySettings {
    pub require_https: bool,
    pub timeout_secs: u64,
    pub disable_after_failures: u32,
    pub retention_days: u32,
}

impl Default for WebhookDeliverySettings {
    fn default() -> Self {
        let base = OutboundHttpConfig::default();
        Self {
            require_https: base.require_https,
            timeout_secs: base.timeout.as_secs(),
            disable_after_failures: 15,
            retention_days: 30,
        }
    }
}

pub fn delivery_settings(config: &OrganizationConfig) -> WebhookDeliverySettings {
    config
        .extensions
        .get("webhooks")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}
pub struct DeliveryConfiguration;
impl OrganizationConfigExtension for DeliveryConfiguration {
    fn defaults(&self) -> (String, Value) {
        (
            "webhooks".to_owned(),
            serde_json::to_value(WebhookDeliverySettings::default())
                .expect("webhook delivery settings"),
        )
    }
    fn validate(&self, values: &std::collections::BTreeMap<String, Value>) -> Vec<FieldViolation> {
        let Some(value) = values.get("webhooks") else {
            return Vec::new();
        };
        let Ok(settings) = serde_json::from_value::<WebhookDeliverySettings>(value.clone()) else {
            return vec![FieldViolation::new(
                "webhooks",
                "invalid webhook delivery configuration",
            )];
        };
        let mut errors = Vec::new();
        for (key, value, min, max) in [
            ("timeout_secs", settings.timeout_secs, 1, 120),
            (
                "disable_after_failures",
                settings.disable_after_failures as u64,
                1,
                1000,
            ),
            ("retention_days", settings.retention_days as u64, 1, 3650),
        ] {
            if !(min..=max).contains(&value) {
                errors.push(FieldViolation::new(
                    format!("webhooks.{key}"),
                    format!("must be between {min} and {max}"),
                ));
            }
        }
        errors
    }
}
impl aphel_sdk::platform::application::outbound_http::OutboundHttpPolicy for DeliveryConfiguration {
    fn configure(
        &self,
        config: &OrganizationConfig,
        base: &OutboundHttpConfig,
    ) -> OutboundHttpConfig {
        let settings = delivery_settings(config);
        OutboundHttpConfig {
            allow_private_networks: base.allow_private_networks,
            require_https: settings.require_https,
            timeout: std::time::Duration::from_secs(settings.timeout_secs),
        }
    }
}
