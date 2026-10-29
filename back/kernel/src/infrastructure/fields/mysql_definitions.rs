use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlConnection, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::permissions::FieldAccessRule;
use crate::application::fields::{
    FIELD_KEY_TAKEN, FieldDefinitionRepository, OPTION_VALUE_TAKEN, RecordTypeRegistry,
    config_from_json, config_to_json,
};
use crate::application::shared::RecordType;
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::domain::fields::{FieldDefinition, FieldKey, FieldOption, resolve_declared_type};
use crate::errors::AppError;
use crate::infrastructure::fields::permissions::scope_access_rules;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlFieldDefinitionRepository {
    pool: MySqlPool,
}

impl MySqlFieldDefinitionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn definition_from_row(row: &MySqlRow) -> Result<FieldDefinition, AppError> {
    let key: String = row.try_get("key")?;
    let field_type: String = row.try_get("field_type")?;
    let Json(config): Json<Value> = row.try_get("config")?;
    let config = config_from_json(&config)
        .map_err(|_| mapper::invalid_column("config", &config.to_string()))?;
    let field_type = resolve_declared_type(&field_type, config.formula.as_ref())
        .ok_or_else(|| mapper::invalid_column("field_type", &field_type))?;
    Ok(FieldDefinition {
        id: uuid_column(row, "id")?,
        record_type: row.try_get("record_type")?,
        key: FieldKey::new(key.clone()).map_err(|_| mapper::invalid_column("key", &key))?,
        label: row.try_get("label")?,
        field_type,
        is_system: row.try_get("is_system")?,
        column_name: row.try_get("column_name")?,
        is_required: row.try_get("is_required")?,
        is_unique: row.try_get("is_unique")?,
        is_searchable: row.try_get("is_searchable")?,
        is_personal: row.try_get("is_personal")?,
        is_sensitive: row.try_get("is_sensitive")?,
        module_key: row.try_get("module_key")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        config,
        options: Vec::new(),
    })
}

fn option_from_row(row: &MySqlRow) -> Result<FieldOption, AppError> {
    Ok(FieldOption {
        id: uuid_column(row, "id")?,
        value: row.try_get("value")?,
        label: row.try_get("label")?,
        color: row.try_get("color")?,
        position: row.try_get("position")?,
        is_active: row.try_get("is_active")?,
    })
}

fn unique_violation(code: &'static str, message: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code(code, message)
        }
        _ => AppError::from(error),
    }
}

async fn insert_options(
    connection: &mut MySqlConnection,
    field_id: Uuid,
    options: &[FieldOption],
) -> Result<(), AppError> {
    if options.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::new(
        "INSERT INTO field_options (id, field_id, value, label, color, position, is_active) ",
    );
    builder.push_values(options, |mut values, option| {
        values
            .push_bind(option.id.to_string())
            .push_bind(field_id.to_string())
            .push_bind(option.value.clone())
            .push_bind(option.label.clone())
            .push_bind(option.color.clone())
            .push_bind(option.position)
            .push_bind(option.is_active);
    });
    builder
        .build()
        .execute(connection)
        .await
        .map_err(unique_violation(
            OPTION_VALUE_TAKEN,
            "the option value already exists",
        ))?;
    Ok(())
}

async fn reorder_rows(
    connection: &mut MySqlConnection,
    table: &str,
    scope_column: &str,
    scope_value: &str,
    ordered_ids: &[Uuid],
) -> Result<(), AppError> {
    if ordered_ids.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::new(format!("UPDATE {table} SET position = CASE id"));
    for (index, id) in ordered_ids.iter().enumerate() {
        builder
            .push(" WHEN ")
            .push_bind(id.to_string())
            .push(" THEN ")
            .push_bind(index as i32);
    }
    builder
        .push(format!(" ELSE position END WHERE {scope_column} = "))
        .push_bind(scope_value.to_owned());
    builder.build().execute(connection).await?;
    Ok(())
}

#[async_trait]
impl FieldDefinitionRepository for MySqlFieldDefinitionRepository {
    type Tx = MySqlTx;

