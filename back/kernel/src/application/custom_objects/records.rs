use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_values};
use crate::application::custom_objects::{
    ChangeRecord, CustomRecordPorts, CustomRecordRepository, CustomRecordsTx, RecordLink,
    RecordLinkRepository, custom_object_not_found, custom_record_definition, publish_change,
    relation_sides,
};
use crate::application::fields::permissions::check_record_writes;
use crate::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, EntityDefinition, FieldSet,
    RecordDraft, RecordPipeline, RecordWrite, WriteMode, custom_field_path, saving_unsupported,
};
use crate::application::files::RecordAccessChecker;
use crate::application::organization_admin::Listing;
use crate::application::query::{RecordQuery, RecordQueryEngine, RecordQueryPorts};
use crate::application::security::Clock;
use crate::application::shared::{
    FieldChange, RecordRef, RecordType, TransactionManager, ensure_version,
};
use crate::domain::access::AccessScope;
use crate::domain::custom_objects::{
    CustomObject, CustomRecord, RelationSide, RelationshipDefinition, record_title,
};
use crate::errors::{AppError, FieldViolation};
use crate::records::{InputValidator, Patch, PendingCommit, check_owner, permission_denied};

pub const CUSTOM_RECORD_NOT_FOUND: &str = "custom_record_not_found";
pub const MAX_RELATIONS_ON_CREATE: usize = 100;

const INVALID_RECORD: &str = "invalid record";

pub fn custom_record_not_found() -> AppError {
    AppError::not_found_code(CUSTOM_RECORD_NOT_FOUND, "record was not found")
}

pub fn record_event(record_type: &str, action: &str) -> String {
    format!("{record_type}.{action}")
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SaveCustomRecordCommand {
    pub owner_id: Patch<String>,
    pub custom_fields: Map<String, Value>,
    pub relations: Map<String, Value>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomRecordDetails {
    pub object: CustomObject,
    pub record: CustomRecord,
    pub custom_fields: Map<String, Value>,
}

pub fn custom_record_snapshot(record: &CustomRecord) -> Value {
    json!({
        "title": record.title,
        "owner_id": record.owner_id.as_ref().map(|owner| owner.as_str().to_owned()),
    })
}

fn title_from(value: &Value) -> String {
    match value {
        Value::String(text) => record_title(text),
        Value::Number(number) => record_title(&number.to_string()),
        Value::Bool(flag) => record_title(&flag.to_string()),
        _ => String::new(),
    }
}

fn relation_ids(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Null => Some(Vec::new()),
        Value::String(text) => Some(vec![text.trim().to_owned()]),
        Value::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(|text| text.trim().to_owned()))
            .collect(),
        _ => None,
    }
}

struct PendingLink {
    relationship: RelationshipDefinition,
    side: RelationSide,
    key: String,
    target: RecordRef,
}

pub struct CustomRecordUseCases<P> {
    ports: P,
}

