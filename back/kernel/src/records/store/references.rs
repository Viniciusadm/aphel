use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, ReferenceTarget, SoftReference};
use crate::application::shared::RecordRef;
use crate::database::transaction::Tx;
use crate::errors::AppError;
use crate::infrastructure::query::computed::system_expression;

pub(crate) async fn attached_file_mime(
    tx: &mut Tx,
    file_id: Uuid,
    record: &RecordRef,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT f.mime_type FROM attachments a JOIN files f ON f.id = a.file_id \
         WHERE a.file_id = ? AND a.record_type = ? AND a.record_id = ? AND f.deleted_at IS NULL",
    )
    .bind(file_id.to_string())
    .bind(record.record_type.as_str())
    .bind(record.record_id.to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

pub(crate) async fn dependent_ids(
    tx: &mut Tx,
    source: &EntityDefinition,
    reference: &SoftReference,
    target_definition: &EntityDefinition,
    target: &RecordRef,
) -> Result<Vec<String>, AppError> {
    let mut query =
        QueryBuilder::<MySql>::new(format!("SELECT r.id FROM `{}` r WHERE ", source.table));
    value_expression(&mut query, source, &reference.field);
    query.push(" = ").push_bind(target.record_id.to_string());
    if let ReferenceTarget::Trait { type_field, .. } = &reference.target {
        query.push(" AND ");
        value_expression(&mut query, source, type_field);
        query
            .push(" IN (")
            .push_bind(target_definition.key.clone())
            .push(", ")
            .push_bind(target.record_type.as_str().to_owned())
            .push(")");
    }
    if let Some(column) = &source.deleted_column {
        query.push(format!(" AND r.`{column}` IS NULL"));
    }
    if let Some(partition) = &source.partition {
        query
            .push(format!(" AND r.`{}` = ", partition.column))
            .push_bind(partition.value.to_string());
    }
    query.push(" ORDER BY r.id LIMIT 1001 FOR UPDATE");
    Ok(query
        .build_query_scalar::<String>()
        .fetch_all(&mut **tx)
        .await?)
}

fn value_expression(query: &mut QueryBuilder<MySql>, source: &EntityDefinition, field: &str) {
    if let Some(field) = source.system_field_by_key(field) {
        query.push(system_expression(field, "r"));
    } else {
        query.push("(SELECT v.value_text FROM custom_field_values v JOIN field_definitions f ON f.id = v.field_id WHERE v.record_id = r.id AND v.record_type = ")
            .push_bind(source.record_type.as_str().to_owned()).push(" AND f.`key` = ").push_bind(field.to_owned()).push(")");
    }
}

pub async fn existing_tables(
    tx: &mut Tx,
    tables: &[&str],
) -> Result<std::collections::HashSet<String>, AppError> {
    if tables.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 storage tables may be requested".into(),
        ));
    }
    if tables.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name IN (",
    );
    let mut names = query.separated(", ");
    for table in tables {
        names.push_bind((*table).to_owned());
    }
    query.push(")");
    Ok(query
        .build_query_scalar()
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .collect())
}

pub async fn move_core_references(
    tx: &mut Tx,
    record_type: &str,
    from: Uuid,
    to: Uuid,
) -> Result<std::collections::BTreeMap<String, u64>, AppError> {
    crate::application::shared::RecordType::parse(record_type.to_owned())?;
    if from == to {
        return Err(AppError::Validation(
            "reference merge targets must differ".into(),
        ));
    }
    let from = from.to_string();
    let to = to.to_string();
    let moved = sqlx::query(
        "UPDATE IGNORE attachments SET record_id = ? WHERE record_type = ? AND record_id = ?",
    )
    .bind(&to)
    .bind(record_type)
    .bind(&from)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    sqlx::query("DELETE FROM attachments WHERE record_type = ? AND record_id = ?")
        .bind(record_type)
        .bind(&from)
        .execute(&mut **tx)
        .await?;
    let references = sqlx::query("UPDATE custom_field_values v JOIN field_definitions f ON f.id = v.field_id SET v.value_text = ? WHERE f.field_type = 'record' AND JSON_UNQUOTE(JSON_EXTRACT(f.config, '$.target_record_type')) = ? AND v.value_text = ?")
        .bind(&to).bind(record_type).bind(&from).execute(&mut **tx).await?.rows_affected();
    let mut relationships = 0;
    for (column, type_column) in [("from_id", "from_type"), ("to_id", "to_type")] {
        relationships += sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE IGNORE record_relations l JOIN relationship_definitions d ON d.id = l.relationship_id SET l.{column} = ? WHERE d.{type_column} = ? AND l.{column} = ?")))
            .bind(&to).bind(record_type).bind(&from).execute(&mut **tx).await?.rows_affected();
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE l FROM record_relations l JOIN relationship_definitions d ON d.id = l.relationship_id WHERE d.{type_column} = ? AND l.{column} = ?")))
            .bind(record_type).bind(&from).execute(&mut **tx).await?;
    }
    Ok(std::collections::BTreeMap::from([
        ("attachments".to_owned(), moved),
        ("references".to_owned(), references),
        ("relationships".to_owned(), relationships),
    ]))
}

