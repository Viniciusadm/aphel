use std::collections::{BTreeMap, BTreeSet};

use chrono::SecondsFormat;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::permissions::{FieldAccess, FieldAccessPolicy, check_access};
use crate::application::fields::{
    ConditionalRules, CustomFieldPorts, CustomFieldValueRepository, EntityDefinition,
    FieldDefinitionRepository, FieldSet, PurgedRecord, RecordPipeline, ReferenceChecker,
    load_values_in, raw_from_json, value_to_json,
};
use crate::application::shared::{FieldChange, RecordRef};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::domain::fields::{
    FieldDefinition, FieldType, FieldValue, FieldValueError, validate_value,
};
use crate::errors::{AppError, FieldViolation};

pub const CUSTOM_FIELDS_PREFIX: &str = "custom_fields";
const CREATED_AT_FIELD: &str = "created_at";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Create,
    Update,
}

#[derive(Debug, Clone, PartialEq)]
enum PendingWrite {
    Set(FieldDefinition, FieldValue),
    Clear(FieldDefinition),
}

impl PendingWrite {
    fn definition(&self) -> &FieldDefinition {
        match self {
            Self::Set(definition, _) | Self::Clear(definition) => definition,
        }
    }
}

pub fn custom_field_path(key: &str) -> String {
    format!("{CUSTOM_FIELDS_PREFIX}.{key}")
}

pub struct CustomFieldWriter<'a, P> {
    ports: &'a P,
    record_values: Map<String, Value>,
}

