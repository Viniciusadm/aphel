use crate::application::fields::{
    CustomTypeLabels, EntityDefinition, EntityTraits, RecordRelation, RecordTypeRegistry,
    RelationPath, SystemField,
};
use crate::application::shared::RecordType;
use crate::domain::custom_objects::{CustomObject, RelationSide, RelationshipDefinition};
use crate::domain::fields::FieldType;
use crate::errors::AppError;

pub const CUSTOM_RECORDS_TABLE: &str = "custom_records";
pub const TITLE_FIELD: &str = "title";
pub const OWNER_FIELD: &str = "owner_id";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomSchema {
    pub objects: Vec<CustomObject>,
    pub relationships: Vec<RelationshipDefinition>,
}

fn field(
    key: &str,
    label: impl Into<crate::application::fields::LocalizedLabels>,
    field_type: FieldType,
) -> Result<SystemField, AppError> {
    SystemField::new(key, label, field_type, key)
}

fn missing_permission(object: &CustomObject) -> AppError {
    AppError::Configuration(format!(
        "custom object {} has an invalid key for permissions",
        object.key
    ))
}

pub fn custom_record_definition(object: &CustomObject) -> Result<EntityDefinition, AppError> {
    let record_type = RecordType::parse(object.record_type())?;
    let read = object
        .permission("read")
        .ok_or_else(|| missing_permission(object))?;
    let update = object
        .permission("update")
        .ok_or_else(|| missing_permission(object))?;
    if let Some(declaration) = &object.entity_definition {
        let declaration: crate::application::fields::EntityDeclaration =
            serde_json::from_value(declaration.clone()).map_err(|error| {
                AppError::Configuration(format!("invalid stored entity: {error}"))
            })?;
        if declaration.storage == crate::application::fields::EntityStorage::Managed {
            let owner = object.module_key.as_deref().ok_or_else(|| {
                AppError::Configuration("managed entities need an owner".to_owned())
            })?;
            let mut definition =
                declaration.definition(&format!("{owner}.{}", declaration.key), read)?;
            definition.record_type = record_type;
            definition.custom = Some(CustomTypeLabels {
                singular: object.data.label_singular.clone(),
                plural: object.data.label_plural.clone(),
                icon: object.data.icon.clone(),
                color: object.data.color.clone(),
            });
            return Ok(definition);
        }
    }
    let mut definition = EntityDefinition::new(record_type, CUSTOM_RECORDS_TABLE, read)
        .capabilities(
            EntityTraits::default()
                .importable()
                .exportable()
                .restore_order(3),
        )
        .update_permission(update)
        .soft_delete_column("deleted_at")
        .archive_column("archived_at")
        .partitioned("object_id", object.id)
        .title_field(TITLE_FIELD)
        .custom_labels(CustomTypeLabels {
            singular: object.data.label_singular.clone(),
            plural: object.data.label_plural.clone(),
            icon: object.data.icon.clone(),
            color: object.data.color.clone(),
        })
        .system_field(field(TITLE_FIELD, crate::labels("Title"), FieldType::Text)?.searchable());
    definition.storage = crate::application::fields::EntityStorage::Dynamic;
    if let Some(declaration) = &object.entity_definition {
        let declaration: crate::application::fields::EntityDeclaration =
            serde_json::from_value(declaration.clone())
                .map_err(|error| AppError::Configuration(error.to_string()))?;
        definition.capabilities.traits.extend(declaration.traits);
        definition.references = declaration.references;
    }
    definition.key = match &object.module_key {
        Some(owner) => format!(
            "{owner}.{}",
            object
                .key
                .strip_prefix(&format!("{owner}_"))
                .unwrap_or(&object.key)
        ),
        None => format!("custom.{}", object.key),
    };
    if object.data.owner_enabled {
        definition = definition.owner_column(OWNER_FIELD).system_field(field(
            OWNER_FIELD,
            crate::labels("Owner"),
            FieldType::User,
        )?);
    }
    Ok(definition
        .system_field(field(
            "created_by",
            crate::labels("Created by"),
            FieldType::User,
        )?)
        .system_field(field(
            "created_at",
            crate::labels("Created at"),
            FieldType::DateTime,
        )?)
        .system_field(field(
            "updated_at",
            crate::labels("Updated at"),
            FieldType::DateTime,
        )?))
}

pub fn relationship_relation(
    relationship: &RelationshipDefinition,
    side: RelationSide,
) -> Result<RecordRelation, AppError> {
    Ok(RecordRelation::new(
        relationship.relation_key(side),
        relationship.label(side),
        RecordType::parse(relationship.target_type(side))?,
    )
    .path(RelationPath::Relationship {
        relationship_id: relationship.id,
        inverse: side == RelationSide::To,
        single: relationship.single(side),
    }))
}

pub fn build_registry(
    builtin: &RecordTypeRegistry,
    schema: &CustomSchema,
) -> Result<RecordTypeRegistry, AppError> {
    let mut registry = builtin.clone();
    for object in schema.objects.iter().filter(|object| object.data.is_active) {
        match custom_record_definition(object).and_then(|definition| {
            registry
                .clone()
                .register(definition)
                .map(|extended| registry = extended)
        }) {
            Ok(()) => {}
            Err(error) => {
                tracing::warn!(object = %object.key, %error, "custom object skipped from the registry");
            }
        }
    }
    for relationship in &schema.relationships {
        let (Ok(from), Ok(to)) = (
            RecordType::parse(relationship.from_type.as_str()),
            RecordType::parse(relationship.to_type.as_str()),
        ) else {
            continue;
        };
        if registry.get(&from).is_none() || registry.get(&to).is_none() {
            continue;
        }
        for side in [RelationSide::From, RelationSide::To] {
            let source = if side == RelationSide::From {
                &from
            } else {
                &to
            };
            let added = relationship_relation(relationship, side)
                .and_then(|relation| registry.add_relation(source, relation));
            if let Err(error) = added {
                tracing::warn!(relationship = %relationship.key, %error, "relationship side skipped from the registry");
            }
        }
    }
    Ok(registry)
}
