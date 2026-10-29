use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use sha2::{Digest, Sha256};

use crate::errors::{AppError, FieldViolation};

pub const IDEMPOTENCY_KEY_REUSED: &str = "idempotency_key_reused";
pub const IDEMPOTENCY_KEY_IN_PROGRESS: &str = "idempotency_key_in_progress";
pub const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 255;
pub const MAX_STORED_RESPONSE_BYTES: usize = 1024 * 1024;
pub const IDEMPOTENCY_RETENTION: Duration = Duration::from_secs(24 * 3600);
pub const IDEMPOTENCY_IN_PROGRESS_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let key = raw.trim();
        let valid = !key.is_empty()
            && key.len() <= MAX_IDEMPOTENCY_KEY_LENGTH
            && key.bytes().all(|byte| (0x20..=0x7e).contains(&byte));
        if valid {
            Ok(Self(key.to_owned()))
        } else {
            Err(AppError::invalid_fields(
                "invalid Idempotency-Key header",
                vec![FieldViolation::new(
                    "Idempotency-Key",
                    format!(
                        "must have between 1 and {MAX_IDEMPOTENCY_KEY_LENGTH} printable ASCII characters"
                    ),
                )],
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotentRequest {
    pub scope_hash: String,
    pub key: IdempotencyKey,
    pub request_hash: String,
}

impl IdempotentRequest {
    pub fn new(
        scope_hash: impl Into<String>,
        key: IdempotencyKey,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Self {
        Self {
            scope_hash: scope_hash.into(),
            key,
            request_hash: request_fingerprint(method, path, body),
        }
    }
}

pub fn request_fingerprint(method: &str, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.to_ascii_uppercase().as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl StoredResponse {
    pub fn is_storable(status: u16, body_len: usize) -> bool {
        status < 500 && status != 429 && body_len <= MAX_STORED_RESPONSE_BYTES
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeginOutcome {
    Started,
    InProgress,
    Mismatch,
    Replay(StoredResponse),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyDecision {
    Proceed,
    Replay(StoredResponse),
}

#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    async fn begin(
        &self,
        request: &IdempotentRequest,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        stale_before: DateTime<Utc>,
    ) -> Result<BeginOutcome, AppError>;
    async fn complete(
        &self,
        request: &IdempotentRequest,
        response: &StoredResponse,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn release(&self, request: &IdempotentRequest) -> Result<(), AppError>;
    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, AppError>;
}

pub struct IdempotencyUseCases<S> {
    store: S,
}

impl<S> IdempotencyUseCases<S>
where
    S: IdempotencyStore,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn begin(
        &self,
        request: &IdempotentRequest,
        now: DateTime<Utc>,
    ) -> Result<IdempotencyDecision, AppError> {
        let expires_at = now + delta(IDEMPOTENCY_RETENTION);
        let stale_before = now - delta(IDEMPOTENCY_IN_PROGRESS_TIMEOUT);
        match self
            .store
            .begin(request, now, expires_at, stale_before)
            .await?
        {
            BeginOutcome::Started => Ok(IdempotencyDecision::Proceed),
            BeginOutcome::Replay(response) => Ok(IdempotencyDecision::Replay(response)),
            BeginOutcome::InProgress => Err(AppError::conflict_code(
                IDEMPOTENCY_KEY_IN_PROGRESS,
                "a request with this Idempotency-Key is still being processed; retry later",
            )),
            BeginOutcome::Mismatch => Err(AppError::validation_code(
                IDEMPOTENCY_KEY_REUSED,
                "this Idempotency-Key was already used with a different request",
            )),
        }
    }

    pub async fn finish(
        &self,
        request: &IdempotentRequest,
        response: Option<StoredResponse>,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        match response {
            Some(response) if StoredResponse::is_storable(response.status, response.body.len()) => {
                self.store.complete(request, &response, now).await
            }
            _ => self.store.release(request).await,
        }
    }

    pub async fn release(&self, request: &IdempotentRequest) -> Result<(), AppError> {
        self.store.release(request).await
    }

    pub async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        self.store.purge_expired(now).await
    }
}

fn delta(duration: Duration) -> TimeDelta {
    TimeDelta::from_std(duration).unwrap_or(TimeDelta::zero())
}
