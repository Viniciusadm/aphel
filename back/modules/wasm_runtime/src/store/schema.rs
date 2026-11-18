use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::config_to_json;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::fields::{FieldDefinition, FieldOption};

pub(crate) async fn bump_schema_version(pool: &Db) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE organization_settings SET custom_schema_version = custom_schema_version + 1 WHERE id = 1",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn object_owner(pool: &Db, key: &str) -> Result<Option<Option<String>>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT module_key FROM custom_objects WHERE `key` = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?,
    )
}

pub(crate) async fn object_entity_definition(
    pool: &Db,
    key: &str,
) -> Result<Option<Option<Value>>, AppError> {
    let stored: Option<Option<sqlx::types::Json<Value>>> =
        sqlx::query_scalar("SELECT entity_definition FROM custom_objects WHERE `key` = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(stored.map(|definition| definition.map(|json| json.0)))
}

pub(crate) struct ObjectLabels<'a> {
    pub label_singular: &'a str,
    pub label_plural: &'a str,
    pub description: &'a Option<String>,
    pub owner_enabled: bool,
    pub internal: bool,
}

pub(crate) async fn update_object_labels(
    pool: &Db,
    key: &str,
    labels: ObjectLabels<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE custom_objects SET label_singular = ?, label_plural = ?, description = ?, \
         owner_enabled = ?, is_internal = ?, updated_at = CURRENT_TIMESTAMP(6) WHERE `key` = ?",
    )
    .bind(labels.label_singular)
    .bind(labels.label_plural)
    .bind(labels.description)
    .bind(labels.owner_enabled)
    .bind(labels.internal)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn set_entity_definition<T: serde::Serialize + Sync>(
    pool: &Db,
    object_key: &str,
    module_key: &str,
    entity: &T,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE custom_objects SET entity_definition = ? WHERE `key` = ? AND module_key = ?",
    )
    .bind(sqlx::types::Json(entity))
    .bind(object_key)
    .bind(module_key)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn field_id_and_owner(
    tx: &mut Tx,
    record_type: &str,
    key: &str,
) -> Result<Option<(String, Option<String>)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id, module_key FROM field_definitions WHERE record_type = ? AND `key` = ?",
    )
    .bind(record_type)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?)
}

pub(crate) async fn update_plugin_field(
    tx: &mut Tx,
    id: &str,
    module_key: &str,
    definition: &FieldDefinition,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE field_definitions SET module_key = ?, is_system = ?, column_name = ?, label = ?, is_required = ?, is_unique = ?, \
         is_searchable = ?, is_personal = ?, is_sensitive = ?, config = ?, \
         is_active = TRUE WHERE id = ?",
    )
    .bind(module_key)
    .bind(definition.is_system)
    .bind(&definition.column_name)
    .bind(&definition.label)
    .bind(definition.is_required)
    .bind(definition.is_unique)
    .bind(definition.is_searchable)
    .bind(definition.is_personal)
    .bind(definition.is_sensitive)
    .bind(sqlx::types::Json(config_to_json(&definition.config)))
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn upsert_field_option(
    tx: &mut Tx,
    field_id: &str,
    option: &FieldOption,
    position: i32,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO field_options (id, field_id, value, label, color, position, is_active) \
         VALUES (?, ?, ?, ?, ?, ?, TRUE) AS new \
         ON DUPLICATE KEY UPDATE label = new.label, color = new.color, is_active = TRUE",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(field_id)
    .bind(&option.value)
    .bind(&option.label)
    .bind(&option.color)
    .bind(position)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn deactivate_undeclared_options(
    tx: &mut Tx,
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

pub(crate) async fn field_claim(
    pool: &Db,
    record_type: &str,
    key: &str,
) -> Result<Option<(Option<String>, bool, Option<String>)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT module_key, is_system, column_name FROM field_definitions WHERE record_type = ? AND `key` = ?",
    )
    .bind(record_type)
    .bind(key)
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn module_field_types(
    pool: &Db,
    module_key: &str,
) -> Result<Vec<(String, String, String)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT record_type, `key`, field_type FROM field_definitions WHERE module_key = ?",
    )
    .bind(module_key)
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn active_module_fields(
    pool: &Db,
    module_key: &str,
) -> Result<Vec<(String, String, String)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id, record_type, `key` FROM field_definitions WHERE module_key = ? AND is_active = TRUE",
    )
    .bind(module_key)
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn deactivate_field(pool: &Db, id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE field_definitions SET is_active = FALSE WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn active_module_objects(
    pool: &Db,
    module_key: &str,
) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT `key` FROM custom_objects WHERE module_key = ? AND is_active = TRUE",
    )
    .bind(module_key)
    .fetch_all(pool)
    .await?)
}

pub(crate) async fn deactivate_object(pool: &Db, key: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE custom_objects SET is_active = FALSE WHERE `key` = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn set_module_schema_active(
    pool: &Db,
    module_key: &str,
    active: bool,
) -> Result<(), AppError> {
    sqlx::query("UPDATE custom_objects SET is_active = ? WHERE module_key = ?")
        .bind(active)
        .bind(module_key)
        .execute(pool)
        .await?;
    sqlx::query(
        "UPDATE field_definitions SET is_active = ? WHERE module_key = ? \
         AND record_type NOT LIKE 'co\\_%'",
    )
    .bind(active)
    .bind(module_key)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn purge_record_type(
    tx: &mut Tx,
    record_type: &str,
) -> Result<Vec<Uuid>, AppError> {
    let files: Vec<String> =
        sqlx::query_scalar("SELECT file_id FROM attachments WHERE record_type = ?")
            .bind(record_type)
            .fetch_all(&mut **tx)
            .await?;
    for table in ["attachments", "record_shares"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ?"
        )))
        .bind(record_type)
        .execute(&mut **tx)
        .await?;
    }
    for table in ["custom_field_values", "custom_field_multi_values"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ?"
        )))
        .bind(record_type)
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("DELETE FROM field_definitions WHERE record_type = ?")
        .bind(record_type)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM role_permissions WHERE permission LIKE ?")
        .bind(format!("{record_type}.%"))
        .execute(&mut **tx)
        .await?;
    Ok(files
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect())
}

pub(crate) async fn delete_module_schema(tx: &mut Tx, module_key: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM field_definitions WHERE module_key = ?")
        .bind(module_key)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM custom_objects WHERE module_key = ?")
        .bind(module_key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn drop_table(pool: &Db, table: &str) -> Result<(), AppError> {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TABLE IF EXISTS `{table}`"
    )))
    .execute(pool)
    .await?;
    Ok(())
}
