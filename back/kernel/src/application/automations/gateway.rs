use std::sync::Arc;

use async_trait::async_trait;
use chrono_tz::Tz;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application::fields::FieldSet;
use crate::application::organization_admin::Listing;
use crate::application::query::{Filter, RecordQuery};
use crate::application::secrets::SecretKey;
use crate::application::shared::{EventCausation, RecordRef};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::state::AppState;

pub const AUTOMATION_MESSAGE: &str = "automation.message";

#[derive(Debug, Clone, PartialEq)]
pub struct TaskSpec {
    pub title: String,
    pub description: Option<String>,
    pub activity_type_id: Option<String>,
    pub owner_id: Option<String>,
    pub due_at: Option<String>,
    pub priority: Option<String>,
    pub record: Option<RecordRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageChannel {
    InApp,
    Email,
}

impl MessageChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageSpec {
    pub automation_id: Uuid,
    pub run_id: Uuid,
    pub record: Option<RecordRef>,
    pub recipients: Vec<UserId>,
    pub channel: MessageChannel,
    pub title: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WebhookCall {
    pub url: String,
    pub delivery_id: Uuid,
    pub event_type: String,
    pub body: String,
    pub signing_secret: Option<String>,
    pub method: crate::application::webhooks::outgoing::WebhookMethod,
    pub headers: Vec<(String, String)>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WebhookCallResult {
    pub status: Option<u16>,
    pub duration_ms: u32,
    pub error: Option<String>,
}

impl WebhookCallResult {
    pub fn succeeded(&self) -> bool {
        self.error.is_none()
            && self
                .status
                .is_some_and(|status| (200..300).contains(&status))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference<'a> {
    pub kind: aphel_contract::ReferenceKind,
    pub id: &'a str,
}

impl<'a> Reference<'a> {
    pub const fn new(kind: aphel_contract::ReferenceKind, id: &'a str) -> Self {
        Self { kind, id }
    }
}

pub const USER: aphel_contract::ReferenceKind = aphel_contract::reference!("iam.user");

#[async_trait]
pub trait AutomationGateway: Send + Sync {
    async fn timezone(&self) -> Result<Tz, AppError>;
    async fn fields(&self, scope: &AccessScope, record_type: &str) -> Result<FieldSet, AppError>;
    async fn relation_keys(&self, record_type: &str) -> Result<Vec<String>, AppError>;
    async fn load_record(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<Option<Value>, AppError>;
    async fn record_matches(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        filter: &Filter,
    ) -> Result<bool, AppError>;
    async fn validate_filter(
        &self,
        scope: &AccessScope,
        record_type: &str,
        filter: &Filter,
    ) -> Result<(), AppError>;
    async fn reference_exists(
        &self,
        scope: &AccessScope,
        reference: Reference<'_>,
    ) -> Result<bool, AppError>;
    async fn create_task(&self, scope: &AccessScope, task: TaskSpec) -> Result<Uuid, AppError>;
    async fn update_record(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        values: Map<String, Value>,
    ) -> Result<(), AppError>;
    async fn call_operation(
        &self,
        scope: &AccessScope,
        operation: &str,
        input: Value,
    ) -> Result<Value, AppError>;
    async fn move_stage(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        stage_id: &str,
    ) -> Result<(), AppError>;
    async fn create_note(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        body: &str,
    ) -> Result<Uuid, AppError>;
    async fn send_message(&self, scope: &AccessScope, message: MessageSpec)
    -> Result<(), AppError>;
    async fn call_webhook(&self, call: WebhookCall) -> Result<WebhookCallResult, AppError>;
    async fn create_record(
        &self,
        scope: &AccessScope,
        record_type: &str,
        values: Map<String, Value>,
    ) -> Result<Uuid, AppError>;
    async fn link(
        &self,
        scope: &AccessScope,
        from: &RecordRef,
        relation: &str,
        target: Uuid,
    ) -> Result<(), AppError>;
    async fn reveal_secret(
        &self,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<Option<String>, AppError>;
    async fn find_records(
        &self,
        scope: &AccessScope,
        record_type: &str,
        query: RecordQuery,
    ) -> Result<Listing<Uuid>, AppError>;
    fn state(&self) -> &AppState;
}

pub type SharedAutomationGateway = Arc<dyn AutomationGateway>;

pub trait AutomationGatewayFactory: Send + Sync {
    fn gateway(&self, causation: Option<EventCausation>) -> SharedAutomationGateway;
}

impl<F> AutomationGatewayFactory for Arc<F>
where
    F: AutomationGatewayFactory + ?Sized,
{
    fn gateway(&self, causation: Option<EventCausation>) -> SharedAutomationGateway {
        (**self).gateway(causation)
    }
}