impl<P> CustomRecordUseCases<P>
where
    P: CustomRecordPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub fn ports(&self) -> &P {
        &self.ports
    }

    pub fn object(&self, key: &str) -> Result<&CustomObject, AppError> {
        self.ports
            .objects()
            .iter()
            .find(|object| object.key == key.trim() && object.data.is_active)
            .ok_or_else(custom_object_not_found)
    }

    pub fn object_by_type(&self, record_type: &str) -> Result<&CustomObject, AppError> {
        self.ports
            .objects()
            .iter()
            .find(|object| object.record_type() == record_type && object.data.is_active)
            .ok_or_else(custom_object_not_found)
    }

    fn definition(&self, object: &CustomObject) -> Result<EntityDefinition, AppError> {
        match self.ports.query().registry().require(&object.record_type()) {
            Ok(definition) => Ok(definition.clone()),
            Err(_) => custom_record_definition(object),
        }
    }

    pub async fn fields(&self, scope: &AccessScope, key: &str) -> Result<FieldSet, AppError> {
        let object = self.object(key)?;
        if self.definition(object)?.storage != crate::application::fields::EntityStorage::Dynamic {
            return Err(crate::records::operation_not_supported(
                &object.record_type(),
                "use the generic entity record routes for this storage backend",
            ));
        }
        let definition = self.definition(object)?;
        scope.record_filter(&definition.read_permission)?;
        FieldSet::for_scope(self.ports.query().definitions(), &definition, scope).await
    }

    pub async fn list(
        &self,
        scope: &AccessScope,
        key: &str,
        query: RecordQuery,
    ) -> Result<Listing<CustomRecordDetails>, AppError> {
        let object = self.object(key)?.clone();
        let page = RecordQueryEngine::new(self.ports.query())
            .query(scope, &object.record_type(), query)
            .await?;
        let records = self
            .ports
            .records()
            .find_many(object.id, &page.items)
            .await?;
        let ordered = page
            .items
            .iter()
            .filter_map(|id| records.iter().find(|record| record.id == *id).cloned())
            .collect();
        Ok(Listing {
            items: self.with_custom_fields(scope, &object, ordered).await?,
            total: page.total,
        })
    }

    pub async fn listed(
        &self,
        scope: &AccessScope,
        key: &str,
        ids: &[Uuid],
    ) -> Result<Vec<CustomRecordDetails>, AppError> {
        let object = self.object(key)?.clone();
        let records = self.ports.records().find_many(object.id, ids).await?;
        let ordered = ids
            .iter()
            .filter_map(|id| records.iter().find(|record| record.id == *id).cloned())
            .collect();
        self.with_custom_fields(scope, &object, ordered).await
    }

    pub async fn get(
        &self,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
    ) -> Result<CustomRecordDetails, AppError> {
        let object = self.object(key)?.clone();
        let record = self.allowed(scope, &object, record_id, "read").await?;
        self.details(scope, &object, record).await
    }

    pub async fn create(
        &self,
        scope: &AccessScope,
        key: &str,
        command: SaveCustomRecordCommand,
    ) -> Result<CustomRecordDetails, AppError> {
        let object = self.object(key)?.clone();
        let mut tx = self.ports.transactions().begin().await?;
        let (id, pending) = self.create_in_tx(&mut tx, scope, key, command).await?;
        self.ports.transactions().commit(tx).await?;
        self.ports
            .fields()
            .pipeline()
            .committed(scope, &pending.write())
            .await;
        self.reload(scope, &object, id).await
    }

    pub async fn create_in_tx(
        &self,
        tx: &mut CustomRecordsTx<P>,
        scope: &AccessScope,
        key: &str,
        command: SaveCustomRecordCommand,
    ) -> Result<(Uuid, PendingCommit), AppError> {
        let object = self.object(key)?.clone();
        let record_type = object.record_type();
        let permission = object
            .permission("create")
            .ok_or_else(custom_object_not_found)?;
        scope.record_filter(&permission)?;
        let mut validator = InputValidator::new();
        let owner_id = if object.data.owner_enabled {
            let value = command
                .owner_id
                .clone()
                .unwrap_or_else(|| Some(scope.user_id().as_str().to_owned()));
            validator.user("owner_id", value)
        } else {
            None
        };
        check_owner(self.ports.fields(), owner_id.as_ref(), &mut validator).await?;
        let mut provided = Vec::new();
        if command.owner_id.is_some() {
            provided.push("owner_id");
        }
        let now = self.ports.clock().now();
        let mut record = CustomRecord {
            id: Uuid::now_v7(),
            object_id: object.id,
            owner_id,
            title: String::new(),
            created_by: Some(scope.user_id().clone()),
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        check_record_writes(
            self.ports.fields().definitions(),
            scope,
            &RecordType::parse(record_type.as_str())?,
            &provided,
            &Value::Null,
            &custom_record_snapshot(&record),
        )
        .await?;
        let links = self
            .pending_links(scope, &object, &command.relations, &mut validator)
            .await?;
        let reference = RecordRef::new(RecordType::parse(record_type.as_str())?, record.id);
        let custom_fields = self
            .saving(
                tx,
                scope,
                &object,
                &reference,
                WriteMode::Create,
                &mut record,
                command.custom_fields,
                &mut validator,
            )
            .await?;
        let custom = CustomFieldWriter::new(self.ports.fields())
            .with_record_values(&custom_record_snapshot(&record))
            .apply(tx, scope, &reference, &custom_fields, WriteMode::Create)
            .await;
        let mut changes = crate::records::merge_violations(validator, custom, INVALID_RECORD)?;
        record.title = self.derived_title(&object, &changes).unwrap_or_default();
        self.ports.records().insert(tx, &record).await?;
        for link in &links {
            let (from_id, to_id) = match link.side {
                RelationSide::From => (record.id, link.target.record_id),
                RelationSide::To => (link.target.record_id, record.id),
            };
            self.check_capacity(tx, link, from_id, to_id).await?;
            self.ports
                .links()
                .insert(
                    tx,
                    &RecordLink {
                        relationship_id: link.relationship.id,
                        from_id,
                        to_id,
                        created_by: Some(scope.user_id().clone()),
                        created_at: now,
                    },
                )
                .await?;
            changes.push(FieldChange::new(
                format!("relations.{}", link.key),
                Value::Null,
                json!({
                    "record_type": link.target.record_type.as_str(),
                    "record_id": link.target.record_id.to_string(),
                }),
            ));
        }
        let mut all_changes = diff_values(
            &Value::Null,
            &custom_record_snapshot(&record),
            &DiffOptions::default(),
        )?;
        all_changes.extend(changes);
        let record_values = custom_record_snapshot(&record);
        let write = RecordWrite {
            record: &reference,
            mode: WriteMode::Create,
            changes: &all_changes,
            values: &record_values,
        };
        let pipeline = self.ports.fields().pipeline();
        pipeline.validate(tx, scope, &write).await?;
        pipeline.saved(tx, scope, &write).await?;
        self.publish(
            tx,
            scope,
            &reference,
            record_event(&record_type, "created"),
            json!({
                "object": object.key,
                "title": record.title,
                "owner_id": record.owner_id.as_ref().map(|owner| owner.as_str().to_owned()),
            }),
            all_changes.clone(),
            command.ip,
        )
        .await?;
        Ok((
            record.id,
            PendingCommit {
                record: reference,
                mode: WriteMode::Create,
                changes: all_changes,
                values: record_values,
            },
        ))
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
        command: SaveCustomRecordCommand,
    ) -> Result<CustomRecordDetails, AppError> {
        let object = self.object(key)?.clone();
        let mut tx = self.ports.transactions().begin().await?;
        let (id, pending) = self
            .update_in_tx(&mut tx, scope, key, record_id, command)
            .await?;
        match pending {
            Some(pending) => {
                self.ports.transactions().commit(tx).await?;
                self.ports
                    .fields()
                    .pipeline()
                    .committed(scope, &pending.write())
                    .await;
            }
            None => drop(tx),
        }
        self.reload(scope, &object, id).await
    }

    pub async fn update_in_tx(
        &self,
        tx: &mut CustomRecordsTx<P>,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
        command: SaveCustomRecordCommand,
    ) -> Result<(Uuid, Option<PendingCommit>), AppError> {
        let object = self.object(key)?.clone();
        let record_type = object.record_type();
        if let Ok(id) = Uuid::parse_str(record_id.trim()) {
            self.ports.records().lock(tx, id).await?;
        }
        let current = self.allowed(scope, &object, record_id, "update").await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        let mut validator = InputValidator::new();
        let mut record = current.clone();
        let mut provided = Vec::new();
        if let Some(value) = command.owner_id.clone()
            && object.data.owner_enabled
        {
            provided.push("owner_id");
            record.owner_id = validator.user("owner_id", value);
            if record.owner_id != current.owner_id {
                check_owner(
                    self.ports.fields(),
                    record.owner_id.as_ref(),
                    &mut validator,
                )
                .await?;
            }
        }
        if !command.relations.is_empty() {
            validator.violation(
                "relations",
                "use the relation endpoints to link or unlink records of an existing record",
            );
        }
        check_record_writes(
            self.ports.fields().definitions(),
            scope,
            &RecordType::parse(record_type.as_str())?,
            &provided,
            &custom_record_snapshot(&current),
            &custom_record_snapshot(&record),
        )
        .await?;
        let reference = RecordRef::new(RecordType::parse(record_type.as_str())?, record.id);
        let custom_fields = self
            .saving(
                tx,
                scope,
                &object,
                &reference,
                WriteMode::Update,
                &mut record,
                command.custom_fields,
                &mut validator,
            )
            .await?;
        let custom = CustomFieldWriter::new(self.ports.fields())
            .with_record_values(&custom_record_snapshot(&record))
            .apply(tx, scope, &reference, &custom_fields, WriteMode::Update)
            .await;
        let custom = crate::records::merge_violations(validator, custom, INVALID_RECORD)?;
        if let Some(title) = self.derived_title(&object, &custom) {
            record.title = title;
        }
        let mut changes = diff_values(
            &custom_record_snapshot(&current),
            &custom_record_snapshot(&record),
            &DiffOptions::default(),
        )?;
        changes.extend(custom);
        if changes.is_empty() {
            return Ok((current.id, None));
        }
        let record_values = custom_record_snapshot(&record);
        let write = RecordWrite {
            record: &reference,
            mode: WriteMode::Update,
            changes: &changes,
            values: &record_values,
        };
        let pipeline = self.ports.fields().pipeline();
        pipeline.validate(tx, scope, &write).await?;
        record.updated_at = self.ports.clock().now();
        self.ports.records().update(tx, &record).await?;
        pipeline.saved(tx, scope, &write).await?;
        self.publish(
            tx,
            scope,
            &reference,
            record_event(&record_type, "updated"),
            crate::records::changes_payload(&changes),
            changes.clone(),
            command.ip,
        )
        .await?;
        Ok((
            record.id,
            Some(PendingCommit {
                record: reference,
                mode: WriteMode::Update,
                changes,
                values: record_values,
            }),
        ))
    }

    pub async fn set_archived(
        &self,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
        archived: bool,
        ip: Option<String>,
    ) -> Result<CustomRecordDetails, AppError> {
        let object = self.object(key)?.clone();
        let record_type = object.record_type();
        let current = self.allowed(scope, &object, record_id, "archive").await?;
        if current.archived_at.is_some() == archived {
            return self.details(scope, &object, current).await;
        }
        let now = self.ports.clock().now();
        let archived_at = archived.then_some(now);
        let reference = RecordRef::new(RecordType::parse(record_type.as_str())?, current.id);
        let mut tx = self.ports.transactions().begin().await?;
        if archived {
            self.ports
                .fields()
                .pipeline()
                .archiving(&mut tx, scope, &reference)
                .await?;
        }
        self.ports
            .records()
            .set_archived(&mut tx, current.id, archived_at, now)
            .await?;
        self.publish(
            &mut tx,
            scope,
            &reference,
            record_event(
                &record_type,
                if archived { "archived" } else { "unarchived" },
            ),
            json!({ "title": current.title }),
            vec![FieldChange::new(
                "archived_at",
                json!(current.archived_at.map(|at| at.to_rfc3339())),
                json!(archived_at.map(|at| at.to_rfc3339())),
            )],
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.reload(scope, &object, current.id).await
    }

    pub async fn delete(
        &self,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let mut tx = self.ports.transactions().begin().await?;
        self.delete_in_tx(&mut tx, scope, key, record_id, ip)
            .await?;
        self.ports.transactions().commit(tx).await
    }

    pub async fn delete_in_tx(
        &self,
        tx: &mut CustomRecordsTx<P>,
        scope: &AccessScope,
        key: &str,
        record_id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let object = self.object(key)?.clone();
        let record_type = object.record_type();
        let current = self.allowed(scope, &object, record_id, "delete").await?;
        let now = self.ports.clock().now();
        let reference = RecordRef::new(RecordType::parse(record_type.as_str())?, current.id);
        let pipeline = self.ports.fields().pipeline();
        pipeline.deleting(tx, scope, &reference).await?;
        self.ports
            .records()
            .soft_delete(tx, current.id, now)
            .await?;
        self.ports
            .links()
            .delete_record_links(tx, current.id)
            .await?;
        pipeline.deleted(tx, scope, &reference).await?;
        self.publish(
            tx,
            scope,
            &reference,
            record_event(&record_type, "deleted"),
            json!({ "title": current.title }),
            diff_values(
                &custom_record_snapshot(&current),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip,
        )
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn saving(
        &self,
        tx: &mut CustomRecordsTx<P>,
        scope: &AccessScope,
        object: &CustomObject,
        reference: &RecordRef,
        mode: WriteMode,
        record: &mut CustomRecord,
        custom_fields: Map<String, Value>,
        validator: &mut InputValidator,
    ) -> Result<Map<String, Value>, AppError> {
        let values = custom_record_snapshot(record);
        let draft = self
            .ports
            .fields()
            .pipeline()
            .saving(
                tx,
                scope,
                RecordDraft::new(reference.clone(), mode, &values, custom_fields),
            )
            .await?;
        for (key, value) in draft.changed_values(&values) {
            match key.as_str() {
                "owner_id" if object.data.owner_enabled => {
                    record.owner_id = validator.user("owner_id", value.as_str().map(str::to_owned));
                    check_owner(self.ports.fields(), record.owner_id.as_ref(), validator).await?;
                }
                _ => return Err(saving_unsupported(reference, &key)),
            }
        }
        Ok(draft.custom_fields)
    }

    async fn allowed(
        &self,
        scope: &AccessScope,
        object: &CustomObject,
        record_id: &str,
        action: &str,
    ) -> Result<CustomRecord, AppError> {
        let id = Uuid::parse_str(record_id.trim()).map_err(|_| custom_record_not_found())?;
        let permission = object
            .permission(action)
            .ok_or_else(custom_object_not_found)?;
        scope.record_filter(&permission)?;
        let reference = RecordRef::new(RecordType::parse(object.record_type())?, id);
        let read = object
            .permission("read")
            .ok_or_else(custom_object_not_found)?;
        if !self
            .ports
            .access()
            .can_with(scope, &reference, &read)
            .await?
        {
            return Err(custom_record_not_found());
        }
        if action != "read"
            && !self
                .ports
                .access()
                .can_with(scope, &reference, &permission)
                .await?
        {
            return Err(permission_denied());
        }
        self.ports
            .records()
            .find(object.id, id)
            .await?
            .ok_or_else(custom_record_not_found)
    }

    async fn pending_links(
        &self,
        scope: &AccessScope,
        object: &CustomObject,
        values: &Map<String, Value>,
        validator: &mut InputValidator,
    ) -> Result<Vec<PendingLink>, AppError> {
        let record_type = object.record_type();
        let sides = relation_sides(self.ports.relationships(), &record_type);
        let mut links = Vec::new();
        for (key, value) in values {
            let Some((relationship, side)) = sides
                .iter()
                .find(|(relationship, side)| relationship.relation_key(*side) == key)
            else {
                validator.violation(
                    &format!("relations.{key}"),
                    "is not a relation of this object",
                );
                continue;
            };
            let Some(ids) = relation_ids(value) else {
                validator.violation(
                    &format!("relations.{key}"),
                    "must be a record id or a list of record ids",
                );
                continue;
            };
            if ids.len() > MAX_RELATIONS_ON_CREATE || (relationship.single(*side) && ids.len() > 1)
            {
                validator.violation(
                    &format!("relations.{key}"),
                    if relationship.single(*side) {
                        "accepts a single record".to_owned()
                    } else {
                        format!("accepts at most {MAX_RELATIONS_ON_CREATE} records")
                    },
                );
                continue;
            }
            let target_type = RecordType::parse(relationship.target_type(*side))?;
            for id in ids {
                let Ok(target_id) = Uuid::parse_str(&id) else {
                    validator.violation(&format!("relations.{key}"), "must contain record ids");
                    continue;
                };
                let target = RecordRef::new(target_type.clone(), target_id);
                if !self.ports.access().can_read(scope, &target).await? {
                    validator.violation(
                        &format!("relations.{key}"),
                        "references a record that does not exist or you cannot access",
                    );
                    continue;
                }
                links.push(PendingLink {
                    relationship: relationship.clone(),
                    side: *side,
                    key: key.clone(),
                    target,
                });
            }
        }
        for (relationship, side) in &sides {
            if *side == RelationSide::To
                && relationship.required
                && !links
                    .iter()
                    .any(|link| link.relationship.id == relationship.id && link.side == *side)
            {
                validator.violation(
                    &format!("relations.{}", relationship.inverse_key),
                    "is required",
                );
            }
        }
        Ok(links)
    }

    async fn check_capacity(
        &self,
        tx: &mut CustomRecordsTx<P>,
        link: &PendingLink,
        from_id: Uuid,
        to_id: Uuid,
    ) -> Result<(), AppError> {
        let relationship = &link.relationship;
        let checks = [
            (relationship.single(RelationSide::From), false, from_id),
            (relationship.single(RelationSide::To), true, to_id),
        ];
        for (single, inverse, record_id) in checks {
            if single
                && self
                    .ports
                    .links()
                    .lock_links(tx, relationship.id, inverse, record_id)
                    .await?
                    > 0
            {
                return Err(AppError::invalid_fields(
                    INVALID_RECORD,
                    vec![FieldViolation::new(
                        format!("relations.{}", link.key),
                        "the linked record already has a record in this relationship",
                    )],
                ));
            }
        }
        Ok(())
    }

    fn derived_title(&self, object: &CustomObject, changes: &[FieldChange]) -> Option<String> {
        let path = custom_field_path(&object.data.title_field_key);
        changes
            .iter()
            .find(|change| change.field == path)
            .map(|change| title_from(&change.new))
    }

    async fn reload(
        &self,
        scope: &AccessScope,
        object: &CustomObject,
        record_id: Uuid,
    ) -> Result<CustomRecordDetails, AppError> {
        let record = self
            .ports
            .records()
            .find(object.id, record_id)
            .await?
            .ok_or_else(custom_record_not_found)?;
        self.details(scope, object, record).await
    }

    async fn details(
        &self,
        scope: &AccessScope,
        object: &CustomObject,
        record: CustomRecord,
    ) -> Result<CustomRecordDetails, AppError> {
        Ok(self
            .with_custom_fields(scope, object, vec![record])
            .await?
            .remove(0))
    }

    async fn with_custom_fields(
        &self,
        scope: &AccessScope,
        object: &CustomObject,
        records: Vec<CustomRecord>,
    ) -> Result<Vec<CustomRecordDetails>, AppError> {
        let record_type = RecordType::parse(object.record_type())?;
        let ids: Vec<Uuid> = records.iter().map(|record| record.id).collect();
        let mut values = CustomFieldReader::new(self.ports.fields())
            .load(scope, &record_type, &ids)
            .await?;
        Ok(records
            .into_iter()
            .map(|record| CustomRecordDetails {
                object: object.clone(),
                custom_fields: values.remove(&record.id).unwrap_or_default(),
                record,
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    async fn publish(
        &self,
        tx: &mut CustomRecordsTx<P>,
        scope: &AccessScope,
        record: &RecordRef,
        event_type: String,
        payload: Value,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        publish_change(
            self.ports.events(),
            self.ports.audit(),
            tx,
            scope,
            self.ports.clock().now(),
            ChangeRecord {
                record: record.clone(),
                event_type,
                payload,
                changes,
                ip,
            },
        )
        .await
    }
}
