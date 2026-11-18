pub use aphel_contract_timeline::*;
use aphel_kernel::database::Db;
use aphel_kernel::database::SqlBuilder;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::{
    Ctx,
    application::{files::UserReference, shared::RecordRef},
    errors::AppError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRecord {
    pub record: RecordRef,
    pub title: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCursor {
    pub occurred_at: DateTime<Utc>,
    pub item_id: String,
}

impl TimelineCursor {
    pub fn new(occurred_at: DateTime<Utc>, item_id: impl Into<String>) -> Self {
        Self {
            occurred_at,
            item_id: item_id.into(),
        }
    }

    pub fn encode(&self) -> String {
        format!("{}|{}", self.occurred_at.timestamp_micros(), self.item_id)
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn decode(value: &str) -> Option<Self> {
        if value.is_empty() || !value.len().is_multiple_of(2) || value.len() > 512 {
            return None;
        }
        let bytes = (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
            .collect::<Option<Vec<u8>>>()?;
        let text = String::from_utf8(bytes).ok()?;
        let (micros, item_id) = text.split_once('|')?;
        let valid_id = !item_id.is_empty()
            && item_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.')
            });
        if !valid_id {
            return None;
        }
        let occurred_at = DateTime::<Utc>::from_timestamp_micros(micros.parse().ok()?)?;
        Some(Self::new(occurred_at, item_id))
    }
}

#[derive(Debug, Clone)]
pub struct TimelineRequest {
    pub subject: RecordRef,
    pub kinds: BTreeSet<String>,
    pub cursor: Option<TimelineCursor>,
    pub limit: u32,
}
impl TimelineRequest {
    pub fn wants(&self, kind: &str) -> bool {
        self.kinds.contains(kind)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEntry {
    pub id: String,
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
    pub actor: Option<UserReference>,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<LinkedRecord>,
    pub metadata: Value,
}

#[async_trait]
pub trait TimelineSource: Send + Sync {
    fn key(&self) -> &'static str;
    fn kinds(&self) -> Vec<KindDefinition>;
    async fn entries(
        &self,
        ctx: &Ctx<'_>,
        request: &TimelineRequest,
    ) -> Result<Vec<TimelineEntry>, AppError>;
}
#[async_trait]
pub trait TimelineEventExtension: Send + Sync {
    fn restrict(
        &self,
        builder: &mut SqlBuilder,
        scope: &AccessScope,
        subject: &RecordRef,
        request: &TimelineRequest,
    );
    async fn enrich(
        &self,
        pool: &Db,
        scope: &AccessScope,
        entries: &mut [TimelineEntry],
    ) -> Result<(), AppError>;
}
pub fn push_cursor(
    builder: &mut SqlBuilder,
    instant_sql: &str,
    id_sql: &str,
    prefix: &str,
    cursor: Option<&TimelineCursor>,
) {
    let Some(cursor) = cursor else {
        return;
    };
    let at = aphel_kernel::infrastructure::repositories::mapper::naive(cursor.occurred_at);
    builder
        .push(format!(" AND ({instant_sql} < "))
        .push_bind(at)
        .push(format!(" OR ({instant_sql} = "))
        .push_bind(at)
        .push(format!(" AND CONCAT('{prefix}', {id_sql}) < "))
        .push_bind(cursor.item_id.clone())
        .push("))");
}

#[async_trait]
pub trait TimelineEventScope: Send + Sync {
    async fn extend(
        &self,
        ctx: &Ctx<'_>,
        builder: &mut SqlBuilder,
        subject: &RecordRef,
    ) -> Result<(), AppError>;
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LinkedRecordResponse {
    pub record_type: String,
    pub record_id: String,
    pub title: Option<String>,
}

impl From<LinkedRecord> for LinkedRecordResponse {
    fn from(linked: LinkedRecord) -> Self {
        Self {
            record_type: linked.record.record_type.as_str().to_owned(),
            record_id: linked.record.record_id.to_string(),
            title: linked.title,
        }
    }
}
