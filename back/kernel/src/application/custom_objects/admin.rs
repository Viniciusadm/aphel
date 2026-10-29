use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::audit::{DiffOptions, diff_values};
use crate::application::custom_objects::{
    CUSTOM_OBJECTS_MAX, CustomObjectAdminPorts, CustomObjectRepository, CustomObjectsTx,
    RelationshipRepository, custom_record_definition,
};
use crate::application::fields::{
    FieldDefinitionRepository, RecordTypeRegistry, is_sql_identifier, normalize_color,
};
use crate::application::security::Clock;
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    TransactionManager, ensure_version,
};
use crate::domain::access::{AccessScope, Permission};
use crate::domain::custom_objects::{
    Cardinality, CustomObject, CustomObjectData, DEFAULT_TITLE_FIELD_KEY,
    MAX_OBJECT_DESCRIPTION_LENGTH, MAX_OBJECT_ICON_LENGTH, MAX_OBJECT_KEY_LENGTH,
    MAX_OBJECT_LABEL_LENGTH, MAX_RELATIONSHIP_KEY_LENGTH, MAX_RELATIONSHIP_LABEL_LENGTH, OnDelete,
    RelationSide, RelationshipDefinition, is_custom_record_type, is_object_key,
};
use crate::domain::fields::{FieldConfig, FieldDefinition, FieldKey, FieldType};
use crate::errors::{AppError, FieldViolation};
use crate::records::Patch;

pub const CUSTOM_OBJECT_RECORD_TYPE: RecordType = RecordType::from_static("custom_object");
pub const RELATIONSHIP_RECORD_TYPE: RecordType = RecordType::from_static("relationship");

pub const CUSTOM_OBJECT_CREATED: &str = "custom_object.created";
pub const CUSTOM_OBJECT_UPDATED: &str = "custom_object.updated";
pub const CUSTOM_OBJECT_DELETED: &str = "custom_object.deleted";
pub const CUSTOM_OBJECTS_REORDERED: &str = "custom_object.reordered";
pub const RELATIONSHIP_CREATED: &str = "relationship.created";
pub const RELATIONSHIP_UPDATED: &str = "relationship.updated";
pub const RELATIONSHIP_DELETED: &str = "relationship.deleted";

pub const CUSTOM_OBJECT_NOT_FOUND: &str = "custom_object_not_found";
pub const CUSTOM_OBJECT_KEY_TAKEN: &str = "custom_object_key_taken";
pub const TOO_MANY_CUSTOM_OBJECTS: &str = "too_many_custom_objects";
pub const CUSTOM_OBJECT_HAS_RECORDS: &str = "custom_object_has_records";
pub const RELATIONSHIP_NOT_FOUND: &str = "relationship_not_found";
pub const RELATIONSHIP_KEY_TAKEN: &str = "relationship_key_taken";
pub const TOO_MANY_RELATIONSHIPS: &str = "too_many_relationships";

pub const MAX_RELATIONSHIPS_PER_ORGANIZATION: usize = 200;
pub fn is_relatable_core_type(registry: &RecordTypeRegistry, record_type: &str) -> bool {
    registry
        .capabilities(record_type)
        .is_some_and(|capabilities| capabilities.relatable)
}

const INVALID_OBJECT: &str = "invalid custom object";
const INVALID_RELATIONSHIP: &str = "invalid relationship";
const TITLE_FIELD_LABEL: &str = "Nome";

pub fn custom_object_not_found() -> AppError {
    AppError::not_found_code(CUSTOM_OBJECT_NOT_FOUND, "custom object was not found")
}

pub fn relationship_not_found() -> AppError {
    AppError::not_found_code(RELATIONSHIP_NOT_FOUND, "relationship was not found")
}

