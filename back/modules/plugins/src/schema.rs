use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::manifest::{FieldDef, PluginManifest};
use aphel_kernel::application::custom_objects::{
    CreateObjectCommand, CustomObjectAdminUseCases, custom_record_definition,
};
use aphel_kernel::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::custom_objects::{
    CustomObject, CustomObjectData, DEFAULT_TITLE_FIELD_KEY,
};
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
const FIELD_TYPE_CHANGED: &str = "plugin_field_type_changed";

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
    ensure_compatible(&pool, manifest).await?;
    preflight(&pool, manifest).await?;
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
    let fields = declared_fields(manifest);
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
                deactivate_undeclared_options(&mut tx, &id, &definition.options).await?;
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
    deactivate_undeclared(&pool, manifest).await?;
    bump_schema(state).await
}

fn reserved_keys(manifest: &PluginManifest, entity_key: &str) -> Result<Vec<String>, AppError> {
    let Some(entity) = manifest
        .entities
        .iter()
        .find(|entity| entity.key == entity_key)
    else {
        return Ok(Vec::new());
    };
    let now = chrono::Utc::now();
    let object = CustomObject {
        id: Uuid::nil(),
        key: manifest.object_key(&entity.key),
        data: CustomObjectData {
            label_singular: entity.label.clone(),
            label_plural: entity.label_plural.clone(),
            description: None,
            icon: None,
            color: None,
            title_field_key: DEFAULT_TITLE_FIELD_KEY.to_owned(),
            owner_enabled: entity.owner_enabled,
            is_active: true,
        },
        position: 0,
        module_key: Some(manifest.key.clone()),
        created_by: None,
        created_at: now,
        updated_at: now,
    };
    let mut keys: Vec<String> = custom_record_definition(&object)?
        .system_fields
        .iter()
        .map(|field| field.key.as_str().to_owned())
        .collect();
    keys.push(DEFAULT_TITLE_FIELD_KEY.to_owned());
    Ok(keys)
}

async fn preflight(pool: &MySqlPool, manifest: &PluginManifest) -> Result<(), AppError> {
    let mut new_entities = Vec::new();
    for entity in &manifest.entities {
        let object_key = manifest.object_key(&entity.key);
        match object_owner(pool, &object_key).await? {
            Some(Some(owner)) if owner == manifest.key => {}
            Some(_) => {
                return Err(AppError::conflict_code(
                    OBJECT_TAKEN,
                    format!("a custom object with the key {object_key} already exists"),
                ));
            }
            None => new_entities.push((manifest.entity_record_type(&entity.key), &entity.key)),
        }
    }
    for (record_type, field) in declared_fields(manifest) {
        let taken = match new_entities
            .iter()
            .find(|(entity_type, _)| entity_type == &record_type)
        {
            Some((_, entity_key)) => reserved_keys(manifest, entity_key)?.contains(&field.key),
            None => {
                let owner: Option<Option<String>> = sqlx::query_scalar(
                    "SELECT module_key FROM field_definitions WHERE record_type = ? AND `key` = ?",
                )
                .bind(&record_type)
                .bind(&field.key)
                .fetch_optional(pool)
                .await?;
                owner.is_some_and(|owner| owner.as_deref() != Some(manifest.key.as_str()))
            }
        };
        if taken {
            return Err(AppError::conflict_code(
                FIELD_TAKEN,
                format!("the field {} already exists on {record_type}", field.key),
            ));
        }
    }
    Ok(())
}

fn declared_fields(manifest: &PluginManifest) -> Vec<(String, &FieldDef)> {
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
    fields
}

async fn ensure_compatible(pool: &MySqlPool, manifest: &PluginManifest) -> Result<(), AppError> {
    let existing: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT record_type, `key`, field_type FROM field_definitions WHERE module_key = ?",
    )
    .bind(&manifest.key)
    .fetch_all(pool)
    .await?;
    for (record_type, field) in declared_fields(manifest) {
        let Some(new_type) = FieldType::parse(&field.field_type) else {
            continue;
        };
        if let Some((_, _, old_type)) = existing
            .iter()
            .find(|(existing_type, key, _)| existing_type == &record_type && key == &field.key)
            && old_type != new_type.as_str()
        {
            return Err(AppError::conflict_code(
                FIELD_TYPE_CHANGED,
                format!(
                    "the field {} on {record_type} is {old_type} and the new version declares it as {}; the type of a field cannot change, declare a field with a new key",
                    field.key,
                    new_type.as_str()
                ),
            ));
        }
    }
    Ok(())
}

async fn deactivate_undeclared_options(
    tx: &mut sqlx::Transaction<'static, sqlx::MySql>,
    field_id: &str,
    options: &[FieldOption],
) -> Result<(), AppError> {
    let mut builder = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "UPDATE field_options SET is_active = FALSE WHERE field_id = ",
    );
    builder.push_bind(field_id.to_owned());
    if !options.is_empty() {
        builder.push(" AND value NOT IN (");
        let mut values = builder.separated(", ");
        for option in options {
            values.push_bind(option.value.clone());
        }
        builder.push(")");
    }
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

async fn deactivate_undeclared(
    pool: &MySqlPool,
    manifest: &PluginManifest,
) -> Result<(), AppError> {
    let declared: Vec<(String, String)> = declared_fields(manifest)
        .into_iter()
        .map(|(record_type, field)| (record_type, field.key.clone()))
        .collect();
    let existing: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, record_type, `key` FROM field_definitions WHERE module_key = ? AND is_active = TRUE",
    )
    .bind(&manifest.key)
    .fetch_all(pool)
    .await?;
    for (id, record_type, key) in existing {
        if !declared.contains(&(record_type, key)) {
            sqlx::query("UPDATE field_definitions SET is_active = FALSE WHERE id = ?")
                .bind(&id)
                .execute(pool)
                .await?;
        }
    }
    let objects: Vec<String> = manifest
        .entities
        .iter()
        .map(|entity| manifest.object_key(&entity.key))
        .collect();
    let owned: Vec<String> = sqlx::query_scalar(
        "SELECT `key` FROM custom_objects WHERE module_key = ? AND is_active = TRUE",
    )
    .bind(&manifest.key)
    .fetch_all(pool)
    .await?;
    for key in owned {
        if !objects.contains(&key) {
            sqlx::query("UPDATE custom_objects SET is_active = FALSE WHERE `key` = ?")
                .bind(&key)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
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
    if active {
        deactivate_undeclared(&pool, manifest).await?;
    }
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