fn custom_merge_storage(definition: &EntityDefinition) -> Result<(), AppError> {
    if !definition.is_custom() {
        return Err(AppError::Configuration(
            "native record merges require an owner capability".into(),
        ));
    }
    Ok(())
}

pub async fn copy_custom_columns(
    tx: &mut Tx,
    definition: &EntityDefinition,
    from: Uuid,
    to: Uuid,
    columns: &[String],
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    custom_merge_storage(definition)?;
    if from == to {
        return Err(AppError::Validation("merge targets must differ".into()));
    }
    if columns.is_empty() {
        return Ok(());
    }
    let reserved = [
        definition.deleted_column.as_deref(),
        definition.archived_column.as_deref(),
        definition
            .partition
            .as_ref()
            .map(|partition| partition.column.as_str()),
    ];
    for column in columns {
        if !definition.system_fields.iter().any(|field| {
            field.column == *column
                && !field.is_computed()
                && field.extension.is_none()
                && field.contributed_by.is_none()
                && !["id", "created_by", "created_at", "updated_at"].contains(&field.key.as_str())
                && !reserved.contains(&Some(field.column.as_str()))
                && !definition
                    .capabilities
                    .locked_fields
                    .iter()
                    .any(|locked| locked == field.key.as_str())
        }) {
            return Err(AppError::Validation("invalid custom merge column".into()));
        }
    }
    let assignments = columns
        .iter()
        .map(|column| {
            format!(
                "{} = {}",
                crate::infrastructure::query::compiler::aliased("p", column),
                crate::infrastructure::query::compiler::aliased("d", column)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut query = QueryBuilder::<MySql>::new(format!(
        "UPDATE `{table}` p JOIN `{table}` d ON d.id = ",
        table = definition.table
    ));
    query
        .push_bind(from.to_string())
        .push(format!(" SET {assignments}"));
    if let Some(updated) = definition.system_field_by_key("updated_at") {
        query
            .push(format!(
                ", {} = ",
                crate::infrastructure::query::compiler::aliased("p", &updated.column)
            ))
            .push_bind(now.naive_utc());
    }
    query.push(" WHERE p.id = ").push_bind(to.to_string());
    if let Some(deleted) = &definition.deleted_column {
        query.push(format!(
            " AND {} IS NULL AND {} IS NULL",
            crate::infrastructure::query::compiler::aliased("p", deleted),
            crate::infrastructure::query::compiler::aliased("d", deleted)
        ));
    }
    crate::infrastructure::query::compiler::push_partition(&mut query, definition, "p");
    crate::infrastructure::query::compiler::push_partition(&mut query, definition, "d");
    query.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn retire_custom(
    tx: &mut Tx,
    definition: &EntityDefinition,
    id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    custom_merge_storage(definition)?;
    let deleted = definition.deleted_column.as_ref().ok_or_else(|| {
        AppError::Configuration("custom merge retirement requires soft deletion".into())
    })?;
    let deleted = crate::infrastructure::query::compiler::aliased("r", deleted);
    let mut query =
        QueryBuilder::<MySql>::new(format!("UPDATE `{}` r SET {deleted} = ", definition.table));
    query
        .push_bind(now.naive_utc())
        .push(" WHERE r.id = ")
        .push_bind(id.to_string())
        .push(format!(" AND {deleted} IS NULL"));
    crate::infrastructure::query::compiler::push_partition(&mut query, definition, "r");
    query.build().execute(&mut **tx).await?;
    Ok(())
}
