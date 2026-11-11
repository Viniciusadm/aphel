use std::collections::HashMap;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application::fields::StoredFieldValue;
use crate::application::fields::{
    CustomFieldPorts, CustomFieldValueRepository, FieldDefinitionRepository, FieldSet,
    value_to_json,
};
use crate::application::shared::RecordType;
use crate::domain::access::AccessScope;
use crate::domain::fields::FieldDefinition;
use crate::errors::AppError;

pub type CustomFieldMap = Map<String, Value>;

pub struct CustomFieldReader<'a, P> {
    ports: &'a P,
}

impl<'a, P> CustomFieldReader<'a, P>
where
    P: CustomFieldPorts,
{
    pub fn new(ports: &'a P) -> Self {
        Self { ports }
    }

    pub async fn load(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, CustomFieldMap>, AppError> {
        let definition = self.ports.registry().require(record_type.as_str())?;
        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        load_values(self.ports.values(), &fields, record_ids).await
    }

    pub async fn load_unrestricted(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, CustomFieldMap>, AppError> {
        let definition = self.ports.registry().require(record_type.as_str())?;
        let fields = FieldSet::new(
            definition,
            self.ports.definitions().list(record_type).await?,
        );
        load_values(self.ports.values(), &fields, record_ids).await
    }
}

pub async fn load_values<V>(
    values: &V,
    fields: &FieldSet,
    record_ids: &[Uuid],
) -> Result<HashMap<Uuid, CustomFieldMap>, AppError>
where
    V: CustomFieldValueRepository,
{
    let (active, result) = empty_values(fields, record_ids);
    if record_ids.is_empty() || active.is_empty() {
        return Ok(result);
    }
    let stored = values
        .load(&fields.record_type.record_type, &active, record_ids)
        .await?;
    Ok(merge_stored(&active, result, stored))
}

pub async fn load_values_in<V>(
    values: &V,
    tx: &mut V::Tx,
    fields: &FieldSet,
    record_ids: &[Uuid],
) -> Result<HashMap<Uuid, CustomFieldMap>, AppError>
where
    V: CustomFieldValueRepository,
{
    let (active, result) = empty_values(fields, record_ids);
    if record_ids.is_empty() || active.is_empty() {
        return Ok(result);
    }
    let stored = values
        .load_in(tx, &fields.record_type.record_type, &active, record_ids)
        .await?;
    Ok(merge_stored(&active, result, stored))
}

fn empty_values(
    fields: &FieldSet,
    record_ids: &[Uuid],
) -> (Vec<FieldDefinition>, HashMap<Uuid, CustomFieldMap>) {
    let active: Vec<_> = fields
        .active_custom()
        .map(|field| field.definition.clone())
        .collect();
    let empty: CustomFieldMap = active
        .iter()
        .map(|definition| (definition.key.as_str().to_owned(), Value::Null))
        .collect();
    let result = record_ids.iter().map(|id| (*id, empty.clone())).collect();
    (active, result)
}

fn merge_stored(
    active: &[FieldDefinition],
    mut result: HashMap<Uuid, CustomFieldMap>,
    stored: Vec<StoredFieldValue>,
) -> HashMap<Uuid, CustomFieldMap> {
    for value in stored {
        let Some(definition) = active.iter().find(|field| field.id == value.field_id) else {
            continue;
        };
        if let Some(map) = result.get_mut(&value.record_id) {
            map.insert(
                definition.key.as_str().to_owned(),
                value_to_json(definition, &value.value),
            );
        }
    }
    result
}
