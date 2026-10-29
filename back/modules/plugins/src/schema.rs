use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::manifest::{FieldDef, PluginManifest};
use aphel_kernel::application::custom_objects::{CreateObjectCommand, CustomObjectAdminUseCases};
use aphel_kernel::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::{
    FieldConfig, FieldDefinition, FieldKey, FieldOption, FieldType,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::custom_objects::MySqlCustomObjectAdminPorts;
use aphel_kernel::infrastructure::fields::mysql_definitions::MySqlFieldDefinitionRepository;
use aphel_kernel::state::AppState;

const PLUGIN_FIELD_POSITION: i32 = 20_000;
const FIELD_TAKEN: &str = "plugin_field_conflict";
const OBJECT_TAKEN: &str = "plugin_object_conflict";

async fn bump_schema(state: &AppState) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE organization_settings SET custom_schema_version = custom_schema_version + 1 WHERE id = 1",
    )
    .execute(&state.db()?)
    .await?;
    state.record_types.invalidate();
    Ok(())
}

async fn object_owner(pool: &MySqlPool, key: &str) -> Result<Option<Option<String>>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT module_key FROM custom_objects WHERE `key` = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn apply(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PluginManifest,
) -> Result<(), AppError> {
    let pool = state.db()?;
    for entity in &manifest.entities {
        let object_key = manifest.object_key(&entity.key);
        match object_owner(&pool, &object_key).await? {
            Some(Some(owner)) if owner == manifest.key => {
                sqlx::query(
                    "UPDATE custom_objects SET label_singular = ?, label_plural = ?, description = ?, \
                     owner_enabled = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE `key` = ?",
                )
                .bind(&entity.label)
                .bind(&entity.label_plural)
                .bind(&entity.description)
                .bind(entity.owner_enabled)
                .bind(&object_key)
                .execute(&pool)
                .await?;
            }
            Some(_) => {
                return Err(AppError::conflict_code(
                    OBJECT_TAKEN,
                    format!("a custom object with the key {object_key} already exists"),
                ));
            }
            None => {
                CustomObjectAdminUseCases::new(MySqlCustomObjectAdminPorts::new(
                    pool.clone(),
                    state.registry().await?,
                    state.events.clone(),
                    state.audit.clone(),
                ))
                .create(
                    scope,
                    CreateObjectCommand {
                        key: object_key.clone(),
                        label_singular: entity.label.clone(),
                        label_plural: entity.label_plural.clone(),
                        description: entity.description.clone(),
                        icon: entity.icon.clone(),
                        color: None,
                        owner_enabled: Some(entity.owner_enabled),
                        ip: None,
                    },
                )
                .await?;
                sqlx::query("UPDATE custom_objects SET module_key = ? WHERE `key` = ?")
                    .bind(&manifest.key)
                    .bind(&object_key)
                    .execute(&pool)
                    .await?;
            }
        }
    }
    bump_schema(state).await?;
    let registry = state.registry().await?;
    let mut fields: Vec<(String, &FieldDef)> = Vec::new();
    for entity in &manifest.entities {
        let record_type = manifest.entity_record_type(&entity.key);
        fields.extend(
            entity
                .fields
                .iter()
                .map(|field| (record_type.clone(), field)),
        );
    }
    fields.extend(
        manifest
            .fields
            .iter()
            .map(|extension| (extension.record_type.clone(), &extension.field)),
    );
    let repository = MySqlFieldDefinitionRepository::new(pool.clone());
    let mut tx = pool.begin().await?;
    for (index, (record_type, field)) in fields.into_iter().enumerate() {
        let definition = definition(&registry, &manifest.key, &record_type, field, index)?;
        let existing: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT id, module_key FROM field_definitions WHERE record_type = ? AND `key` = ?",
        )
        .bind(&record_type)
        .bind(&field.key)
        .fetch_optional(&mut *tx)
        .await?;
        match existing {
            None => repository.insert(&mut tx, &definition).await?,
            Some((id, Some(owner))) if owner == manifest.key => {
                sqlx::query(
                    "UPDATE field_definitions SET label = ?, is_required = ?, is_searchable = ?, \
                     config = ?, is_active = TRUE WHERE id = ?",
                )
                .bind(&definition.label)
                .bind(definition.is_required)
                .bind(definition.is_searchable)
                .bind(sqlx::types::Json(config_json(&definition.config)))
                .bind(&id)
                .execute(&mut *tx)
                .await?;
                for (position, option) in definition.options.iter().enumerate() {
                    sqlx::query(
                        "INSERT INTO field_options (id, field_id, value, label, color, position, is_active) \
                         VALUES (?, ?, ?, ?, ?, ?, TRUE) AS new \
                         ON DUPLICATE KEY UPDATE label = new.label, color = new.color, is_active = TRUE",
                    )
                    .bind(Uuid::now_v7().to_string())
                    .bind(&id)
                    .bind(&option.value)
                    .bind(&option.label)
                    .bind(&option.color)
                    .bind(position as i32)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            Some(_) => {
                return Err(AppError::conflict_code(
                    FIELD_TAKEN,
                    format!("the field {} already exists on {record_type}", field.key),
                ));
            }
        }
    }
    tx.commit().await?;
    bump_schema(state).await
}

fn config_json(config: &FieldConfig) -> serde_json::Value {
    match &config.target_record_type {
        Some(target) => json!({ "target_record_type": target }),
        None => json!({}),
    }
}

fn definition(
    registry: &RecordTypeRegistry,
    plugin: &str,
    record_type: &str,
    field: &FieldDef,
    index: usize,
) -> Result<FieldDefinition, AppError> {
    let invalid = |message: String| {
        AppError::invalid_fields(
            "invalid plugin field",
            vec![FieldViolation::new(
                format!("{record_type}.{}", field.key),
                message,
            )],
        )
    };
    registry
        .require(record_type)
        .map_err(|_| invalid(format!("{record_type} is not a record type")))?;
    let field_type = FieldType::parse(&field.field_type)
        .ok_or_else(|| invalid(format!("{} is not a field type", field.field_type)))?;
    let config = FieldConfig {
        target_record_type: field.target.clone(),
        ..FieldConfig::default()
    };
    if let Some(target) = &field.target
        && registry.require(target).is_err()
    {
        return Err(invalid(format!("the target {target} is not a record type")));
    }
    if let Some(violation) = config.validate(field_type).into_iter().next() {
        return Err(invalid(violation.message));
    }
    Ok(FieldDefinition {
        id: Uuid::now_v7(),
        record_type: record_type.to_owned(),
        key: FieldKey::new(&field.key).map_err(|_| invalid("invalid key".to_owned()))?,
        label: field.label.clone(),
        field_type,
        is_system: false,
        column_name: None,
        is_required: field.required,
        is_unique: false,
        is_searchable: field.searchable,
        is_personal: false,
        is_sensitive: false,
        module_key: Some(plugin.to_owned()),
        is_active: true,
        position: PLUGIN_FIELD_POSITION + index as i32,
        config,
        options: field
            .options
            .iter()
            .enumerate()
            .map(|(position, option)| FieldOption {
                id: Uuid::now_v7(),
                value: option.value.clone(),
                label: option.label.clone(),
                color: option.color.clone(),
                position: position as i32,
                is_active: true,
            })
            .collect(),
    })
}

pub async fn set_active(
    state: &AppState,
    manifest: &PluginManifest,
    active: bool,
) -> Result<(), AppError> {
    let pool = state.db()?;
    sqlx::query("UPDATE custom_objects SET is_active = ? WHERE module_key = ?")
        .bind(active)
        .bind(&manifest.key)
        .execute(&pool)
        .await?;
    sqlx::query(
        "UPDATE field_definitions SET is_active = ? WHERE module_key = ? \
         AND record_type NOT LIKE 'co\\_%'",
    )
    .bind(active)
    .bind(&manifest.key)
    .execute(&pool)
    .await?;
    bump_schema(state).await
}

pub async fn purge(state: &AppState, manifest: &PluginManifest) -> Result<(), AppError> {
    let pool = state.db()?;
    let mut tx = pool.begin().await?;
    for record_type in manifest.entity_record_types() {
        for table in ["custom_field_values", "custom_field_multi_values"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE record_type = ?"
            )))
            .bind(&record_type)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("DELETE FROM field_definitions WHERE record_type = ?")
            .bind(&record_type)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM role_permissions WHERE permission LIKE ?")
            .bind(format!("{record_type}.%"))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM field_definitions WHERE module_key = ?")
        .bind(&manifest.key)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM custom_objects WHERE module_key = ?")
        .bind(&manifest.key)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    bump_schema(state).await
}
