use serde_json::{Map, Value, json};

use crate::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use crate::application::shared::RecordRef;
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::errors::AppError;
use crate::infrastructure::fields::mysql_definitions::MySqlFieldDefinitionRepository;

pub async fn capture(
    tx: &mut Tx,
    pool: &Db,
    registry: &RecordTypeRegistry,
    record: &RecordRef,
) -> Result<Option<Value>, AppError> {
    let definitions = MySqlFieldDefinitionRepository::new(pool.clone())
        .list(&record.record_type)
        .await?;
    super::store::snapshot::capture_on(tx, definitions, registry, record, true).await
}

pub async fn read(
    pool: &Db,
    registry: &RecordTypeRegistry,
    record: &RecordRef,
) -> Result<Option<Value>, AppError> {
    let definitions = MySqlFieldDefinitionRepository::new(pool.clone())
        .list(&record.record_type)
        .await?;
    let mut connection = pool.acquire().await?;
    super::store::snapshot::capture_on(&mut connection, definitions, registry, record, false).await
}

pub(crate) fn remember(tx: &mut Tx, record: &RecordRef, snapshot: Option<Value>) {
    tx.fresh_snapshot = Some((
        (record.record_type.as_str().to_owned(), record.record_id),
        snapshot,
    ));
}

pub(crate) fn take_fresh(tx: &mut Tx, record: &RecordRef) -> Option<Option<Value>> {
    let ((record_type, record_id), snapshot) = tx.fresh_snapshot.take()?;
    (record_type == record.record_type.as_str() && record_id == record.record_id)
        .then_some(snapshot)
}

pub fn proposed(
    before: Option<&Value>,
    values: &Map<String, Value>,
    custom: &Map<String, Value>,
) -> Value {
    let mut proposed = before
        .cloned()
        .unwrap_or_else(|| json!({"values": {}, "custom_fields": {}}));
    if let Some(target) = proposed.get_mut("values").and_then(Value::as_object_mut) {
        target.extend(values.clone());
    }
    if let Some(target) = proposed
        .get_mut("custom_fields")
        .and_then(Value::as_object_mut)
    {
        target.extend(custom.clone());
    }
    proposed
}
