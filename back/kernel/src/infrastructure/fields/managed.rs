use crate::application::fields::{EntityDefinition, EntityStorage, managed_table};
use crate::domain::fields::{FieldType, is_identifier};
use crate::errors::AppError;
use sqlx::{MySqlPool, Row};

pub fn column_type(field_type: FieldType) -> &'static str {
    match field_type.base_type() {
        FieldType::Text => "VARCHAR(1024)",
        FieldType::LongText => "TEXT",
        FieldType::Number | FieldType::Currency => "DECIMAL(24,6)",
        FieldType::Date => "DATE",
        FieldType::DateTime => "DATETIME(6)",
        FieldType::Checkbox => "BOOLEAN",
        FieldType::MultiSelect => "JSON",
        FieldType::Select => "VARCHAR(64)",
        FieldType::Url => "VARCHAR(2048)",
        FieldType::Email => "VARCHAR(320)",
        FieldType::Phone => "VARCHAR(32)",
        FieldType::User | FieldType::Record => "CHAR(36)",
        FieldType::Custom { .. } => unreachable!("native field representation"),
    }
}

fn field_column_type(field: &crate::application::fields::SystemField) -> String {
    if field.field_type.base_type() == FieldType::Text {
        format!("VARCHAR({})", field.config.max_length.unwrap_or(1024))
    } else {
        column_type(field.field_type).to_owned()
    }
}

pub fn managed_schema(definition: &EntityDefinition) -> Result<Vec<String>, AppError> {
    definition.check()?;
    if definition.storage != EntityStorage::Managed {
        return Err(AppError::Configuration(
            "only managed entities have generated schemas".to_owned(),
        ));
    }
    if definition.table != managed_table(&definition.key) {
        return Err(AppError::Configuration(
            "managed storage must use its own generated table".to_owned(),
        ));
    }
    let mut columns = vec!["`id` CHAR(36) NOT NULL PRIMARY KEY".to_owned()];
    for field in definition
        .system_fields
        .iter()
        .filter(|field| !field.is_computed() && field.contributed_by.is_none())
    {
        if [
            "id",
            "created_at",
            "updated_at",
            "created_by",
            "deleted_at",
            "archived_at",
        ]
        .contains(&field.column.as_str())
        {
            if field.writable {
                return Err(AppError::Configuration(format!(
                    "managed field {} uses a reserved writable column",
                    field.key
                )));
            }
            continue;
        }
        columns.push(format!(
            "`{}` {} {}",
            field.column,
            field_column_type(field),
            if field.required { "NOT NULL" } else { "NULL" }
        ));
    }
    columns.extend([
        "`created_by` CHAR(36) NULL".to_owned(),
        "`created_at` DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)".to_owned(),
        "`updated_at` DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)".to_owned(),
    ]);
    for column in [&definition.deleted_column, &definition.archived_column]
        .into_iter()
        .flatten()
    {
        columns.push(format!("`{column}` DATETIME(6) NULL"));
    }
    let mut statements = vec![format!(
        "CREATE TABLE IF NOT EXISTS `{}` ({}) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci",
        definition.table,
        columns.join(", ")
    )];
    let mut names = std::collections::BTreeSet::new();
    for index in &definition.indexes {
        if !is_identifier(&index.key, 64)
            || index.key == "primary"
            || !names.insert(&index.key)
            || index.fields.is_empty()
            || index.fields.len() > 8
        {
            return Err(AppError::Configuration(format!(
                "invalid entity index {}",
                index.key
            )));
        }
        let mut width = 0;
        let columns = index
            .fields
            .iter()
            .map(|key| {
                let field = definition
                    .system_field_by_key(key)
                    .ok_or_else(|| AppError::Configuration(format!("unknown index field {key}")))?;
                if field.is_computed()
                    || field.contributed_by.is_some()
                    || matches!(
                        field.field_type.base_type(),
                        FieldType::LongText | FieldType::MultiSelect | FieldType::Url
                    )
                {
                    return Err(AppError::Configuration(format!(
                        "field {key} cannot be indexed"
                    )));
                }
                width += match field.field_type.base_type() {
                    FieldType::Text => field.config.max_length.unwrap_or(1024),
                    FieldType::Email => 320,
                    FieldType::Select => 64,
                    FieldType::User | FieldType::Record => 36,
                    FieldType::Phone => 32,
                    _ => 16,
                };
                Ok(format!("`{}`", field.column))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        if width > 768 {
            return Err(AppError::Configuration(format!(
                "index {} exceeds the storage key limit; declare bounded text fields",
                index.key
            )));
        }
        statements.push(format!(
            "CREATE {}INDEX `{}` ON `{}` ({})",
            if index.unique { "UNIQUE " } else { "" },
            index.key,
            definition.table,
            columns.join(", ")
        ));
    }
    Ok(statements)
}

pub async fn ensure_managed_schema(
    pool: &MySqlPool,
    definition: &EntityDefinition,
) -> Result<(), AppError> {
    let statements = managed_schema(definition)?;
    let mut connection = pool.acquire().await?;
    connection.close_on_drop();
    let lock = format!("aphel:schema:{}", definition.table);
    let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 30)")
        .bind(&lock)
        .fetch_one(&mut *connection)
        .await?;
    if acquired != Some(1) {
        return Err(AppError::Persistence(
            "could not lock the entity schema".to_owned(),
        ));
    }
    let result = reconcile(&mut connection, definition, &statements).await;
    let release: Result<Option<i64>, _> = sqlx::query_scalar("SELECT RELEASE_LOCK(?)")
        .bind(&lock)
        .fetch_one(&mut *connection)
        .await;
    result?;
    release?;
    Ok(())
}

async fn reconcile(
    connection: &mut sqlx::MySqlConnection,
    definition: &EntityDefinition,
    statements: &[String],
) -> Result<(), AppError> {
    sqlx::raw_sql(sqlx::AssertSqlSafe(statements[0].as_str()))
        .execute(&mut *connection)
        .await?;
    let stored = sqlx::query("SELECT COLUMN_NAME, COLUMN_TYPE FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?")
            .bind(&definition.table).fetch_all(&mut *connection).await?;
    let stored = stored
        .iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("COLUMN_NAME")?,
                row.try_get::<String, _>("COLUMN_TYPE")?,
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, sqlx::Error>>()?;
    for field in definition
        .system_fields
        .iter()
        .filter(|field| !field.is_computed() && field.contributed_by.is_none())
    {
        if let Some(existing) = stored.get(&field.column) {
            let expected = field_column_type(field)
                .to_ascii_lowercase()
                .replace("boolean", "tinyint(1)");
            if existing.to_ascii_lowercase() != expected {
                return Err(AppError::Configuration(format!(
                    "changing the storage of field {} needs a data migration",
                    field.key
                )));
            }
        } else {
            if field.required {
                return Err(AppError::Configuration(format!(
                    "adding required field {} needs a data migration",
                    field.key
                )));
            }
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "ALTER TABLE `{}` ADD COLUMN `{}` {} NULL",
                definition.table,
                field.column,
                field_column_type(field)
            )))
            .execute(&mut *connection)
            .await?;
        }
    }
    for (index, statement) in definition.indexes.iter().zip(&statements[1..]) {
        let rows = sqlx::query("SELECT COLUMN_NAME, NON_UNIQUE FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND INDEX_NAME = ? ORDER BY SEQ_IN_INDEX")
                .bind(&definition.table).bind(&index.key).fetch_all(&mut *connection).await?;
        if rows.is_empty() {
            sqlx::raw_sql(sqlx::AssertSqlSafe(statement.as_str()))
                .execute(&mut *connection)
                .await?;
        } else {
            let columns = rows
                .iter()
                .map(|row| row.try_get::<String, _>("COLUMN_NAME"))
                .collect::<Result<Vec<_>, _>>()?;
            let expected = index
                .fields
                .iter()
                .map(|key| {
                    definition
                        .system_field_by_key(key)
                        .expect("validated index")
                        .column
                        .clone()
                })
                .collect::<Vec<_>>();
            let non_unique: i64 = rows[0].try_get("NON_UNIQUE")?;
            if columns != expected || (non_unique == 0) != index.unique {
                return Err(AppError::Configuration(format!(
                    "changing index {} needs a data migration",
                    index.key
                )));
            }
        }
    }
    Ok::<_, AppError>(())
}

