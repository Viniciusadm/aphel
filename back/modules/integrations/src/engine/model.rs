use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::application::secrets::SecretKey;

pub const INTEGRATION_SECRET_OWNER: &str = "integration";

pub fn secret_key(integration_id: Uuid, name: &str) -> SecretKey {
    SecretKey::new(INTEGRATION_SECRET_OWNER, integration_id.to_string(), name)
}

pub const MAX_RECENT_ERRORS: usize = 5;
pub const MAX_ERROR_LENGTH: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationStatus {
    Inactive,
    Active,
    Error,
}

impl IntegrationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Active => "active",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "inactive" => Some(Self::Inactive),
            "active" => Some(Self::Active),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Active | Self::Error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationErrorSource {
    Test,
    Activation,
    Delivery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationError {
    pub at: DateTime<Utc>,
    pub source: IntegrationErrorSource,
    pub message: String,
}

impl IntegrationError {
    pub fn new(at: DateTime<Utc>, source: IntegrationErrorSource, message: &str) -> Self {
        Self {
            at,
            source,
            message: truncate_error(message),
        }
    }
}

pub fn truncate_error(message: &str) -> String {
    message.chars().take(MAX_ERROR_LENGTH).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Integration {
    pub id: Uuid,
    pub provider_key: String,
    pub config: Map<String, Value>,
    pub status: IntegrationStatus,
    pub recent_errors: Vec<IntegrationError>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_test_ok: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Integration {
    pub fn new(provider_key: &str, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::now_v7(),
            provider_key: provider_key.to_owned(),
            config: Map::new(),
            status: IntegrationStatus::Inactive,
            recent_errors: Vec::new(),
            last_tested_at: None,
            last_test_ok: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn last_error(&self) -> Option<&IntegrationError> {
        self.recent_errors.first()
    }

    pub fn push_error(&mut self, error: IntegrationError) {
        self.recent_errors.insert(0, error);
        self.recent_errors.truncate(MAX_RECENT_ERRORS);
    }
}