    async fn access_rules(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Vec<FieldAccessRule>, AppError> {
        scope_access_rules(&self.pool, scope, record_type).await
    }

    async fn list(&self, record_type: &RecordType) -> Result<Vec<FieldDefinition>, AppError> {
        let rows = sqlx::query(
            "SELECT id, record_type, `key`, label, field_type, is_system, column_name, is_required, \
             is_unique, is_searchable, is_personal, is_sensitive, is_active, position, config, module_key \
             FROM field_definitions \
             WHERE record_type = ? ORDER BY position, created_at, id",
        )
        .bind(record_type.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut definitions = rows
            .iter()
            .map(definition_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        if definitions.is_empty() {
            return Ok(definitions);
        }

        let option_rows = sqlx::query(
            "SELECT o.id, o.field_id, o.value, o.label, o.color, o.position, o.is_active \
             FROM field_options o JOIN field_definitions f ON f.id = o.field_id \
             WHERE f.record_type = ? \
             ORDER BY o.position, o.created_at, o.id",
        )
        .bind(record_type.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut options: HashMap<Uuid, Vec<FieldOption>> = HashMap::new();
        for row in &option_rows {
            options
                .entry(uuid_column(row, "field_id")?)
                .or_default()
                .push(option_from_row(row)?);
        }
        for definition in &mut definitions {
            definition.options = options.remove(&definition.id).unwrap_or_default();
        }
        Ok(definitions)
    }

    async fn insert(&self, tx: &mut MySqlTx, field: &FieldDefinition) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO field_definitions (id, record_type, `key`, label, field_type, \
             is_system, column_name, is_required, is_unique, is_searchable, is_personal, is_sensitive, \
             is_active, position, config) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(field.id.to_string())
        .bind(&field.record_type)
        .bind(field.key.as_str())
        .bind(&field.label)
        .bind(field.type_name())
        .bind(field.is_system)
        .bind(&field.column_name)
        .bind(field.is_required)
        .bind(field.is_unique)
        .bind(field.is_searchable)
        .bind(field.is_personal)
        .bind(field.is_sensitive)
        .bind(field.is_active)
        .bind(field.position)
        .bind(Json(config_to_json(&field.config)))
        .execute(&mut **tx)
        .await
        .map_err(unique_violation(FIELD_KEY_TAKEN, "a field with this key already exists"))?;
        insert_options(tx, field.id, &field.options).await
    }

    async fn update(&self, tx: &mut MySqlTx, field: &FieldDefinition) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE field_definitions SET label = ?, field_type = ?, is_required = ?, is_unique = ?, \
             is_searchable = ?, is_personal = ?, is_sensitive = ?, is_active = ?, config = ?, updated_at = UTC_TIMESTAMP(6) \
             WHERE id = ?",
        )
        .bind(&field.label)
        .bind(field.type_name())
        .bind(field.is_required)
        .bind(field.is_unique)
        .bind(field.is_searchable)
        .bind(field.is_personal)
        .bind(field.is_sensitive)
        .bind(field.is_active)
        .bind(Json(config_to_json(&field.config)))
        .bind(field.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn reorder(
        &self,
        tx: &mut MySqlTx,
        record_type: &RecordType,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        reorder_rows(
            tx,
            "field_definitions",
            "record_type",
            record_type.as_str(),
            ordered_ids,
        )
        .await
    }

    async fn insert_option(
        &self,
        tx: &mut MySqlTx,
        field_id: Uuid,
        option: &FieldOption,
    ) -> Result<(), AppError> {
        insert_options(tx, field_id, std::slice::from_ref(option)).await?;
        touch_field(tx, field_id).await
    }

    async fn update_option(
        &self,
        tx: &mut MySqlTx,
        field_id: Uuid,
        option: &FieldOption,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE field_options SET label = ?, color = ?, is_active = ? \
             WHERE field_id = ? AND id = ?",
        )
        .bind(&option.label)
        .bind(&option.color)
        .bind(option.is_active)
        .bind(field_id.to_string())
        .bind(option.id.to_string())
        .execute(&mut **tx)
        .await?;
        touch_field(tx, field_id).await
    }

    async fn reorder_options(
        &self,
        tx: &mut MySqlTx,
        field_id: Uuid,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        reorder_rows(
            tx,
            "field_options",
            "field_id",
            &field_id.to_string(),
            ordered_ids,
        )
        .await?;
        touch_field(tx, field_id).await
    }

    async fn versions(
        &self,
        record_type: &RecordType,
    ) -> Result<HashMap<Uuid, DateTime<Utc>>, AppError> {
        let rows = sqlx::query(
            "SELECT id, updated_at FROM field_definitions \
             WHERE record_type = ?",
        )
        .bind(record_type.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let updated_at: NaiveDateTime = row.try_get("updated_at")?;
                Ok((uuid_column(row, "id")?, mapper::utc(updated_at)))
            })
            .collect()
    }
}

async fn touch_field(connection: &mut MySqlConnection, field_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE field_definitions SET updated_at = UTC_TIMESTAMP(6) \
         WHERE id = ?",
    )
    .bind(field_id.to_string())
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn seed_system_fields(
    connection: &mut MySqlConnection,
    registry: &RecordTypeRegistry,
) -> Result<(), AppError> {
    for definition in registry.definitions() {
        for (index, field) in definition.system_fields.iter().enumerate() {
            sqlx::query(
                "INSERT INTO field_definitions (id, record_type, `key`, label, field_type, \
                 is_system, column_name, is_required, is_unique, is_searchable, is_personal, is_sensitive, \
                 is_active, position, config) \
                 VALUES (?, ?, ?, ?, ?, TRUE, ?, ?, FALSE, ?, ?, ?, TRUE, ?, ?) AS new \
                 ON DUPLICATE KEY UPDATE \
                 field_type = IF(field_definitions.is_system, new.field_type, field_definitions.field_type), \
                 column_name = IF(field_definitions.is_system, new.column_name, field_definitions.column_name), \
                 is_required = IF(field_definitions.is_system, new.is_required, field_definitions.is_required), \
                 config = IF(field_definitions.is_system, new.config, field_definitions.config)",
            )
            .bind(Uuid::now_v7().to_string())
            .bind(definition.record_type.as_str())
            .bind(field.key.as_str())
            .bind(&field.label)
            .bind(field.field_type.as_str())
            .bind(&field.column)
            .bind(field.required)
            .bind(field.searchable)
            .bind(field.personal)
            .bind(field.sensitive)
            .bind(index as i32)
            .bind(Json(config_to_json(&field.config)))
            .execute(&mut *connection)
            .await?;

            if field.options.is_empty() {
                continue;
            }
            let field_id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM field_definitions \
                 WHERE record_type = ? AND `key` = ? AND is_system",
            )
            .bind(definition.record_type.as_str())
            .bind(field.key.as_str())
            .fetch_optional(&mut *connection)
            .await?;
            let Some(field_id) = field_id else {
                continue;
            };
            let mut builder = QueryBuilder::new(
                "INSERT IGNORE INTO field_options (id, field_id, value, label, color, position, is_active) ",
            );
            builder.push_values(
                field.options.iter().enumerate(),
                |mut values, (position, option)| {
                    values
                        .push_bind(Uuid::now_v7().to_string())
                        .push_bind(field_id.clone())
                        .push_bind(option.value.clone())
                        .push_bind(option.label.clone())
                        .push_bind(option.color.clone())
                        .push_bind(position as i32)
                        .push_bind(true);
                },
            );
            builder.build().execute(&mut *connection).await?;
        }
    }
    Ok(())
}