pub async fn ensure_extension_schemas(
    pool: &MySqlPool,
    registry: &crate::application::fields::RecordTypeRegistry,
) -> Result<(), AppError> {
    let mut tables: std::collections::BTreeMap<
        &str,
        Vec<&crate::application::fields::SystemField>,
    > = std::collections::BTreeMap::new();
    for definition in registry.definitions() {
        definition.check()?;
        for field in &definition.system_fields {
            if let Some(extension) = &field.extension {
                tables.entry(&extension.table).or_default().push(field);
            }
        }
    }
    for (table, fields) in tables {
        let mut connection = pool.acquire().await?;
        connection.close_on_drop();
        let lock = format!("aphel:schema:{table}");
        let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 30)")
            .bind(&lock)
            .fetch_one(&mut *connection)
            .await?;
        if acquired != Some(1) {
            return Err(AppError::Persistence(
                "could not lock extension schema".to_owned(),
            ));
        }
        let record_column = &fields[0]
            .extension
            .as_ref()
            .expect("extension")
            .record_column;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE TABLE IF NOT EXISTS `{table}` (`{record_column}` CHAR(36) NOT NULL PRIMARY KEY) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"))).execute(&mut *connection).await?;
        for field in fields {
            let stored: Option<String> = sqlx::query_scalar("SELECT COLUMN_TYPE FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND COLUMN_NAME = ?")
                .bind(table).bind(&field.column).fetch_optional(&mut *connection).await?;
            let expected = field_column_type(field)
                .to_ascii_lowercase()
                .replace("boolean", "tinyint(1)");
            match stored {
                None => {
                    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                        "ALTER TABLE `{table}` ADD COLUMN `{}` {} NULL",
                        field.column,
                        field_column_type(field)
                    )))
                    .execute(&mut *connection)
                    .await?;
                }
                Some(stored) if stored.to_ascii_lowercase() != expected => {
                    return Err(AppError::Configuration(format!(
                        "changing extension column {} needs a migration",
                        field.key
                    )));
                }
                _ => {}
            }
        }
        sqlx::query("SELECT RELEASE_LOCK(?)")
            .bind(&lock)
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}
