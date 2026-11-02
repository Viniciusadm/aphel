use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use uuid::Uuid;

use crate::application::push::{
    NewPushSubscription, PushPorts, PushSubscriptionRepository, VapidKeys,
};
use aphel_kernel::application::security::Clock;
use aphel_kernel::domain::{SessionId, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};

pub const MAX_PUSH_ENDPOINT_LENGTH: usize = 1024;
pub const MAX_PUSH_USER_AGENT_LENGTH: usize = 255;
pub const PUSH_PUBLIC_KEY_LENGTH: usize = 65;
pub const PUSH_AUTH_SECRET_LENGTH: usize = 16;

const PUSH_SERVICE_HOSTS: &[&str] = &[
    "fcm.googleapis.com",
    "android.googleapis.com",
    "push.services.mozilla.com",
    "push.apple.com",
    "notify.windows.com",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSubscriptionInput {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub user_agent: Option<String>,
}

pub fn push_endpoint_host(endpoint: &str) -> Option<&str> {
    let rest = endpoint.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = authority.split(':').next()?;
    if host.is_empty() {
        return None;
    }
    Some(host)
}

pub fn is_known_push_service(endpoint: &str) -> bool {
    let Some(host) = push_endpoint_host(endpoint) else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    PUSH_SERVICE_HOSTS.iter().any(|allowed| {
        host == *allowed
            || host
                .strip_suffix(allowed)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

fn decoded_length(value: &str) -> Option<usize> {
    URL_SAFE_NO_PAD
        .decode(value.trim_end_matches('='))
        .ok()
        .map(|bytes| bytes.len())
}

pub fn validate_push_subscription(
    input: &PushSubscriptionInput,
) -> Result<PushSubscriptionInput, AppError> {
    let endpoint = input.endpoint.trim().to_owned();
    let p256dh = input.p256dh.trim().trim_end_matches('=').to_owned();
    let auth = input.auth.trim().trim_end_matches('=').to_owned();
    let mut violations = Vec::new();
    if endpoint.is_empty() || endpoint.len() > MAX_PUSH_ENDPOINT_LENGTH {
        violations.push(FieldViolation::new(
            "endpoint",
            format!("must have between 1 and {MAX_PUSH_ENDPOINT_LENGTH} characters"),
        ));
    } else if !is_known_push_service(&endpoint) {
        violations.push(FieldViolation::new(
            "endpoint",
            "must be an https URL of a known push service",
        ));
    }
    if decoded_length(&p256dh) != Some(PUSH_PUBLIC_KEY_LENGTH) {
        violations.push(FieldViolation::new(
            "keys.p256dh",
            "must be an uncompressed P-256 public key in base64url",
        ));
    }
    if decoded_length(&auth) != Some(PUSH_AUTH_SECRET_LENGTH) {
        violations.push(FieldViolation::new(
            "keys.auth",
            "must be a 16-byte secret in base64url",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid push subscription",
            violations,
        ));
    }
    let user_agent = input
        .user_agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(MAX_PUSH_USER_AGENT_LENGTH).collect());
    Ok(PushSubscriptionInput {
        endpoint,
        p256dh,
        auth,
        user_agent,
    })
}

pub struct PushUseCases<P> {
    ports: P,
}

impl<P> PushUseCases<P>
where
    P: PushPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn public_key(&self) -> Result<String, AppError> {
        self.ports.keys().public_key().await
    }

    pub async fn subscribe(
        &self,
        user_id: &UserId,
        session_id: &SessionId,
        input: PushSubscriptionInput,
    ) -> Result<(), AppError> {
        let input = validate_push_subscription(&input)?;
        self.ports.keys().public_key().await?;
        self.ports
            .subscriptions()
            .upsert(&NewPushSubscription {
                id: Uuid::now_v7(),
                user_id: user_id.clone(),
                session_id: session_id.clone(),
                endpoint: input.endpoint,
                p256dh: input.p256dh,
                auth: input.auth,
                user_agent: input.user_agent,
                now: self.ports.clock().now(),
            })
            .await
    }

    pub async fn unsubscribe(&self, user_id: &UserId, endpoint: &str) -> Result<bool, AppError> {
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return Err(AppError::invalid_fields(
                "invalid push subscription",
                vec![FieldViolation::new("endpoint", "is required")],
            ));
        }
        self.ports.subscriptions().remove(user_id, endpoint).await
    }
}
