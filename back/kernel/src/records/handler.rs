use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::application::shared::RecordType;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, CodedKind};
use crate::state::AppState;

pub const RECORD_OPERATION_NOT_SUPPORTED: &str = "record_operation_not_supported";

pub struct RecordCtx<'a> {
    pub scope: &'a AccessScope,
    pub state: &'a AppState,
}

impl<'a> RecordCtx<'a> {
    pub fn new(scope: &'a AccessScope, state: &'a AppState) -> Self {
        Self { scope, state }
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

pub fn optional_record<T>(result: Result<T, AppError>) -> Result<Option<T>, AppError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn values_request<T: DeserializeOwned>(body: Value) -> Result<T, AppError> {
    serde_json::from_value(body)
        .map_err(|error| AppError::Validation(format!("invalid field values: {error}")))
}

pub fn operation_not_supported(record_type: &str, operation: &str) -> AppError {
    AppError::validation_code(
        RECORD_OPERATION_NOT_SUPPORTED,
        format!("{record_type} records do not support {operation}"),
    )
}

#[async_trait]
pub trait RecordTypeHandler: Send + Sync {
    fn record_type(&self) -> &str;

    fn durable_mutations(&self) -> bool {
        false
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError>;

    async fn update(&self, ctx: &RecordCtx<'_>, id: Uuid, values: Value)
    -> Result<Value, AppError>;

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError>;

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError>;

    async fn archive(&self, _ctx: &RecordCtx<'_>, _id: Uuid) -> Result<(), AppError> {
        Err(operation_not_supported(self.record_type(), "archive"))
    }

    async fn restore(&self, _ctx: &RecordCtx<'_>, _id: Uuid) -> Result<(), AppError> {
        Err(operation_not_supported(self.record_type(), "restore"))
    }
}

#[derive(Clone)]
pub struct RecordHandler(Arc<dyn RecordTypeHandler>);

impl RecordHandler {
    pub fn new(handler: Arc<dyn RecordTypeHandler>) -> Self {
        Self(handler)
    }

    pub fn get(&self) -> &Arc<dyn RecordTypeHandler> {
        &self.0
    }
}

impl fmt::Debug for RecordHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RecordHandler")
            .field(&self.0.record_type())
            .finish()
    }
}

impl PartialEq for RecordHandler {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub type HandlerFactory = Arc<dyn Fn(&RecordType) -> Arc<dyn RecordTypeHandler> + Send + Sync>;

#[derive(Clone)]
pub struct RecordHandlerFactory(HandlerFactory);

impl RecordHandlerFactory {
    pub fn new(factory: HandlerFactory) -> Self {
        Self(factory)
    }

    pub fn handler(&self, record_type: &RecordType) -> RecordHandler {
        RecordHandler::new((self.0)(record_type))
    }
}

impl fmt::Debug for RecordHandlerFactory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RecordHandlerFactory")
    }
}

impl PartialEq for RecordHandlerFactory {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
