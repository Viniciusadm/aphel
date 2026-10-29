use std::sync::Arc;

use serde_json::Value;

use crate::application::fields::{
    FieldSchema, RecordDraft, RecordTypeRegistry, RecordWrite, WriteMode,
};
use crate::application::query::{PlanNode, PlanSort, QueryPlan};
use crate::application::shared::{FieldChange, RecordRef, RecordType};
use crate::modules::Hook;

pub struct RecordSaving;
pub struct RecordValidating;
pub struct RecordSaved;
pub struct RecordCommitted;
pub struct RecordDeleting;
pub struct RecordDeleted;
pub struct RecordPurged;
pub struct RecordRestored;
pub struct RecordQueryFilter;
pub struct RecordSerializing;
pub struct RecordSchemaFilter;
pub struct SearchPlanFilter;

#[derive(Debug, Clone)]
pub struct SavingRecord {
    pub record_type: RecordType,
    pub draft: RecordDraft,
    pub registry: Arc<RecordTypeRegistry>,
}

#[derive(Debug, Clone)]
pub struct ChangedRecord {
    pub record_type: RecordType,
    pub record: RecordRef,
    pub mode: WriteMode,
    pub changes: Vec<FieldChange>,
    pub values: Value,
    pub registry: Arc<RecordTypeRegistry>,
}

impl ChangedRecord {
    pub fn new(write: &RecordWrite<'_>, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            record_type: write.record.record_type.clone(),
            record: write.record.clone(),
            mode: write.mode,
            changes: write.changes.to_vec(),
            values: write.values.clone(),
            registry,
        }
    }

    pub fn write(&self) -> RecordWrite<'_> {
        RecordWrite {
            record: &self.record,
            mode: self.mode,
            changes: &self.changes,
            values: &self.values,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemovedRecord {
    pub record_type: RecordType,
    pub record: RecordRef,
    pub registry: Arc<RecordTypeRegistry>,
}

#[derive(Debug, Clone)]
pub struct QueryConstraints {
    pub record_type: RecordType,
    pub conditions: Vec<PlanNode>,
    pub sort: Vec<PlanSort>,
}

#[derive(Debug, Clone)]
pub struct SerializedRecords {
    pub record_type: RecordType,
    pub items: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct RecordSchema {
    pub record_type: RecordType,
    pub schema: FieldSchema,
}

impl Hook for RecordSaving {
    type Args = SavingRecord;
}

impl Hook for RecordValidating {
    type Args = ChangedRecord;
}

impl Hook for RecordSaved {
    type Args = ChangedRecord;
}

impl Hook for RecordCommitted {
    type Args = ChangedRecord;
}

impl Hook for RecordDeleting {
    type Args = RemovedRecord;
}

impl Hook for RecordDeleted {
    type Args = RemovedRecord;
}

impl Hook for RecordPurged {
    type Args = RemovedRecord;
}

impl Hook for RecordRestored {
    type Args = RemovedRecord;
}

impl Hook for RecordQueryFilter {
    type Args = QueryConstraints;
}

impl Hook for RecordSerializing {
    type Args = SerializedRecords;
}

impl Hook for RecordSchemaFilter {
    type Args = RecordSchema;
}

impl Hook for SearchPlanFilter {
    type Args = QueryPlan;
}
