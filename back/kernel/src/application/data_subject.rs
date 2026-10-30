use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::application::shared::RecordRef;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSubjectAction {
    Export,
    Anonymize,
    Delete,
}

impl DataSubjectAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Anonymize => "anonymize",
            Self::Delete => "delete",
        }
    }
}

pub struct DataSubjectRequest<'a> {
    pub id: Uuid,
    pub action: DataSubjectAction,
    pub records: &'a [RecordRef],
}

pub struct DataSubjectOutcome {
    pub source: String,
    pub result: Result<Value, AppError>,
}

#[async_trait]
pub trait DataSubjectSource: Send + Sync {
    async fn run(&self, request: &DataSubjectRequest<'_>) -> Vec<DataSubjectOutcome>;
}

pub fn affected_items(data: &Value) -> u64 {
    data.get("affected").and_then(Value::as_u64).unwrap_or(0)
}
