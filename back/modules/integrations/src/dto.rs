use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use super::service::{ConfigureIntegrationCommand, ConnectionTestResult, IntegrationView};
use aphel_kernel::application::integrations::{
    ConfigField, ConfigInput, IntegrationError, IntegrationErrorSource, ProviderKind,
};

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IntegrationFieldResponse {
    #[schema(example = "host")]
    pub(crate) key: String,
    #[serde(rename = "type")]
    #[schema(example = "text")]
    pub(crate) field_type: String,
    pub(crate) required: bool,
    pub(crate) options: Vec<String>,
    #[schema(value_type = Option<Object>)]
    pub(crate) default: Option<Value>,
    pub(crate) min: Option<i64>,
    pub(crate) max: Option<i64>,
}

impl From<&ConfigField> for IntegrationFieldResponse {
    fn from(field: &ConfigField) -> Self {
        Self {
            key: field.key.to_owned(),
            field_type: field.field_type.as_str().to_owned(),
            required: field.required,
            options: field
                .options
                .iter()
                .map(|option| (*option).to_owned())
                .collect(),
            default: field.default.clone(),
            min: field.min,
            max: field.max,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IntegrationErrorResponse {
    #[schema(example = "2026-10-21T13:45:00.000Z")]
    pub(crate) at: String,
    #[schema(example = "test")]
    pub(crate) source: String,
    #[schema(example = "failed to connect to the SMTP server: Connection refused")]
    pub(crate) message: String,
}

impl From<&IntegrationError> for IntegrationErrorResponse {
    fn from(error: &IntegrationError) -> Self {
        let source = match error.source {
            IntegrationErrorSource::Test => "test",
            IntegrationErrorSource::Activation => "activation",
            IntegrationErrorSource::Delivery => "delivery",
        };
        Self {
            at: timestamp(error.at),
            source: source.to_owned(),
            message: error.message.clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IntegrationResponse {
    #[schema(example = "smtp")]
    pub(crate) provider_key: String,
    #[schema(example = "Organization SMTP")]
    pub(crate) name: String,
    pub(crate) description: String,
    #[schema(example = "mail")]
    pub(crate) icon: String,
    #[schema(example = "configurable")]
    pub(crate) kind: String,
    #[schema(example = "/integrations/webhooks")]
    pub(crate) path: Option<String>,
    pub(crate) fields: Vec<IntegrationFieldResponse>,
    pub(crate) id: Option<String>,
    pub(crate) configured: bool,
    #[schema(example = "active")]
    pub(crate) status: String,
    #[schema(value_type = Object)]
    pub(crate) config: Map<String, Value>,
    #[schema(example = json!({ "password": true }))]
    pub(crate) secrets: BTreeMap<String, bool>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_error_at: Option<String>,
    pub(crate) recent_errors: Vec<IntegrationErrorResponse>,
    pub(crate) last_tested_at: Option<String>,
    pub(crate) last_test_ok: Option<bool>,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

impl From<IntegrationView> for IntegrationResponse {
    fn from(view: IntegrationView) -> Self {
        let provider = &view.provider;
        let schema = provider.config_schema();
        let (kind, path) = match provider.kind() {
            ProviderKind::Configurable => ("configurable", None),
            ProviderKind::Shortcut { path } => ("shortcut", Some(path.to_owned())),
        };
        let secrets = schema
            .secret_keys()
            .map(|key| (key.to_owned(), view.configured_secrets.contains(key)))
            .collect();
        let instance = view.instance.as_ref();
        let status = match (provider.kind(), instance) {
            (ProviderKind::Shortcut { .. }, _) => "available",
            (_, None) => "not_configured",
            (_, Some(instance)) => instance.status.as_str(),
        };
        let last_error = instance.and_then(|instance| instance.last_error());
        Self {
            provider_key: provider.key().to_owned(),
            name: provider.name().to_owned(),
            description: provider.description().to_owned(),
            icon: provider.icon().to_owned(),
            kind: kind.to_owned(),
            path,
            fields: schema.fields.iter().map(Into::into).collect(),
            id: instance.map(|instance| instance.id.to_string()),
            configured: instance.is_some(),
            status: status.to_owned(),
            config: instance
                .map(|instance| instance.config.clone())
                .unwrap_or_default(),
            secrets,
            last_error: last_error.map(|error| error.message.clone()),
            last_error_at: last_error.map(|error| timestamp(error.at)),
            recent_errors: instance
                .map(|instance| instance.recent_errors.iter().map(Into::into).collect())
                .unwrap_or_default(),
            last_tested_at: instance.and_then(|instance| instance.last_tested_at.map(timestamp)),
            last_test_ok: instance.and_then(|instance| instance.last_test_ok),
            created_at: instance.map(|instance| timestamp(instance.created_at)),
            updated_at: instance.map(|instance| timestamp(instance.updated_at)),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigureIntegrationRequest {
    #[schema(value_type = Object, example = json!({
        "host": "smtp.example.com",
        "port": 587,
        "security": "starttls",
        "username": "crm@example.com",
        "password": "s3cret",
        "from_email": "crm@example.com",
        "from_name": "Acme CRM"
    }))]
    pub(crate) config: Map<String, Value>,
    #[serde(default)]
    #[schema(example = json!([]))]
    pub(crate) clear_secrets: Vec<String>,
}

impl ConfigureIntegrationRequest {
    pub(crate) fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> ConfigureIntegrationCommand {
        ConfigureIntegrationCommand {
            input: ConfigInput {
                values: self.config,
                clear_secrets: self.clear_secrets,
            },
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestIntegrationRequest {
    #[serde(default)]
    pub(crate) send_test_email: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IntegrationTestResponse {
    pub(crate) ok: bool,
    #[schema(example = "failed to connect to the SMTP server: Connection refused")]
    pub(crate) message: Option<String>,
    pub(crate) tested_at: String,
    pub(crate) sent_to: Option<String>,
    pub(crate) integration: IntegrationResponse,
}

impl IntegrationTestResponse {
    pub(crate) fn new(
        view: IntegrationView,
        result: ConnectionTestResult,
        sent_to: Option<String>,
    ) -> Self {
        Self {
            ok: result.ok,
            message: result.message,
            tested_at: timestamp(result.tested_at),
            sent_to: sent_to.filter(|_| result.ok),
            integration: view.into(),
        }
    }
}
