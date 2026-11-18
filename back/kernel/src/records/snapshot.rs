use serde_json::{Map, Value, json};
use sqlx::{MySql, MySqlConnection, QueryBuilder, Row};

use crate::application::fields::{
    FieldDefinitionRepository, FieldSet, RecordTypeRegistry, value_to_json,
};
use crate::application::shared::RecordRef;
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::fields::FieldDefinition;
use crate::errors::AppError;
use crate::infrastructure::fields::mysql_definitions::MySqlFieldDefinitionRepository;
use crate::infrastructure::fields::mysql_values::MySqlCustomFieldValueRepository;
use crate::infrastructure::query::{compiler::quoted, computed::system_expression};

pub async fn capture(
    tx: &mut Tx,
    pool: &Db,
    registry: &RecordTypeRegistry,
    record: &RecordRef,
) -> Result<Option<Value>, AppError> {
    let definitions = MySqlFieldDefinitionRepository::new(pool.clone())
        .list(&record.record_type)
        .await?;
    capture_on(tx, definitions, registry, record, true).await
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
    capture_on(&mut connection, definitions, registry, record, false).await
}

async fn capture_on(
    connection: &mut MySqlConnection,
    definitions: Vec<FieldDefinition>,
    registry: &RecordTypeRegistry,
    record: &RecordRef,
    lock: bool,
) -> Result<Option<Value>, AppError> {
    let definition = registry.require(record.record_type.as_str())?;
    let fields: Vec<_> = definition
        .system_fields
        .iter()
        .filter(|field| field.is_available())
        .collect();
    let mut query: QueryBuilder<MySql> = QueryBuilder::new("SELECT r.id");
    for (index, field) in fields.iter().enumerate() {
        query.push(format!(
            ", {} AS snapshot_{index}",
            system_expression(field, "r")
        ));
    }
    query
        .push(format!(
            " FROM {} r WHERE r.id = ",
            quoted(&definition.table)
        ))
        .push_bind(record.record_id.to_string());
    if let Some(partition) = &definition.partition {
        query
            .push(format!(" AND r.{} = ", quoted(&partition.column)))
            .push_bind(partition.value.to_string());
    }
    if lock {
        query.push(" FOR UPDATE");
    }
    let Some(row) = query.build().fetch_optional(&mut *connection).await? else {
        return Ok(None);
    };
    let mut values = Map::new();
    for (index, field) in fields.iter().enumerate() {
        values.insert(
            field.key.as_str().to_owned(),
            if field.key.as_str() == "updated_at" {
                json!(
                    row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(&*format!(
                        "snapshot_{index}"
                    ))?
                    .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
                )
            } else {
                super::contributed::decode_record_field(
                    &row,
                    &format!("snapshot_{index}"),
                    field.field_type,
                )?
            },
        );
    }
    let field_set = FieldSet::new(definition, definitions);
    let custom: Vec<_> = field_set
        .active_custom()
        .map(|field| field.definition.clone())
        .collect();
    let mut custom_fields: Map<String, Value> = custom
        .iter()
        .map(|field| (field.key.to_string(), Value::Null))
        .collect();
    for value in MySqlCustomFieldValueRepository::load_on(
        connection,
        &record.record_type,
        &custom,
        &[record.record_id],
    )
    .await?
    {
        if let Some(field) = custom.iter().find(|field| field.id == value.field_id) {
            custom_fields.insert(field.key.to_string(), value_to_json(field, &value.value));
        }
    }
    Ok(Some(
        json!({"values": values, "custom_fields": custom_fields}),
    ))
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
