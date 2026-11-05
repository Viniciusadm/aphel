use crate::shared::RecordRef;
use crate::{AppError, Ctx};
use aphel_kernel::database::transaction::MySqlTx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct MaintenanceCatalog {
    pub stages: Vec<(String, String)>,
    pub lost_reason_ids: Vec<String>,
    pub task_types: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupPhase {
    References,
    Records,
    AfterRecords,
}
#[async_trait]
pub trait DatasetMaintenance: Send + Sync {
    fn order(&self) -> i32 {
        0
    }
    async fn counts(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
    ) -> Result<BTreeMap<String, u64>, AppError>;
    async fn catalog(
        &self,
        _ctx: &Ctx<'_>,
        _catalog: &mut MaintenanceCatalog,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn key_exists(
        &self,
        _ctx: &Ctx<'_>,
        _kind: &str,
        _key: &str,
    ) -> Result<Option<bool>, AppError> {
        Ok(None)
    }
    async fn backdate(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record: &RecordRef,
        created_at: DateTime<Utc>,
        changed_at: DateTime<Utc>,
    ) -> Result<bool, AppError>;
    async fn cleanup(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
        phase: CleanupPhase,
    ) -> Result<(), AppError>;
}