impl<'a, P> CustomFieldWriter<'a, P>
where
    P: CustomFieldPorts,
{
    pub fn new(ports: &'a P) -> Self {
        Self {
            ports,
            record_values: Map::new(),
        }
    }

    pub fn with_record_values(mut self, system_values: &Value) -> Self {
        if let Value::Object(values) = system_values {
            self.record_values = values.clone();
        }
        self
    }

    pub async fn apply(
        &self,
        tx: &mut P::Tx,
        scope: &AccessScope,
        record: &RecordRef,
        values: &Map<String, Value>,
        mode: WriteMode,
    ) -> Result<Vec<FieldChange>, AppError> {
        let writes = self.validate(tx, scope, record, values, mode).await?;
        let record_type = self.ports.registry().require(record.record_type.as_str())?;
        let mut changes = Vec::with_capacity(writes.len());
        for (write, old) in writes {
            let definition = write.definition();
            let new = match &write {
                PendingWrite::Set(definition, value) => {
                    self.ports
                        .values()
                        .upsert(tx, record, definition, value)
                        .await?;
                    if definition.is_unique {
                        self.ports
                            .values()
                            .reserve_unique(tx, record_type, record, definition)
                            .await?;
                    }
                    value_to_json(definition, value)
                }
                PendingWrite::Clear(definition) => {
                    self.ports
                        .values()
                        .delete(tx, record, definition.id)
                        .await?;
                    if definition.is_unique {
                        self.ports
                            .values()
                            .release_unique(tx, record, definition.id)
                            .await?;
                    }
                    Value::Null
                }
            };
            changes.push(FieldChange::new(
                custom_field_path(definition.key.as_str()),
                old,
                new,
            ));
        }
        Ok(changes)
    }

    pub async fn clear_record(&self, tx: &mut P::Tx, record: &RecordRef) -> Result<(), AppError> {
        self.ports.values().delete_record(tx, record).await
    }

    pub async fn purge_record(
        &self,
        tx: &mut P::Tx,
        scope: &AccessScope,
        record: &RecordRef,
    ) -> Result<PurgedRecord, AppError> {
        self.ports.pipeline().purged(tx, scope, record).await
    }

    async fn validate(
        &self,
        tx: &mut P::Tx,
        scope: &AccessScope,
        record: &RecordRef,
        values: &Map<String, Value>,
        mode: WriteMode,
    ) -> Result<Vec<(PendingWrite, Value)>, AppError> {
        let definition = self.ports.registry().require(record.record_type.as_str())?;
        let fields = FieldSet::new(
            definition,
            self.ports.definitions().list(&record.record_type).await?,
        );
        let current = match mode {
            WriteMode::Create => Map::new(),
            WriteMode::Update => {
                load_values_in(self.ports.values(), tx, &fields, &[record.record_id])
                    .await?
                    .remove(&record.record_id)
                    .unwrap_or_default()
            }
        };

        let policy =
            FieldAccessPolicy::load(self.ports.definitions(), scope, &record.record_type).await?;
        let mut access_violations = Vec::new();
        let mut violations = Vec::new();
        let mut writes: Vec<(PendingWrite, Value)> = Vec::new();
        for (key, input) in values {
            let path = custom_field_path(key);
            let Some(field) = fields.find(key) else {
                violations.push(FieldViolation::new(path, "is not a known field"));
                continue;
            };
            if field.is_system() {
                violations.push(FieldViolation::new(path, "is a system field"));
                continue;
            }
            if !field.definition.is_active {
                violations.push(FieldViolation::new(path, "is inactive"));
                continue;
            }
            let access = policy.access_for(&record.record_type, field);
            if access == FieldAccess::Hidden {
                access_violations.push((access, path));
                continue;
            }
            let old = current.get(key).cloned().unwrap_or(Value::Null);
            if mode == WriteMode::Update && *input == old {
                continue;
            }

            match validated_write(&field.definition, input) {
                Ok(write) => {
                    let unchanged = match &write {
                        PendingWrite::Set(definition, value) => {
                            value_to_json(definition, value) == old
                        }
                        PendingWrite::Clear(_) => old.is_null(),
                    };
                    if unchanged && mode == WriteMode::Update {
                        continue;
                    }
                    if access == FieldAccess::ReadOnly {
                        let cleared_on_create =
                            mode == WriteMode::Create && matches!(write, PendingWrite::Clear(_));
                        if !cleared_on_create {
                            access_violations.push((access, path));
                        }
                        continue;
                    }
                    writes.push((write, old));
                }
                Err(_) if access == FieldAccess::ReadOnly => access_violations.push((access, path)),
                Err(error) => violations.push(FieldViolation::new(path, error.message())),
            }
        }

        check_access(access_violations)?;

        if mode == WriteMode::Create {
            for field in fields.active_custom() {
                if values.contains_key(field.key()) || field.definition.is_formula() {
                    continue;
                }
                let default = field
                    .definition
                    .config
                    .default_value
                    .as_ref()
                    .map(crate::application::fields::raw_to_json);
                match default.map(|input| validated_write(&field.definition, &input)) {
                    Some(Ok(write)) => writes.push((write, Value::Null)),
                    Some(Err(error)) => violations.push(FieldViolation::new(
                        custom_field_path(field.key()),
                        error.message(),
                    )),
                    None if field.definition.is_required => violations.push(FieldViolation::new(
                        custom_field_path(field.key()),
                        FieldValueError::Required.message(),
                    )),
                    None => {}
                }
            }
        }

        writes.retain(|(write, _)| match write {
            PendingWrite::Clear(definition) if definition.is_required => {
                violations.push(FieldViolation::new(
                    custom_field_path(definition.key.as_str()),
                    FieldValueError::Required.message(),
                ));
                false
            }
            PendingWrite::Clear(_) if mode == WriteMode::Create => false,
            _ => true,
        });

        violations.extend(self.check_references(tx, scope, &writes).await?);
        violations.extend(
            self.check_uniqueness(tx, definition, record, &writes)
                .await?,
        );
        let mut merged = self.merged_values(&fields, &current, &writes);
        if mode == WriteMode::Update {
            self.complete_created_at(tx, definition, &fields, record, &mut merged)
                .await?;
        }
        violations.extend(
            self.ports
                .rules()
                .violations(scope, &fields, &merged, mode)
                .await?,
        );
        if !violations.is_empty() {
            violations.sort_by(|left, right| left.field.cmp(&right.field));
            return Err(AppError::invalid_fields(
                "invalid custom field values",
                violations,
            ));
        }
        Ok(writes)
    }

    async fn complete_created_at(
        &self,
        tx: &mut P::Tx,
        definition: &EntityDefinition,
        fields: &FieldSet,
        record: &RecordRef,
        merged: &mut Map<String, Value>,
    ) -> Result<(), AppError> {
        if merged
            .get(CREATED_AT_FIELD)
            .is_some_and(|value| !value.is_null())
        {
            return Ok(());
        }
        let Some(column) = fields
            .find_active(CREATED_AT_FIELD)
            .and_then(|field| field.system.as_ref())
            .and_then(|system| system.stored_column())
        else {
            return Ok(());
        };
        if let Some(created_at) = self
            .ports
            .values()
            .record_timestamp_in(tx, definition, column, record.record_id)
            .await?
        {
            merged.insert(
                CREATED_AT_FIELD.to_owned(),
                Value::String(created_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
            );
        }
        Ok(())
    }

    fn merged_values(
        &self,
        fields: &FieldSet,
        current: &Map<String, Value>,
        writes: &[(PendingWrite, Value)],
    ) -> Map<String, Value> {
        let mut merged = self.record_values.clone();
        for field in fields.active_custom() {
            let value = current.get(field.key()).cloned().unwrap_or(Value::Null);
            merged.insert(field.key().to_owned(), value);
        }
        for (write, _) in writes {
            let (key, value) = match write {
                PendingWrite::Set(definition, value) => {
                    (definition.key.as_str(), value_to_json(definition, value))
                }
                PendingWrite::Clear(definition) => (definition.key.as_str(), Value::Null),
            };
            merged.insert(key.to_owned(), value);
        }
        merged
    }

    async fn check_references(
        &self,
        tx: &mut P::Tx,
        scope: &AccessScope,
        writes: &[(PendingWrite, Value)],
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        let users: BTreeSet<UserId> = writes
            .iter()
            .filter_map(|(write, _)| match write {
                PendingWrite::Set(_, FieldValue::User(id)) => Some(id.clone()),
                _ => None,
            })
            .collect();
        if !users.is_empty() {
            let users: Vec<UserId> = users.into_iter().collect();
            let existing: BTreeSet<UserId> = self
                .ports
                .references()
                .existing_users(&users)
                .await?
                .into_iter()
                .collect();
            for (write, _) in writes {
                if let PendingWrite::Set(definition, FieldValue::User(id)) = write
                    && !existing.contains(id)
                {
                    violations.push(FieldViolation::new(
                        custom_field_path(definition.key.as_str()),
                        FieldValueError::ReferenceNotFound.message(),
                    ));
                }
            }
        }

        let mut records: BTreeMap<String, Vec<(&FieldDefinition, Uuid)>> = BTreeMap::new();
        for (write, _) in writes {
            if let PendingWrite::Set(definition, FieldValue::Record(id)) = write {
                let target = definition
                    .config
                    .target_record_type
                    .clone()
                    .unwrap_or_default();
                records.entry(target).or_default().push((definition, *id));
            }
        }
        for (target, references) in records {
            let path_of = |definition: &FieldDefinition| custom_field_path(definition.key.as_str());
            let definition = self.ports.registry().require(&target)?;
            let visibility = scope.record_filter(&definition.read_permission)?;
            let ids: Vec<Uuid> = references.iter().map(|(_, id)| *id).collect();
            let existing: BTreeSet<Uuid> = match self
                .ports
                .values()
                .existing_records_in(tx, scope, definition, &visibility, &ids)
                .await?
            {
                Some(found) => found,
                None => {
                    self.ports
                        .references()
                        .existing_records(scope, definition, &visibility, &ids)
                        .await?
                }
            }
            .into_iter()
            .collect();
            for (definition, id) in references {
                if !existing.contains(&id) {
                    violations.push(FieldViolation::new(
                        path_of(definition),
                        FieldValueError::ReferenceNotFound.message(),
                    ));
                }
            }
        }
        Ok(violations)
    }

    async fn check_uniqueness(
        &self,
        tx: &mut P::Tx,
        record_type: &EntityDefinition,
        record: &RecordRef,
        writes: &[(PendingWrite, Value)],
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        for (write, _) in writes {
            if let PendingWrite::Set(definition, value) = write
                && definition.is_unique
                && self
                    .ports
                    .values()
                    .is_taken(tx, record_type, record, definition, value)
                    .await?
            {
                violations.push(FieldViolation::new(
                    custom_field_path(definition.key.as_str()),
                    FieldValueError::NotUnique.message(),
                ));
            }
        }
        Ok(violations)
    }
}

fn validated_write(
    definition: &FieldDefinition,
    input: &Value,
) -> Result<PendingWrite, FieldValueError> {
    let raw = raw_from_json(input).ok_or(match definition.field_type.base_type() {
        FieldType::Number | FieldType::Currency => FieldValueError::ExpectedNumber,
        FieldType::Checkbox => FieldValueError::ExpectedBool,
        FieldType::MultiSelect => FieldValueError::ExpectedList,
        _ => FieldValueError::ExpectedText,
    })?;
    Ok(match validate_value(definition, &raw)? {
        Some(value) => PendingWrite::Set(definition.clone(), value),
        None => PendingWrite::Clear(definition.clone()),
    })
}
