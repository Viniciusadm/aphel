use crate::{AppError, Ctx};
use aphel_kernel::database::transaction::MySqlTx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[async_trait]
pub trait MergeOwner: Send + Sync {
    fn record_type(&self) -> &'static str;
    fn not_found(&self) -> AppError;
    fn fields(&self) -> &'static [&'static str];
    async fn prepare(
        &self,
        ctx: &Ctx<'_>,
        primary: Uuid,
        duplicate: Uuid,
        choices: &BTreeMap<String, bool>,
    ) -> Result<Box<dyn PreparedMerge>, AppError>;
    async fn retire(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        duplicate: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait PreparedMerge: Send + Sync {
    fn before(&self) -> Value;
    fn duplicate_name(&self) -> String;
    async fn apply(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
        ip: Option<String>,
    ) -> Result<Value, AppError>;
    async fn persist(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait MergeReferences: Send + Sync {
    async fn move_references(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record_type: &str,
        from: Uuid,
        to: Uuid,
    ) -> Result<BTreeMap<String, u64>, AppError>;
}

pub fn owner(ctx: &Ctx<'_>, record_type: &str) -> Option<std::sync::Arc<dyn MergeOwner>> {
    ctx.app()
        .contributions::<dyn MergeOwner>()
        .iter()
        .find(|owner| owner.record_type() == record_type)
        .cloned()
}

pub fn use_duplicate(
    choices: &BTreeMap<String, bool>,
    key: &str,
    primary_empty: bool,
    duplicate_empty: bool,
) -> bool {
    choices
        .get(key)
        .copied()
        .unwrap_or(primary_empty && !duplicate_empty)
}