pub fn is_title_field_type(field_type: FieldType) -> bool {
    matches!(
        field_type,
        FieldType::Text | FieldType::Email | FieldType::Phone | FieldType::Url
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateObjectCommand {
    pub key: String,
    pub label_singular: String,
    pub label_plural: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub owner_enabled: Option<bool>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateObjectCommand {
    pub label_singular: Option<String>,
    pub label_plural: Option<String>,
    pub description: Patch<String>,
    pub icon: Patch<String>,
    pub color: Patch<String>,
    pub title_field_key: Option<String>,
    pub owner_enabled: Option<bool>,
    pub is_active: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectView {
    pub object: CustomObject,
    pub record_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateRelationshipCommand {
    pub key: String,
    pub inverse_key: String,
    pub from_type: String,
    pub to_type: String,
    pub cardinality: String,
    pub from_label: String,
    pub to_label: String,
    pub required: bool,
    pub on_delete: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateRelationshipCommand {
    pub from_label: Option<String>,
    pub to_label: Option<String>,
    pub required: Option<bool>,
    pub on_delete: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipView {
    pub relationship: RelationshipDefinition,
    pub link_count: u64,
}

fn bounded(
    value: &str,
    max: usize,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let text = value.trim();
    if text.is_empty() || text.chars().count() > max {
        violations.push(FieldViolation::new(
            field,
            format!("must have between 1 and {max} characters"),
        ));
        return None;
    }
    Some(text.to_owned())
}

fn optional_text(
    value: Option<&str>,
    max: usize,
    field: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<String> {
    let text = value.map(str::trim).filter(|text| !text.is_empty())?;
    if text.chars().count() > max {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {max} characters"),
        ));
        return None;
    }
    Some(text.to_owned())
}

fn icon(value: Option<&str>, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let icon = value.map(str::trim).filter(|text| !text.is_empty())?;
    let valid = icon.len() <= MAX_OBJECT_ICON_LENGTH
        && icon.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if !valid {
        violations.push(FieldViolation::new(
            "icon",
            format!(
                "must have up to {MAX_OBJECT_ICON_LENGTH} lowercase letters, digits or hyphens"
            ),
        ));
        return None;
    }
    Some(icon.to_owned())
}

fn color(value: Option<&str>, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let text = value.map(str::trim).filter(|text| !text.is_empty())?;
    match normalize_color(text) {
        Some(color) => Some(color),
        None => {
            violations.push(FieldViolation::new("color", "must be a color like #1f6feb"));
            None
        }
    }
}

fn object_snapshot(object: &CustomObject) -> Value {
    json!({
        "key": object.key,
        "label_singular": object.data.label_singular,
        "label_plural": object.data.label_plural,
        "description": object.data.description,
        "icon": object.data.icon,
        "color": object.data.color,
        "title_field_key": object.data.title_field_key,
        "owner_enabled": object.data.owner_enabled,
        "is_active": object.data.is_active,
    })
}

fn relationship_snapshot(relationship: &RelationshipDefinition) -> Value {
    json!({
        "key": relationship.key,
        "inverse_key": relationship.inverse_key,
        "from_type": relationship.from_type,
        "to_type": relationship.to_type,
        "cardinality": relationship.cardinality.as_str(),
        "from_label": relationship.from_label,
        "to_label": relationship.to_label,
        "required": relationship.required,
        "on_delete": relationship.on_delete.as_str(),
    })
}

fn relation_key(value: &str, field: &str, violations: &mut Vec<FieldViolation>) -> Option<String> {
    let key = value.trim();
    if key.is_empty() || key.len() > MAX_RELATIONSHIP_KEY_LENGTH || !is_sql_identifier(key) {
        violations.push(FieldViolation::new(
            field,
            format!(
                "must have up to {MAX_RELATIONSHIP_KEY_LENGTH} lowercase letters, digits or underscores and start with a letter"
            ),
        ));
        return None;
    }
    Some(key.to_owned())
}

pub struct CustomObjectAdminUseCases<P> {
    ports: P,
}

impl<P> CustomObjectAdminUseCases<P>
where
    P: CustomObjectAdminPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn list(&self, scope: &AccessScope) -> Result<Vec<ObjectView>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let counts = self.ports.objects().record_counts().await?;
        Ok(self
            .ports
            .objects()
            .list()
            .await?
            .into_iter()
            .map(|object| ObjectView {
                record_count: counts.get(&object.id).copied().unwrap_or_default(),
                object,
            })
            .collect())
    }

    pub async fn visible(&self, scope: &AccessScope) -> Result<Vec<CustomObject>, AppError> {
        Ok(self
            .ports
            .objects()
            .list()
            .await?
            .into_iter()
            .filter(|object| {
                object.data.is_active
                    && object
                        .permission("read")
                        .is_some_and(|permission| scope.record_filter(&permission).is_ok())
            })
            .collect())
    }

    pub async fn get(&self, scope: &AccessScope, key: &str) -> Result<ObjectView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let object = self.existing(key).await?;
        let counts = self.ports.objects().record_counts().await?;
        Ok(ObjectView {
            record_count: counts.get(&object.id).copied().unwrap_or_default(),
            object,
        })
    }

    pub async fn create(
        &self,
        scope: &AccessScope,
        command: CreateObjectCommand,
    ) -> Result<ObjectView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let mut violations = Vec::new();
        let key = command.key.trim().to_owned();
        if !is_object_key(&key) {
            violations.push(FieldViolation::new(
                "key",
                format!(
                    "must have up to {MAX_OBJECT_KEY_LENGTH} lowercase letters or underscores and start with a letter"
                ),
            ));
        }
        let singular = bounded(
            &command.label_singular,
            MAX_OBJECT_LABEL_LENGTH,
            "label_singular",
            &mut violations,
        );
        let plural = bounded(
            &command.label_plural,
            MAX_OBJECT_LABEL_LENGTH,
            "label_plural",
            &mut violations,
        );
        let description = optional_text(
            command.description.as_deref(),
            MAX_OBJECT_DESCRIPTION_LENGTH,
            "description",
            &mut violations,
        );
        let icon = icon(command.icon.as_deref(), &mut violations);
        let color = color(command.color.as_deref(), &mut violations);
        let (Some(label_singular), Some(label_plural)) = (singular, plural) else {
            return Err(AppError::invalid_fields(INVALID_OBJECT, violations));
        };
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(INVALID_OBJECT, violations));
        }
        let existing = self.ports.objects().list().await?;
        if existing.iter().any(|object| object.key == key) {
            return Err(AppError::conflict_code(
                CUSTOM_OBJECT_KEY_TAKEN,
                format!("a custom object with the key {key} already exists"),
            ));
        }
        let limit = CUSTOM_OBJECTS_MAX;
        if existing.len() >= limit {
            return Err(AppError::validation_code(
                TOO_MANY_CUSTOM_OBJECTS,
                format!("the organization can have at most {limit} custom objects"),
            ));
        }
        let now = self.ports.clock().now();
        let object = CustomObject {
            id: Uuid::now_v7(),
            key,
            data: CustomObjectData {
                label_singular,
                label_plural,
                description,
                icon,
                color,
                title_field_key: DEFAULT_TITLE_FIELD_KEY.to_owned(),
                owner_enabled: command.owner_enabled.unwrap_or(true),
                is_active: true,
            },
            position: existing
                .iter()
                .map(|object| object.position)
                .max()
                .map_or(0, |position| position + 1),
            created_by: Some(scope.user_id().clone()),
            created_at: now,
            updated_at: now,
        };
        let definition = custom_record_definition(&object)?;
        let title_field = FieldDefinition {
            id: Uuid::now_v7(),
            record_type: object.record_type(),
            key: FieldKey::new(DEFAULT_TITLE_FIELD_KEY)
                .map_err(|_| AppError::Internal("invalid title field key".to_owned()))?,
            label: TITLE_FIELD_LABEL.to_owned(),
            field_type: FieldType::Text,
            is_system: false,
            column_name: None,
            is_required: true,
            is_unique: false,
            is_searchable: true,
            is_personal: false,
            is_sensitive: false,
            module_key: None,
            is_active: true,
            position: definition.system_fields.len() as i32,
            config: FieldConfig::default(),
            options: Vec::new(),
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.objects().insert(&mut tx, &object).await?;
        self.ports
            .objects()
            .seed_system_fields(&mut tx, &definition)
            .await?;
        self.ports
            .definitions()
            .insert(&mut tx, &title_field)
            .await?;
        self.ports
            .objects()
            .grant_default_permissions(&mut tx, &object.record_type())
            .await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(CUSTOM_OBJECT_RECORD_TYPE, object.id),
            CUSTOM_OBJECT_CREATED,
            json!({ "key": object.key, "record_type": object.record_type() }),
            diff_values(
                &Value::Null,
                &object_snapshot(&object),
                &DiffOptions::default(),
            )?,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(ObjectView {
            object,
            record_count: 0,
        })
    }

    pub async fn update(
        &self,
        scope: &AccessScope,
        key: &str,
        command: UpdateObjectCommand,
    ) -> Result<ObjectView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.existing(key).await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        let mut object = current.clone();
        let mut violations = Vec::new();
        if let Some(label) = &command.label_singular
            && let Some(label) = bounded(
                label,
                MAX_OBJECT_LABEL_LENGTH,
                "label_singular",
                &mut violations,
            )
        {
            object.data.label_singular = label;
        }
        if let Some(label) = &command.label_plural
            && let Some(label) = bounded(
                label,
                MAX_OBJECT_LABEL_LENGTH,
                "label_plural",
                &mut violations,
            )
        {
            object.data.label_plural = label;
        }
        if let Some(description) = &command.description {
            object.data.description = optional_text(
                description.as_deref(),
                MAX_OBJECT_DESCRIPTION_LENGTH,
                "description",
                &mut violations,
            );
        }
        if let Some(value) = &command.icon {
            object.data.icon = icon(value.as_deref(), &mut violations);
        }
        if let Some(value) = &command.color {
            object.data.color = color(value.as_deref(), &mut violations);
        }
        if let Some(owner_enabled) = command.owner_enabled {
            object.data.owner_enabled = owner_enabled;
        }
        if let Some(is_active) = command.is_active {
            object.data.is_active = is_active;
        }
        let mut title_field = None;
        if let Some(title_key) = &command.title_field_key {
            let title_key = title_key.trim();
            if title_key != object.data.title_field_key {
                let record_type = RecordType::parse(object.record_type())?;
                let field = self
                    .ports
                    .definitions()
                    .list(&record_type)
                    .await?
                    .into_iter()
                    .find(|field| {
                        field.key.as_str() == title_key
                            && !field.is_system
                            && field.is_active
                            && is_title_field_type(field.field_type)
                    });
                match field {
                    Some(field) => {
                        object.data.title_field_key = title_key.to_owned();
                        title_field = Some(field.id);
                    }
                    None => violations.push(FieldViolation::new(
                        "title_field_key",
                        "must be an active custom text, e-mail, phone or URL field of the object",
                    )),
                }
            }
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(INVALID_OBJECT, violations));
        }
        let changes = diff_values(
            &object_snapshot(&current),
            &object_snapshot(&object),
            &DiffOptions::default(),
        )?;
        let counts = self.ports.objects().record_counts().await?;
        let record_count = counts.get(&object.id).copied().unwrap_or_default();
        if changes.is_empty() {
            return Ok(ObjectView {
                object: current,
                record_count,
            });
        }
        object.updated_at = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.objects().update(&mut tx, &object).await?;
        if let Some(field_id) = title_field {
            self.ports
                .objects()
                .refresh_titles(&mut tx, &object, field_id)
                .await?;
        }
        self.ports
            .objects()
            .seed_system_fields(&mut tx, &custom_record_definition(&object)?)
            .await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(CUSTOM_OBJECT_RECORD_TYPE, object.id),
            CUSTOM_OBJECT_UPDATED,
            json!({ "key": object.key, "record_type": object.record_type() }),
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(ObjectView {
            object,
            record_count,
        })
    }

    pub async fn delete(
        &self,
        scope: &AccessScope,
        key: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let object = self.existing(key).await?;
        let counts = self.ports.objects().record_counts().await?;
        if counts.get(&object.id).copied().unwrap_or_default() > 0 {
            return Err(AppError::conflict_code(
                CUSTOM_OBJECT_HAS_RECORDS,
                "a custom object with records cannot be deleted; deactivate it instead",
            ));
        }
        let record_type = object.record_type();
        let relationships: Vec<RelationshipDefinition> = self
            .ports
            .relationships()
            .list()
            .await?
            .into_iter()
            .filter(|relationship| {
                relationship.from_type == record_type || relationship.to_type == record_type
            })
            .collect();
        let mut tx = self.ports.transactions().begin().await?;
        for relationship in &relationships {
            self.ports
                .relationships()
                .delete(&mut tx, relationship.id)
                .await?;
            self.publish(
                &mut tx,
                scope,
                RecordRef::new(RELATIONSHIP_RECORD_TYPE, relationship.id),
                RELATIONSHIP_DELETED,
                json!({ "key": relationship.key }),
                diff_values(
                    &relationship_snapshot(relationship),
                    &Value::Null,
                    &DiffOptions::default(),
                )?,
                ip.clone(),
            )
            .await?;
        }
        self.ports.objects().delete(&mut tx, &object).await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(CUSTOM_OBJECT_RECORD_TYPE, object.id),
            CUSTOM_OBJECT_DELETED,
            json!({ "key": object.key, "record_type": record_type }),
            diff_values(
                &object_snapshot(&object),
                &Value::Null,
                &DiffOptions::default(),
            )?,
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await
    }

    pub async fn reorder(
        &self,
        scope: &AccessScope,
        ordered_keys: &[String],
        ip: Option<String>,
    ) -> Result<Vec<CustomObject>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let objects = self.ports.objects().list().await?;
        let unique: BTreeSet<&str> = ordered_keys.iter().map(String::as_str).collect();
        let complete = unique.len() == ordered_keys.len()
            && ordered_keys.len() == objects.len()
            && objects
                .iter()
                .all(|object| unique.contains(object.key.as_str()));
        if !complete {
            return Err(AppError::invalid_fields(
                "invalid order",
                vec![FieldViolation::new(
                    "keys",
                    "must list every custom object key exactly once",
                )],
            ));
        }
        let ordered_ids: Vec<Uuid> = ordered_keys
            .iter()
            .filter_map(|key| objects.iter().find(|object| &object.key == key))
            .map(|object| object.id)
            .collect();
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.objects().reorder(&mut tx, &ordered_ids).await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        let old: Vec<&str> = objects.iter().map(|object| object.key.as_str()).collect();
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(CUSTOM_OBJECT_RECORD_TYPE, ordered_ids[0]),
            CUSTOM_OBJECTS_REORDERED,
            json!({ "keys": ordered_keys }),
            vec![FieldChange::new("order", json!(old), json!(ordered_keys))],
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        self.ports.objects().list().await
    }

    pub async fn list_relationships(
        &self,
        scope: &AccessScope,
        record_type: Option<&str>,
    ) -> Result<Vec<RelationshipView>, AppError> {
        let manager = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
        let counts = if manager {
            self.ports.relationships().link_counts().await?
        } else {
            Default::default()
        };
        let registry = self.ports.registry();
        Ok(self
            .ports
            .relationships()
            .list()
            .await?
            .into_iter()
            .filter(|relationship| {
                record_type.is_none_or(|record_type| {
                    relationship.from_type == record_type || relationship.to_type == record_type
                })
            })
            .filter(|relationship| {
                manager
                    || [&relationship.from_type, &relationship.to_type]
                        .into_iter()
                        .all(|record_type| {
                            registry.require(record_type).is_ok_and(|definition| {
                                scope.record_filter(&definition.read_permission).is_ok()
                            })
                        })
            })
            .map(|relationship| RelationshipView {
                link_count: counts.get(&relationship.id).copied().unwrap_or_default(),
                relationship,
            })
            .collect())
    }

    pub async fn create_relationship(
        &self,
        scope: &AccessScope,
        command: CreateRelationshipCommand,
    ) -> Result<RelationshipView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let mut violations = Vec::new();
        let key = relation_key(&command.key, "key", &mut violations);
        let inverse_key = relation_key(&command.inverse_key, "inverse_key", &mut violations);
        let from_type = self.relatable_type(&command.from_type, "from_type", &mut violations);
        let to_type = self.relatable_type(&command.to_type, "to_type", &mut violations);
        let cardinality = Cardinality::parse(command.cardinality.trim());
        if cardinality.is_none() {
            violations.push(FieldViolation::new(
                "cardinality",
                "must be one_to_one, one_to_many or many_to_many",
            ));
        }
        let on_delete = match command.on_delete.as_deref().map(str::trim) {
            None | Some("") => Some(OnDelete::Restrict),
            Some(value) => OnDelete::parse(value),
        };
        if on_delete.is_none() {
            violations.push(FieldViolation::new(
                "on_delete",
                "must be restrict or unlink",
            ));
        }
        let from_label = bounded(
            &command.from_label,
            MAX_RELATIONSHIP_LABEL_LENGTH,
            "from_label",
            &mut violations,
        );
        let to_label = bounded(
            &command.to_label,
            MAX_RELATIONSHIP_LABEL_LENGTH,
            "to_label",
            &mut violations,
        );
        if let (Some(key), Some(inverse_key), Some(from_type), Some(to_type)) =
            (&key, &inverse_key, &from_type, &to_type)
        {
            if from_type == to_type && key == inverse_key {
                violations.push(FieldViolation::new(
                    "inverse_key",
                    "must differ from key when both sides are the same record type",
                ));
            }
            self.check_relation_key(from_type, key, "key", &mut violations)
                .await?;
            self.check_relation_key(to_type, inverse_key, "inverse_key", &mut violations)
                .await?;
        }
        let (
            Some(key),
            Some(inverse_key),
            Some(from_type),
            Some(to_type),
            Some(cardinality),
            Some(on_delete),
            Some(from_label),
            Some(to_label),
        ) = (
            key,
            inverse_key,
            from_type,
            to_type,
            cardinality,
            on_delete,
            from_label,
            to_label,
        )
        else {
            return Err(AppError::invalid_fields(INVALID_RELATIONSHIP, violations));
        };
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(INVALID_RELATIONSHIP, violations));
        }
        let existing = self.ports.relationships().list().await?;
        if existing.iter().any(|relationship| relationship.key == key) {
            return Err(AppError::conflict_code(
                RELATIONSHIP_KEY_TAKEN,
                format!("a relationship with the key {key} already exists"),
            ));
        }
        if existing.len() >= MAX_RELATIONSHIPS_PER_ORGANIZATION {
            return Err(AppError::validation_code(
                TOO_MANY_RELATIONSHIPS,
                format!(
                    "an organization can have at most {MAX_RELATIONSHIPS_PER_ORGANIZATION} relationships"
                ),
            ));
        }
        let now = self.ports.clock().now();
        let relationship = RelationshipDefinition {
            id: Uuid::now_v7(),
            key,
            inverse_key,
            from_type,
            to_type,
            cardinality,
            from_label,
            to_label,
            required: command.required,
            on_delete,
            created_by: Some(scope.user_id().clone()),
            created_at: now,
            updated_at: now,
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .relationships()
            .insert(&mut tx, &relationship)
            .await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(RELATIONSHIP_RECORD_TYPE, relationship.id),
            RELATIONSHIP_CREATED,
            json!({ "key": relationship.key }),
            diff_values(
                &Value::Null,
                &relationship_snapshot(&relationship),
                &DiffOptions::default(),
            )?,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(RelationshipView {
            relationship,
            link_count: 0,
        })
    }

    pub async fn update_relationship(
        &self,
        scope: &AccessScope,
        key: &str,
        command: UpdateRelationshipCommand,
    ) -> Result<RelationshipView, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let current = self.relationship(key).await?;
        ensure_version(command.expected_updated_at, current.updated_at)?;
        let mut relationship = current.clone();
        let mut violations = Vec::new();
        if let Some(label) = &command.from_label
            && let Some(label) = bounded(
                label,
                MAX_RELATIONSHIP_LABEL_LENGTH,
                "from_label",
                &mut violations,
            )
        {
            relationship.from_label = label;
        }
        if let Some(label) = &command.to_label
            && let Some(label) = bounded(
                label,
                MAX_RELATIONSHIP_LABEL_LENGTH,
                "to_label",
                &mut violations,
            )
        {
            relationship.to_label = label;
        }
        if let Some(required) = command.required {
            relationship.required = required;
        }
        if let Some(value) = &command.on_delete {
            match OnDelete::parse(value.trim()) {
                Some(on_delete) => relationship.on_delete = on_delete,
                None => violations.push(FieldViolation::new(
                    "on_delete",
                    "must be restrict or unlink",
                )),
            }
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields(INVALID_RELATIONSHIP, violations));
        }
        let counts = self.ports.relationships().link_counts().await?;
        let link_count = counts.get(&relationship.id).copied().unwrap_or_default();
        let changes = diff_values(
            &relationship_snapshot(&current),
            &relationship_snapshot(&relationship),
            &DiffOptions::default(),
        )?;
        if changes.is_empty() {
            return Ok(RelationshipView {
                relationship: current,
                link_count,
            });
        }
        relationship.updated_at = self.ports.clock().now();
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .relationships()
            .update(&mut tx, &relationship)
            .await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(RELATIONSHIP_RECORD_TYPE, relationship.id),
            RELATIONSHIP_UPDATED,
            json!({ "key": relationship.key }),
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(RelationshipView {
            relationship,
            link_count,
        })
    }

    pub async fn delete_relationship(
        &self,
        scope: &AccessScope,
        key: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let relationship = self.relationship(key).await?;
        let mut tx = self.ports.transactions().begin().await?;
        let removed = self
            .ports
            .relationships()
            .delete(&mut tx, relationship.id)
            .await?;
        self.ports.objects().bump_schema_version(&mut tx).await?;
        let mut changes = diff_values(
            &relationship_snapshot(&relationship),
            &Value::Null,
            &DiffOptions::default(),
        )?;
        changes.push(FieldChange::new("links", json!(removed), Value::Null));
        self.publish(
            &mut tx,
            scope,
            RecordRef::new(RELATIONSHIP_RECORD_TYPE, relationship.id),
            RELATIONSHIP_DELETED,
            json!({ "key": relationship.key }),
            changes,
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn existing(&self, key: &str) -> Result<CustomObject, AppError> {
        self.ports
            .objects()
            .find_by_key(key.trim())
            .await?
            .ok_or_else(custom_object_not_found)
    }

    async fn relationship(&self, key: &str) -> Result<RelationshipDefinition, AppError> {
        self.ports
            .relationships()
            .list()
            .await?
            .into_iter()
            .find(|relationship| relationship.key == key.trim())
            .ok_or_else(relationship_not_found)
    }

    fn relatable_type(
        &self,
        value: &str,
        field: &str,
        violations: &mut Vec<FieldViolation>,
    ) -> Option<String> {
        let record_type = value.trim();
        let registry = self.ports.registry();
        let relatable = (is_custom_record_type(record_type)
            || is_relatable_core_type(registry, record_type))
            && registry.require(record_type).is_ok();
        if !relatable {
            violations.push(FieldViolation::new(
                field,
                "must be a relatable record type or an active custom object",
            ));
            return None;
        }
        Some(record_type.to_owned())
    }

    async fn check_relation_key(
        &self,
        record_type: &str,
        key: &str,
        field: &str,
        violations: &mut Vec<FieldViolation>,
    ) -> Result<(), AppError> {
        let Ok(definition) = self.ports.registry().require(record_type) else {
            return Ok(());
        };
        let custom_field_taken = self
            .ports
            .definitions()
            .list(&definition.record_type)
            .await?
            .iter()
            .any(|field| field.key.as_str() == key);
        if custom_field_taken
            || definition.relation_by_key(key).is_some()
            || definition.system_field_by_key(key).is_some()
        {
            violations.push(FieldViolation::new(
                field,
                format!("is already used by a field or relation of {record_type}"),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn publish(
        &self,
        tx: &mut CustomObjectsTx<P>,
        scope: &AccessScope,
        record: RecordRef,
        event_type: &str,
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
                record,
                event_type: event_type.to_owned(),
                payload,
                changes,
                ip,
            },
        )
        .await
    }
}

pub struct ChangeRecord {
    pub record: RecordRef,
    pub event_type: String,
    pub payload: Value,
    pub changes: Vec<FieldChange>,
    pub ip: Option<String>,
}

pub async fn publish_change<Tx, E, A>(
    events: &E,
    audit: &A,
    tx: &mut Tx,
    scope: &AccessScope,
    now: DateTime<Utc>,
    change: ChangeRecord,
) -> Result<(), AppError>
where
    Tx: Send,
    E: EventPublisher<Tx> + ?Sized,
    A: AuditLog<Tx> + ?Sized,
{
    let actor = Actor::from_scope(scope);
    events
        .publish(
            tx,
            DomainEvent::new(actor.clone(), change.event_type.clone(), now)
                .with_record(change.record.clone())
                .with_payload(change.payload),
        )
        .await?;
    audit
        .record(
            tx,
            AuditEntry::new(actor, change.event_type, now)
                .with_record(change.record)
                .with_changes(change.changes)
                .with_ip(change.ip),
        )
        .await
}

pub fn relation_sides(
    relationships: &[RelationshipDefinition],
    record_type: &str,
) -> Vec<(RelationshipDefinition, RelationSide)> {
    relationships
        .iter()
        .flat_map(|relationship| {
            relationship
                .sides_of(record_type)
                .into_iter()
                .map(|side| (relationship.clone(), side))
        })
        .collect()
}
