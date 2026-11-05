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

#[derive(Debug, Clone)]
pub struct DatasetRandom(u64);

impl DatasetRandom {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn for_owner(seed: u64, owner: &str) -> Self {
        let mixed = owner.bytes().fold(seed, |state, byte| {
            (state ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
        Self::new(mixed)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound.max(1) as u64) as usize
    }

    pub fn between(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next_u64() % (high - low + 1).max(1) as u64) as i64
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[derive(Debug, Clone)]
pub struct DatasetEntry {
    pub record: RecordRef,
    pub created_at: DateTime<Utc>,
    pub changed_at: DateTime<Utc>,
    pub attributes: BTreeMap<String, String>,
}

pub struct DatasetSession {
    now: DateTime<Utc>,
    seed: u64,
    catalog: MaintenanceCatalog,
    entries: BTreeMap<String, Vec<DatasetEntry>>,
    pending: Vec<DatasetEntry>,
    values: BTreeMap<String, String>,
    counts: BTreeMap<String, u64>,
}

impl DatasetSession {
    pub fn new(now: DateTime<Utc>, seed: u64, catalog: MaintenanceCatalog) -> Self {
        Self {
            now,
            seed,
            catalog,
            entries: BTreeMap::new(),
            pending: Vec::new(),
            values: BTreeMap::new(),
            counts: BTreeMap::new(),
        }
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.now
    }

    pub fn random(&self, owner: &str) -> DatasetRandom {
        DatasetRandom::for_owner(self.seed, owner)
    }

    pub fn catalog(&self) -> &MaintenanceCatalog {
        &self.catalog
    }

    pub fn entries(&self, entity: &str) -> &[DatasetEntry] {
        self.entries.get(entity).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn ids(&self, entity: &str) -> Vec<uuid::Uuid> {
        self.entries(entity)
            .iter()
            .map(|entry| entry.record.record_id)
            .collect()
    }

    pub fn push(&mut self, entry: DatasetEntry) {
        self.entries
            .entry(entry.record.record_type.as_str().to_owned())
            .or_default()
            .push(entry.clone());
        self.pending.push(entry);
    }

    pub fn record(
        &mut self,
        record: RecordRef,
        created_at: DateTime<Utc>,
        changed_at: DateTime<Utc>,
        attributes: BTreeMap<String, String>,
    ) {
        self.push(DatasetEntry {
            record,
            created_at,
            changed_at,
            attributes,
        });
    }

    pub fn set_value(&mut self, key: &str, value: String) {
        self.values.insert(key.to_owned(), value);
    }

    pub fn value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn add_count(&mut self, key: &str, amount: u64) {
        if amount > 0 {
            *self.counts.entry(key.to_owned()).or_default() += amount;
        }
    }

    pub fn take_counts(&mut self) -> BTreeMap<String, u64> {
        std::mem::take(&mut self.counts)
    }

    pub fn take_pending(&mut self) -> Vec<DatasetEntry> {
        std::mem::take(&mut self.pending)
    }
}

#[async_trait]
pub trait DatasetSeed: Send + Sync {
    fn order(&self) -> i32;
    fn entities(&self) -> Vec<crate::shared::RecordType>;
    async fn catalog(
        &self,
        _ctx: &Ctx<'_>,
        _catalog: &mut MaintenanceCatalog,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError>;
    async fn backdate(&self, _ctx: &Ctx<'_>, _entry: &DatasetEntry) -> Result<bool, AppError> {
        Ok(false)
    }
    async fn cleanup(
        &self,
        _tx: &mut MySqlTx,
        _records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        Ok(false)
    }
    async fn counts(&self, _tx: &mut MySqlTx) -> Result<Option<BTreeMap<String, u64>>, AppError> {
        Ok(None)
    }
}

pub fn push_record_tuples(query: &mut sqlx::QueryBuilder<sqlx::MySql>, records: &[RecordRef]) {
    if records.is_empty() {
        query.push("FALSE");
        return;
    }
    query.push("(record_type, record_id) IN (");
    let mut tuples = query.separated(", ");
    for record in records {
        tuples
            .push("(")
            .push_bind_unseparated(record.record_type.as_str().to_owned())
            .push_unseparated(", ")
            .push_bind_unseparated(record.record_id.to_string())
            .push_unseparated(")");
    }
    query.push(")");
}

pub fn ids_of(records: &[RecordRef], record_type: &crate::shared::RecordType) -> Vec<String> {
    records
        .iter()
        .filter(|record| &record.record_type == record_type)
        .map(|record| record.record_id.to_string())
        .collect()
}

#[async_trait]
pub trait TemplateOnboarding: Send + Sync {
    async fn complete_with_template(
        &self,
        ctx: &Ctx<'_>,
        template_key: &str,
        install_id: &str,
    ) -> Result<(), AppError>;
}
