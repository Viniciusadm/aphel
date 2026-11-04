pub use aphel_contract_telephony::*;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelephonyStartRequest {
    pub user_id: Uuid,
    pub to: String,
    pub record: Option<RecordRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelephonyCall {
    pub provider_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelephonyRecordingRequest {
    pub user_id: Uuid,
    pub provider_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelephonyRecording {
    pub url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait TelephonyProvider: Send + Sync {
    fn key(&self) -> &str;

    fn module(&self) -> Option<&str> {
        None
    }

    fn label(&self) -> &str;

    async fn start_call(&self, request: &TelephonyStartRequest) -> Result<TelephonyCall, AppError>;

    async fn recording(
        &self,
        _request: &TelephonyRecordingRequest,
    ) -> Result<Option<TelephonyRecording>, AppError> {
        Ok(None)
    }
}

pub type SharedTelephonyProvider = Arc<dyn TelephonyProvider>;

pub trait TelephonyRouter: Send + Sync {
    fn providers(&self, scope: &AccessScope) -> Vec<SharedTelephonyProvider>;

    fn provider(&self, scope: &AccessScope, key: &str) -> Option<SharedTelephonyProvider> {
        self.providers(scope)
            .into_iter()
            .find(|provider| provider.key() == key)
    }
}
