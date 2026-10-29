use sqlx::Row;
use sqlx::mysql::MySqlRow;

use crate::domain::custom_objects::{
    Cardinality, CustomObject, CustomObjectData, OnDelete, RelationshipDefinition,
};
use crate::errors::AppError;
use crate::infrastructure::fields::mysql_definitions::uuid_column;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mapper::{instant, optional_user};

pub(crate) const OBJECT_COLUMNS: &str = "o.id, o.`key`, o.label_singular, o.label_plural, o.description, \
     o.icon, o.color, o.title_field_key, o.owner_enabled, o.is_active, o.position, o.created_by, \
     o.created_at, o.updated_at";

pub(crate) const RELATIONSHIP_COLUMNS: &str = "d.id, d.`key`, d.inverse_key, d.from_type, d.to_type, \
     d.cardinality, d.from_label, d.to_label, d.is_required, d.on_delete, d.created_by, \
     d.created_at, d.updated_at";

pub(crate) fn object_from_row(row: &MySqlRow) -> Result<CustomObject, AppError> {
    Ok(CustomObject {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        data: CustomObjectData {
            label_singular: row.try_get("label_singular")?,
            label_plural: row.try_get("label_plural")?,
            description: row.try_get("description")?,
            icon: row.try_get("icon")?,
            color: row.try_get("color")?,
            title_field_key: row.try_get("title_field_key")?,
            owner_enabled: row.try_get("owner_enabled")?,
            is_active: row.try_get("is_active")?,
        },
        position: row.try_get("position")?,
        created_by: optional_user(row, "created_by")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

pub(crate) fn relationship_from_row(row: &MySqlRow) -> Result<RelationshipDefinition, AppError> {
    let cardinality: String = row.try_get("cardinality")?;
    let on_delete: String = row.try_get("on_delete")?;
    Ok(RelationshipDefinition {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        inverse_key: row.try_get("inverse_key")?,
        from_type: row.try_get("from_type")?,
        to_type: row.try_get("to_type")?,
        cardinality: Cardinality::parse(&cardinality).ok_or_else(|| {
            mapper::invalid_column("relationship_definitions.cardinality", &cardinality)
        })?,
        from_label: row.try_get("from_label")?,
        to_label: row.try_get("to_label")?,
        required: row.try_get("is_required")?,
        on_delete: OnDelete::parse(&on_delete).ok_or_else(|| {
            mapper::invalid_column("relationship_definitions.on_delete", &on_delete)
        })?,
        created_by: optional_user(row, "created_by")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}
