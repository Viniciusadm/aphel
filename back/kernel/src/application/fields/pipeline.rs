use async_trait::async_trait;
use serde_json::{Map, Value};

use crate::application::fields::WriteMode;
use crate::application::shared::{FieldChange, RecordRef};
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy)]
pub struct RecordWrite<'a> {
    pub record: &'a RecordRef,
    pub mode: WriteMode,
    pub changes: &'a [FieldChange],
    pub values: &'a Value,
}

impl RecordWrite<'_> {
    pub fn changed(&self, field: &str) -> Option<&FieldChange> {
        self.changes.iter().find(|change| change.field == field)
    }

    pub fn changed_fields(&self) -> impl Iterator<Item = &str> {
        self.changes.iter().map(|change| change.field.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDraft {
    pub record: RecordRef,
    pub mode: WriteMode,
    pub values: Map<String, Value>,
    pub custom_fields: Map<String, Value>,
}

impl RecordDraft {
    pub fn new(
        record: RecordRef,
        mode: WriteMode,
        values: &Value,
        custom_fields: Map<String, Value>,
    ) -> Self {
        Self {
            record,
            mode,
            values: values.as_object().cloned().unwrap_or_default(),
            custom_fields,
        }
    }

    pub fn changed_values(&self, before: &Value) -> Map<String, Value> {
        let before = before.as_object();
        self.values
            .iter()
            .filter(|(key, value)| {
                before
                    .and_then(|before| before.get(*key))
                    .unwrap_or(&Value::Null)
                    != *value
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

#[async_trait]
pub trait RecordPipeline: Send + Sync {
    type Tx: Send;

    async fn saving(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        draft: RecordDraft,
    ) -> Result<RecordDraft, AppError>;

    async fn validate(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError>;

    async fn saved(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError>;

    async fn committed(&self, scope: &AccessScope, write: &RecordWrite<'_>);

    async fn deleting(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError>;

    async fn deleted(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError>;

    async fn restored(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<(), AppError>;

    async fn purged(
        &self,
        tx: &mut Self::Tx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<PurgedRecord, AppError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PurgedRecord {
    pub orphan_storage_keys: Vec<String>,
}

pub const RECORD_SAVING_UNSUPPORTED: &str = "record_saving_unsupported";

pub fn saving_unsupported(record: &RecordRef, key: &str) -> AppError {
    AppError::Internal(format!(
        "{RECORD_SAVING_UNSUPPORTED}: {} records do not accept the system field {key} changed while saving",
        record.record_type
    ))
}
