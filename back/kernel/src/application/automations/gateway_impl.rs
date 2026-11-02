use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use chrono_tz::Tz;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::Ctx;
use crate::application::automations::gateway::USER;
use crate::application::automations::{
    AUTOMATION_MESSAGE, AutomationGateway, AutomationGatewayFactory, MessageSpec, NoteOperations,
    Reference, SharedAutomationGateway, StageOperations, TaskOperations, TaskSpec, WebhookCall,
    WebhookCallResult,
};
use crate::application::fields::{FieldSet, RelationPath};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine, RecordQueryExecutor,
    RecordQueryPorts,
};
use crate::application::secrets::{SecretKey, SecretVault};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, CausedAuditLog, CausedEventPublisher, DomainEvent, EventCausation,
    EventPublisher, RecordRef, TransactionManager,
};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, CodedKind};
use crate::modules::ReferenceProvider;
use crate::records::{RecordCtx, field_set, query_ports};
use crate::state::AppState;

pub const AUTOMATION_MESSAGE_SENT: &str = "automation.message_sent";

#[async_trait]
pub trait AutomationOperations: Send + Sync {
    async fn reference_exists(
        &self,
        state: &AppState,
        reference: Reference<'_>,
    ) -> Result<bool, AppError>;
    async fn call_webhook(
        &self,
        state: &AppState,
        call: WebhookCall,
    ) -> Result<WebhookCallResult, AppError>;
    async fn link(
        &self,
        state: &AppState,
        scope: &AccessScope,
        from: &RecordRef,
        relation: &str,
        target: Uuid,
    ) -> Result<(), AppError>;
}

pub type SharedAutomationOperations = Arc<dyn AutomationOperations>;

#[derive(Clone)]
pub struct HandlerAutomationGateway {
    state: AppState,
    operations: SharedAutomationOperations,
}

impl HandlerAutomationGateway {
    pub fn new(state: AppState, operations: SharedAutomationOperations) -> Self {
        Self { state, operations }
    }

    fn service<T>(&self, name: &str) -> Result<Arc<T>, AppError>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.state.app.services().get::<T>().ok_or_else(|| {
            AppError::Validation(format!("{name} are not available in this installation"))
        })
    }

    fn ctx<'a>(&'a self, scope: &'a AccessScope) -> RecordCtx<'a> {
        RecordCtx::new(scope, &self.state)
    }

    async fn body(
        &self,
        scope: &AccessScope,
        record_type: &str,
        values: Map<String, Value>,
    ) -> Result<Value, AppError> {
        let fields = self.fields(scope, record_type).await?;
        let mut body = Map::new();
        let mut custom = Map::new();
        for (key, value) in values {
            match fields.find(&key) {
                Some(field) if field.is_system() => {
                    body.insert(key, value);
                }
                Some(_) => {
                    custom.insert(key, value);
                }
                None => {
                    return Err(AppError::Validation(format!(
                        "{key} is not a field of {record_type} records"
                    )));
                }
            }
        }
        if !custom.is_empty() {
            body.insert("custom_fields".to_owned(), Value::Object(custom));
        }
        Ok(Value::Object(body))
    }

    async fn writable(
        &self,
        record_type: &str,
    ) -> Result<Arc<dyn crate::records::RecordTypeHandler>, AppError> {
        self.state
            .registry()
            .await?
            .handler(record_type)
            .map_err(|_| unsupported(record_type))
    }

    async fn matches(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: Uuid,
        filter: &Filter,
    ) -> Result<bool, AppError> {
        let ports = query_ports(&self.state).await?;
        let query = RecordQuery {
            filter: Some(Filter::and(vec![
                filter.clone(),
                Filter::condition("id", FilterOperator::Eq, json!(record_id.to_string())),
            ])),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
            ..RecordQuery::default()
        };
        Ok(RecordQueryEngine::new(&ports)
            .query(scope, record_type, query)
            .await?
            .total
            > 0)
    }
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

