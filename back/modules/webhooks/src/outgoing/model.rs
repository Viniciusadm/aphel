use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::shared::RecordType;

pub(crate) use crate::outgoing::{OutboundRequest, OutboundResponse};

pub(crate) const WEBHOOK_ENDPOINT_RECORD_TYPE: RecordType =
    RecordType::from_static("webhooks.webhook_endpoint");
pub(crate) const WEBHOOK_DELIVERY_RECORD_TYPE: RecordType =
    RecordType::from_static("webhooks.webhook_delivery");
pub(crate) const SIGNING_SECRET_OWNER: &str = "webhook_endpoint";
pub(crate) const SIGNING_SECRET_NAME: &str = "signing_secret";
pub(crate) const SIGNING_SECRET_PREFIX: &str = "whsec_";
pub(crate) const ALL_EVENTS: &str = "*";
pub(crate) const TEST_EVENT_TYPE: &str = "webhook.test";

pub(crate) const MAX_ENDPOINTS_PER_ORGANIZATION: u64 = 20;
pub(crate) const MAX_URL_LENGTH: usize = 2048;
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 255;
pub(crate) const MAX_ERROR_LENGTH: usize = 500;
pub(crate) const EVENT_DELIVERY_MAX_ATTEMPTS: u32 = 7;
pub(crate) const MANUAL_DELIVERY_MAX_ATTEMPTS: u32 = 1;

const RETRY_DELAYS_SECONDS: [i64; 6] = [60, 5 * 60, 30 * 60, 2 * 3600, 6 * 3600, 12 * 3600];

pub(crate) fn retry_delay(attempts_made: u32) -> Duration {
    let index = usize::try_from(attempts_made.saturating_sub(1)).unwrap_or(usize::MAX);
    let seconds = RETRY_DELAYS_SECONDS
        .get(index)
        .copied()
        .unwrap_or(RETRY_DELAYS_SECONDS[RETRY_DELAYS_SECONDS.len() - 1]);
    Duration::seconds(seconds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisabledReason {
    Manual,
    Failures,
}

impl DisabledReason {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Failures => "failures",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "failures" => Some(Self::Failures),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EventSubscription {
    All,
    Only(Vec<String>),
}

impl EventSubscription {
    pub(crate) fn includes(&self, event_type: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(types) => types.iter().any(|item| item == event_type),
        }
    }

    pub(crate) fn to_strings(&self) -> Vec<String> {
        match self {
            Self::All => vec![ALL_EVENTS.to_owned()],
            Self::Only(types) => types.clone(),
        }
    }

    pub(crate) fn from_strings(values: Vec<String>) -> Self {
        if values.iter().any(|value| value == ALL_EVENTS) {
            Self::All
        } else {
            Self::Only(values)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebhookEndpoint {
    pub(crate) id: Uuid,
    pub(crate) url: String,
    pub(crate) description: Option<String>,
    pub(crate) events: EventSubscription,
    pub(crate) is_active: bool,
    pub(crate) disabled_reason: Option<DisabledReason>,
    pub(crate) disabled_at: Option<DateTime<Utc>>,
    pub(crate) consecutive_failures: u32,
    pub(crate) last_delivery_at: Option<DateTime<Utc>>,
    pub(crate) last_success_at: Option<DateTime<Utc>>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_by_name: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebhookEndpointView {
    pub(crate) endpoint: WebhookEndpoint,
    pub(crate) secret_configured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryStatus {
    Pending,
    Succeeded,
    Failed,
}

impl DeliveryStatus {
    pub(crate) const ALL: [Self; 3] = [Self::Pending, Self::Succeeded, Self::Failed];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryTrigger {
    Event,
    Test,
    Redelivery,
}

impl DeliveryTrigger {
    pub(crate) const ALL: [Self; 3] = [Self::Event, Self::Test, Self::Redelivery];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Test => "test",
            Self::Redelivery => "redelivery",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|trigger| trigger.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebhookDelivery {
    pub(crate) id: Uuid,
    pub(crate) endpoint_id: Uuid,
    pub(crate) event_id: Uuid,
    pub(crate) event_type: String,
    pub(crate) trigger: DeliveryTrigger,
    pub(crate) dedupe_key: Option<String>,
    pub(crate) redelivery_of: Option<Uuid>,
    pub(crate) body: String,
    pub(crate) status: DeliveryStatus,
    pub(crate) attempts: u32,
    pub(crate) max_attempts: u32,
    pub(crate) next_attempt_at: Option<DateTime<Utc>>,
    pub(crate) last_attempt_at: Option<DateTime<Utc>>,
    pub(crate) response_status: Option<u16>,
    pub(crate) response_body: Option<String>,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) error: Option<String>,
    pub(crate) requested_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

impl WebhookDelivery {
    pub(crate) fn new(
        endpoint_id: Uuid,
        event_id: Uuid,
        event_type: impl Into<String>,
        trigger: DeliveryTrigger,
        body: String,
        requested_by: Option<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        let max_attempts = match trigger {
            DeliveryTrigger::Event => EVENT_DELIVERY_MAX_ATTEMPTS,
            DeliveryTrigger::Test | DeliveryTrigger::Redelivery => MANUAL_DELIVERY_MAX_ATTEMPTS,
        };
        Self {
            id: Uuid::now_v7(),
            endpoint_id,
            event_id,
            event_type: event_type.into(),
            trigger,
            dedupe_key: None,
            redelivery_of: None,
            body,
            status: DeliveryStatus::Pending,
            attempts: 0,
            max_attempts,
            next_attempt_at: Some(now),
            last_attempt_at: None,
            response_status: None,
            response_body: None,
            duration_ms: None,
            error: None,
            requested_by,
            created_at: now,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeliveryAttempt {
    pub(crate) status: DeliveryStatus,
    pub(crate) attempts: u32,
    pub(crate) attempted_at: DateTime<Utc>,
    pub(crate) next_attempt_at: Option<DateTime<Utc>>,
    pub(crate) response_status: Option<u16>,
    pub(crate) response_body: Option<String>,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeliveryListFilter {
    pub(crate) status: Option<DeliveryStatus>,
    pub(crate) offset: u64,
    pub(crate) limit: u64,
}

pub(crate) fn truncate_chars(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
