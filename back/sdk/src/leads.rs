use crate::{AppError, Ctx};
pub use aphel_contract_leads::*;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::modules::Hook;
use aphel_kernel::records::PendingCommit;
use async_trait::async_trait;
use serde_json::{Map, Value};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq)]
pub struct Lead {
    pub values: Map<String, Value>,
    id: Uuid,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
impl Lead {
    pub fn from_record(value: Value) -> Result<Self, AppError> {
        let values = value
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::Internal("the lead record is not an object".into()))?;
        let id = values
            .get("id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| AppError::Internal("the lead has no identity".into()))?;
        let updated_at = values
            .get("updated_at")
            .and_then(Value::as_str)
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&chrono::Utc))
            .ok_or_else(|| AppError::Forbidden("the lead record version is not readable".into()))?;
        Ok(Self {
            values,
            id,
            updated_at,
        })
    }
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn company_id(&self) -> Option<Uuid> {
        self.values
            .get("company_id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LeadQualifying;

impl Hook for LeadQualifying {
    type Args = Lead;
    fn key() -> &'static str {
        QUALIFYING
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeadConversion {
    pub lead: Lead,
    pub deal: Map<String, Value>,
    pub company: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Copy)]
pub struct LeadConverting;

impl Hook for LeadConverting {
    type Args = LeadConversion;
    fn key() -> &'static str {
        CONVERTING
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertLeadCommand {
    pub pipeline_id: String,
    pub stage_id: Option<String>,
    pub title: Option<String>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub deal_id: Uuid,
    pub company_id: Option<Uuid>,
    pub pending: Vec<PendingCommit>,
}

#[async_trait]
pub trait LeadConverter: Send + Sync {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError>;
}