fn unsupported(record_type: &str) -> AppError {
    AppError::Validation(format!(
        "automations cannot change {record_type} records this way"
    ))
}

fn saved_id(record: &Value) -> Result<Uuid, AppError> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal("the saved record has no id".to_owned()))
}

#[async_trait]
impl AutomationGateway for HandlerAutomationGateway {
    async fn timezone(&self) -> Result<Tz, AppError> {
        let ports = query_ports(&self.state).await?;
        ports.executor().timezone().await
    }

    async fn fields(&self, scope: &AccessScope, record_type: &str) -> Result<FieldSet, AppError> {
        field_set(&self.state, scope, record_type).await
    }

    async fn relation_keys(&self, record_type: &str) -> Result<Vec<String>, AppError> {
        let registry = self.state.registry().await?;
        Ok(registry
            .require(record_type)?
            .relations
            .iter()
            .filter(|relation| {
                relation
                    .paths
                    .iter()
                    .any(|path| matches!(path, RelationPath::Relationship { .. }))
            })
            .map(|relation| relation.key.clone())
            .collect())
    }

    async fn load_record(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<Option<Value>, AppError> {
        let registry = self.state.registry().await?;
        let Ok(handler) = registry.handler(record.record_type.as_str()) else {
            return Ok(None);
        };
        match handler.load(&self.ctx(scope), record.record_id).await {
            Ok(value) => Ok(value),
            Err(error) if is_missing(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn record_matches(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        filter: &Filter,
    ) -> Result<bool, AppError> {
        self.matches(scope, record.record_type.as_str(), record.record_id, filter)
            .await
    }

    async fn validate_filter(
        &self,
        scope: &AccessScope,
        record_type: &str,
        filter: &Filter,
    ) -> Result<(), AppError> {
        self.matches(scope, record_type, Uuid::nil(), filter)
            .await
            .map(|_| ())
    }

    async fn reference_exists(
        &self,
        scope: &AccessScope,
        reference: Reference<'_>,
    ) -> Result<bool, AppError> {
        if reference.kind == USER {
            return self
                .operations
                .reference_exists(&self.state, reference)
                .await;
        }
        let ctx = Ctx::from_state(&self.state, scope).await?;
        let Some(provider) = self
            .state
            .app
            .contributions::<dyn ReferenceProvider>()
            .into_iter()
            .find(|provider| provider.kind() == reference.kind)
        else {
            return Ok(false);
        };
        provider.exists(&ctx, reference.id).await
    }

    async fn create_task(&self, scope: &AccessScope, task: TaskSpec) -> Result<Uuid, AppError> {
        self.service::<dyn TaskOperations>("tasks")?
            .create_task(&Ctx::from_state(&self.state, scope).await?, task)
            .await
    }

    async fn update_record(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        values: Map<String, Value>,
    ) -> Result<(), AppError> {
        let record_type = record.record_type.as_str();
        let body = self.body(scope, record_type, values).await?;
        let handler = self.writable(record_type).await?;
        handler
            .update(&self.ctx(scope), record.record_id, body)
            .await
            .map(|_| ())
    }

    async fn call_operation(
        &self,
        scope: &AccessScope,
        operation: &str,
        input: Value,
    ) -> Result<Value, AppError> {
        let entry = self.state.app.operation(operation).ok_or_else(|| {
            AppError::unavailable_code(
                crate::modules::CONTRACT_UNAVAILABLE,
                format!("no enabled provider for operation `{operation}`"),
            )
        })?;
        entry
            .call(&Ctx::from_state(&self.state, scope).await?, input)
            .await
    }

    async fn move_stage(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        stage_id: &str,
    ) -> Result<(), AppError> {
        self.service::<dyn StageOperations>("record stages")?
            .move_to_stage(
                &Ctx::from_state(&self.state, scope).await?,
                record,
                stage_id,
            )
            .await
    }

    async fn create_note(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        body: &str,
    ) -> Result<Uuid, AppError> {
        self.service::<dyn NoteOperations>("notes")?
            .create_note(&Ctx::from_state(&self.state, scope).await?, record, body)
            .await
    }

    async fn send_message(
        &self,
        scope: &AccessScope,
        message: MessageSpec,
    ) -> Result<(), AppError> {
        let state = &self.state;
        let now = Utc::now();
        let actor = Actor::from_scope(scope);
        let payload = json!({
            "automation_id": message.automation_id.to_string(),
            "run_id": message.run_id.to_string(),
            "channel": message.channel.as_str(),
            "recipients": message.recipients.iter().map(UserId::as_str).collect::<Vec<_>>(),
            "title": message.title,
            "body": message.body,
        });
        let mut event =
            DomainEvent::new(actor.clone(), AUTOMATION_MESSAGE, now).with_payload(payload);
        let mut entry = AuditEntry::new(actor, AUTOMATION_MESSAGE_SENT, now);
        if let Some(record) = message.record {
            event = event.with_record(record.clone());
            entry = entry.with_record(record);
        }
        let transactions = state.transactions()?;
        let mut tx = transactions.begin().await?;
        state.events.publish(&mut tx, event).await?;
        state.audit.record(&mut tx, entry).await?;
        transactions.commit(tx).await
    }

    async fn call_webhook(&self, call: WebhookCall) -> Result<WebhookCallResult, AppError> {
        self.operations.call_webhook(&self.state, call).await
    }

    async fn create_record(
        &self,
        scope: &AccessScope,
        record_type: &str,
        values: Map<String, Value>,
    ) -> Result<Uuid, AppError> {
        let body = self.body(scope, record_type, values).await?;
        let handler = self.writable(record_type).await?;
        saved_id(&handler.create(&self.ctx(scope), body).await?)
    }

    async fn link(
        &self,
        scope: &AccessScope,
        from: &RecordRef,
        relation: &str,
        target: Uuid,
    ) -> Result<(), AppError> {
        self.operations
            .link(&self.state, scope, from, relation, target)
            .await
    }

    async fn reveal_secret(
        &self,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<Option<String>, AppError> {
        Ok(self
            .state
            .secrets
            .reveal(scope, key)
            .await?
            .map(|secret| secret.expose().to_owned()))
    }

    async fn find_records(
        &self,
        scope: &AccessScope,
        record_type: &str,
        query: RecordQuery,
    ) -> Result<Listing<Uuid>, AppError> {
        let ports = query_ports(&self.state).await?;
        RecordQueryEngine::new(&ports)
            .query(scope, record_type, query)
            .await
    }

    fn state(&self) -> &AppState {
        &self.state
    }
}

#[derive(Clone)]
pub struct HandlerAutomationGatewayFactory {
    state: AppState,
    operations: SharedAutomationOperations,
}

impl HandlerAutomationGatewayFactory {
    pub fn new(state: AppState, operations: SharedAutomationOperations) -> Self {
        Self { state, operations }
    }

    pub fn shared(
        state: AppState,
        operations: SharedAutomationOperations,
    ) -> Arc<dyn AutomationGatewayFactory> {
        Arc::new(Self::new(state, operations))
    }
}

impl AutomationGatewayFactory for HandlerAutomationGatewayFactory {
    fn gateway(&self, causation: Option<EventCausation>) -> SharedAutomationGateway {
        let state = match causation {
            Some(causation) => self
                .state
                .clone()
                .with_events(Arc::new(CausedEventPublisher::new(
                    self.state.events.clone(),
                    causation.clone(),
                )))
                .with_audit(Arc::new(CausedAuditLog::new(
                    self.state.audit.clone(),
                    causation,
                ))),
            None => self.state.clone(),
        };
        Arc::new(HandlerAutomationGateway::new(
            state,
            self.operations.clone(),
        ))
    }
}