pub(crate) const MANAGED_FIELD_POSITION: i32 = 10_000;

async fn release_undeclared_fields(
    connection: &mut MySqlConnection,
    registry: &RecordTypeRegistry,
) -> Result<(), AppError> {
    let declared: Vec<(&str, &str)> = registry
        .definitions()
        .flat_map(|definition| {
            definition
                .managed_fields
                .iter()
                .map(|managed| (definition.record_type.as_str(), managed.spec.key.as_str()))
        })
        .collect();
    let mut builder = QueryBuilder::new(
        "UPDATE field_definitions SET module_key = NULL WHERE module_key IS NOT NULL",
    );
    if !declared.is_empty() {
        builder.push(" AND (record_type, `key`) NOT IN (");
        let mut pairs = builder.separated(", ");
        for (record_type, key) in declared {
            pairs.push("(");
            pairs.push_bind_unseparated(record_type.to_owned());
            pairs.push_unseparated(", ");
            pairs.push_bind_unseparated(key.to_owned());
            pairs.push_unseparated(")");
        }
        builder.push(")");
    }
    builder.build().execute(&mut *connection).await?;
    Ok(())
}

pub async fn seed_managed_fields(
    connection: &mut MySqlConnection,
    registry: &RecordTypeRegistry,
) -> Result<(), AppError> {
    release_undeclared_fields(connection, registry).await?;
    for definition in registry.definitions() {
        for (index, managed) in definition.managed_fields.iter().enumerate() {
            let spec = &managed.spec;
            sqlx::query(
                "INSERT INTO field_definitions (id, record_type, `key`, label, field_type, \
                 is_system, column_name, is_required, is_unique, is_searchable, is_personal, is_sensitive, \
                 is_active, position, config, module_key) \
                 VALUES (?, ?, ?, ?, ?, FALSE, NULL, FALSE, FALSE, FALSE, FALSE, FALSE, TRUE, ?, ?, ?) AS new \
                 ON DUPLICATE KEY UPDATE \
                 label = IF(field_definitions.module_key <=> new.module_key, new.label, field_definitions.label), \
                 field_type = IF(field_definitions.module_key <=> new.module_key, new.field_type, field_definitions.field_type), \
                 config = IF(field_definitions.module_key <=> new.module_key, new.config, field_definitions.config)",
            )
            .bind(Uuid::now_v7().to_string())
            .bind(definition.record_type.as_str())
            .bind(spec.key.as_str())
            .bind(&spec.label)
            .bind(spec.field_type.as_str())
            .bind(MANAGED_FIELD_POSITION + index as i32)
            .bind(Json(config_to_json(&spec.config)))
            .bind(managed.module)
            .execute(&mut *connection)
            .await?;
            if spec.options.is_empty() {
                continue;
            }
            let field_id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM field_definitions \
                 WHERE record_type = ? AND `key` = ? AND module_key = ?",
            )
            .bind(definition.record_type.as_str())
            .bind(spec.key.as_str())
            .bind(managed.module)
            .fetch_optional(&mut *connection)
            .await?;
            let Some(field_id) = field_id else {
                continue;
            };
            let mut builder = QueryBuilder::new(
                "INSERT IGNORE INTO field_options (id, field_id, value, label, color, position, is_active) ",
            );
            builder.push_values(
                spec.options.iter().enumerate(),
                |mut values, (position, option)| {
                    values
                        .push_bind(Uuid::now_v7().to_string())
                        .push_bind(field_id.clone())
                        .push_bind(option.value.clone())
                        .push_bind(option.label.clone())
                        .push_bind(option.color.clone())
                        .push_bind(position as i32)
                        .push_bind(true);
                },
            );
            builder.build().execute(&mut *connection).await?;
        }
    }
    Ok(())
}

pub async fn sync_system_fields(
    pool: &MySqlPool,
    registry: &RecordTypeRegistry,
) -> Result<u64, AppError> {
    if registry.definitions().all(|definition| {
        definition.system_fields.is_empty() && definition.managed_fields.is_empty()
    }) {
        return Ok(0);
    }
    let mut transaction = pool.begin().await?;
    seed_system_fields(&mut transaction, registry).await?;
    seed_managed_fields(&mut transaction, registry).await?;
    transaction.commit().await?;
    Ok(registry.definitions().count() as u64)
}
