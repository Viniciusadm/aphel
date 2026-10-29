use crate::application::fields::{
    CustomTypeLabels, RecordCapabilities, RecordRelation, RecordTypeDefinition, RecordTypeRegistry,
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

fn field(key: &str, label: &str, field_type: FieldType) -> Result<SystemField, AppError> {
    SystemField::new(key, label, field_type, key)
}

fn missing_permission(object: &CustomObject) -> AppError {
    AppError::Configuration(format!(
        "custom object {} has an invalid key for permissions",
        object.key
    ))
}

pub fn custom_record_definition(object: &CustomObject) -> Result<RecordTypeDefinition, AppError> {
    let record_type = RecordType::parse(object.record_type())?;
    let read = object
        .permission("read")
        .ok_or_else(|| missing_permission(object))?;
    let update = object
        .permission("update")
        .ok_or_else(|| missing_permission(object))?;
    let mut definition = RecordTypeDefinition::new(record_type, CUSTOM_RECORDS_TABLE, read)
        .capabilities(
            RecordCapabilities::default()
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
        .system_field(field(TITLE_FIELD, "Título", FieldType::Text)?.searchable());
    if object.data.owner_enabled {
        definition = definition.owner_column(OWNER_FIELD).system_field(field(
            OWNER_FIELD,
            "Responsável",
            FieldType::User,
        )?);
    }
    Ok(definition
        .system_field(field("created_by", "Criado por", FieldType::User)?)
        .system_field(field("created_at", "Criado em", FieldType::DateTime)?)
        .system_field(field("updated_at", "Atualizado em", FieldType::DateTime)?))
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
