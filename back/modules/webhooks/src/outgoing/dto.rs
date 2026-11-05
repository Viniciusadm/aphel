use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::outgoing::{
    CreateWebhookEndpointCommand, UpdateWebhookEndpointCommand, WebhookDelivery,
    WebhookEndpointView, WebhookEndpointWithSecret,
};
use aphel_sdk::platform::api::dto::common::double_option;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateWebhookEndpointRequest {
    #[schema(example = "https://example.com/aphel/webhooks")]
    pub(crate) url: String,
    #[schema(example = "ERP sync")]
    pub(crate) description: Option<String>,
    #[schema(example = json!(["deal.won", "deal.lost", "contact.created"]))]
    pub(crate) events: Vec<String>,
    #[schema(example = true)]
    pub(crate) is_active: Option<bool>,
}

impl CreateWebhookEndpointRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateWebhookEndpointCommand {
        CreateWebhookEndpointCommand {
            url: self.url,
            description: self.description,
            events: self.events,
            is_active: self.is_active,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateWebhookEndpointRequest {
    #[schema(example = "https://example.com/aphel/webhooks")]
    pub(crate) url: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(example = json!(["*"]))]
    pub(crate) events: Option<Vec<String>>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateWebhookEndpointRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_version: Option<DateTime<Utc>>,
    ) -> UpdateWebhookEndpointCommand {
        UpdateWebhookEndpointCommand {
            url: self.url,
            description: self.description,
            events: self.events,
            is_active: self.is_active,
            expected_version,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WebhookEndpointResponse {
    #[schema(example = "0192a1b2-0000-7000-8000-000000000001")]
    pub(crate) id: String,
    #[schema(example = "https://example.com/aphel/webhooks")]
    pub(crate) url: String,
    pub(crate) description: Option<String>,
    #[schema(example = json!(["deal.won", "deal.lost"]))]
    pub(crate) events: Vec<String>,
    pub(crate) is_active: bool,
    #[schema(example = "failures")]
    pub(crate) disabled_reason: Option<String>,
    pub(crate) disabled_at: Option<String>,
    #[schema(example = 0)]
    pub(crate) consecutive_failures: u32,
    pub(crate) secret_configured: bool,
    pub(crate) last_delivery_at: Option<String>,
    pub(crate) last_success_at: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_by_name: Option<String>,
    #[schema(example = "2026-10-21T13:45:00.000Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-21T13:45:00.000Z")]
    pub(crate) updated_at: String,
}

impl From<WebhookEndpointView> for WebhookEndpointResponse {
    fn from(view: WebhookEndpointView) -> Self {
        let endpoint = view.endpoint;
        Self {
            id: endpoint.id.to_string(),
            url: endpoint.url,
            description: endpoint.description,
            events: endpoint.events.to_strings(),
            is_active: endpoint.is_active,
            disabled_reason: endpoint
                .disabled_reason
                .map(|reason| reason.as_str().to_owned()),
            disabled_at: endpoint.disabled_at.map(timestamp),
            consecutive_failures: endpoint.consecutive_failures,
            secret_configured: view.secret_configured,
            last_delivery_at: endpoint.last_delivery_at.map(timestamp),
            last_success_at: endpoint.last_success_at.map(timestamp),
            created_by: endpoint.created_by.map(|user| user.as_str().to_owned()),
            created_by_name: endpoint.created_by_name,
            created_at: timestamp(endpoint.created_at),
            updated_at: timestamp(endpoint.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WebhookEndpointWithSecretResponse {
    #[serde(flatten)]
    pub(crate) endpoint: WebhookEndpointResponse,
    #[schema(example = "whsec_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a")]
    pub(crate) signing_secret: String,
}

impl From<WebhookEndpointWithSecret> for WebhookEndpointWithSecretResponse {
    fn from(created: WebhookEndpointWithSecret) -> Self {
        Self {
            signing_secret: created.secret.expose().to_owned(),
            endpoint: created.view.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WebhookDeliveryListParams {
    #[param(example = "failed")]
    pub(crate) status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WebhookDeliveryResponse {
    #[schema(example = "0192a1b2-0000-7000-8000-000000000010")]
    pub(crate) id: String,
    pub(crate) endpoint_id: String,
    #[schema(example = "0192f0c4-8c1e-7d2a-9f3b-1c2d3e4f5a6b")]
    pub(crate) event_id: String,
    #[schema(example = "deal.won")]
    pub(crate) event_type: String,
    #[schema(example = "event")]
    pub(crate) trigger: String,
    pub(crate) redelivery_of: Option<String>,
    #[schema(example = "succeeded")]
    pub(crate) status: String,
    #[schema(example = 1)]
    pub(crate) attempts: u32,
    #[schema(example = 7)]
    pub(crate) max_attempts: u32,
    pub(crate) next_attempt_at: Option<String>,
    pub(crate) last_attempt_at: Option<String>,
    #[schema(example = 200)]
    pub(crate) response_status: Option<u16>,
    #[schema(example = "ok")]
    pub(crate) response_body: Option<String>,
    #[schema(example = 184)]
    pub(crate) duration_ms: Option<u32>,
    pub(crate) error: Option<String>,
    pub(crate) requested_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) completed_at: Option<String>,
}

impl From<WebhookDelivery> for WebhookDeliveryResponse {
    fn from(delivery: WebhookDelivery) -> Self {
        Self {
            id: delivery.id.to_string(),
            endpoint_id: delivery.endpoint_id.to_string(),
            event_id: delivery.event_id.to_string(),
            event_type: delivery.event_type,
            trigger: delivery.trigger.as_str().to_owned(),
            redelivery_of: delivery.redelivery_of.map(|id| id.to_string()),
            status: delivery.status.as_str().to_owned(),
            attempts: delivery.attempts,
            max_attempts: delivery.max_attempts,
            next_attempt_at: delivery.next_attempt_at.map(timestamp),
            last_attempt_at: delivery.last_attempt_at.map(timestamp),
            response_status: delivery.response_status,
            response_body: delivery.response_body,
            duration_ms: delivery.duration_ms,
            error: delivery.error,
            requested_by: delivery.requested_by.map(|user| user.as_str().to_owned()),
            created_at: timestamp(delivery.created_at),
            completed_at: delivery.completed_at.map(timestamp),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WebhookDeliveryDetailResponse {
    #[serde(flatten)]
    pub(crate) delivery: WebhookDeliveryResponse,
    pub(crate) request_body: serde_json::Value,
}

impl From<WebhookDelivery> for WebhookDeliveryDetailResponse {
    fn from(delivery: WebhookDelivery) -> Self {
        let request_body = serde_json::from_str(&delivery.body)
            .unwrap_or_else(|_| serde_json::Value::String(delivery.body.clone()));
        Self {
            request_body,
            delivery: delivery.into(),
        }
    }
}
