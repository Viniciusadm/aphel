use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::RecordTypeRegistry;
use crate::application::fields::{DeletePolicy, EntityDefinition, ReferenceTarget, SoftReference};
use crate::application::shared::RecordRef;
use crate::database::transaction::MySqlTx;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::query::compiler::push_record_scope;
use crate::infrastructure::query::computed::system_expression;
use crate::records::RecordCtx;

pub async fn validate(
    tx: &mut MySqlTx,
    scope: &AccessScope,
    registry: &RecordTypeRegistry,
    record: &RecordRef,
    values: &Value,
) -> Result<(), AppError> {
    let source = registry.require(record.record_type.as_str())?;
    for reference in &source.references {
        let Some(value) = values
            .get(&reference.field)
            .filter(|value| !value.is_null())
        else {
            continue;
        };
        let id = value
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| invalid(&reference.field))?;
        let target = match &reference.target {
            ReferenceTarget::Entity { entity } => registry.require(entity)?,
            ReferenceTarget::Trait {
                trait_key,
                type_field,
            } => {
                let key = values
                    .get(type_field)
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid(type_field))?;
                let definition = registry.require(key)?;
                if !definition.capabilities.has_trait(trait_key) {
                    return Err(invalid(type_field));
                }
                definition
            }
        };
        let visibility = scope.record_filter(&target.read_permission)?;
        let mut query = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id FROM `{}` r WHERE r.id = ",
            target.table
        ));
        query.push_bind(id.to_string());
        if let Some(column) = &target.deleted_column {
            query.push(format!(" AND r.`{column}` IS NULL"));
        }
        push_record_scope(&mut query, scope, target, &visibility);
        query.push(" FOR SHARE");
        if query
            .build_query_scalar::<String>()
            .fetch_optional(&mut **tx)
            .await?
            .is_none()
        {
            return Err(invalid(&reference.field));
        }
    }
    validate_translations(source, values)?;
    validate_files(tx, source, record, values).await
}

fn translation_language(key: &str) -> bool {
    let mut parts = key.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next();
    (2..=3).contains(&language.len())
        && language.chars().all(|char| char.is_ascii_lowercase())
        && region.is_none_or(|region| {
            region.len() == 2 && region.chars().all(|char| char.is_ascii_uppercase())
        })
        && parts.next().is_none()
}

fn validate_translations(source: &EntityDefinition, values: &Value) -> Result<(), AppError> {
    for field in source
        .system_fields
        .iter()
        .filter(|field| field.translation_of.is_some() && !field.owner_managed)
    {
        let Some(text) = values
            .get(field.key.as_str())
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
        else {
            continue;
        };
        let base = field
            .translation_of
            .as_deref()
            .and_then(|key| source.system_field_by_key(key));
        let limit = base
            .map(|base| base.config.max_text_length(base.field_type) as usize)
            .unwrap_or(usize::MAX);
        let document = base.is_some_and(|base| {
            base.field_type.base_type() == crate::domain::fields::FieldType::Document
        });
        let problem = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(map)) => map.iter().find_map(|(language, value)| {
                if !translation_language(language) {
                    return Some(format!(
                        "{language} is not a language code such as en or pt-BR"
                    ));
                }
                let Some(value) = value.as_str() else {
                    return (!value.is_null()).then(|| format!("{language} must be text"));
                };
                if value.chars().count() > limit {
                    return Some(format!("{language} must have at most {limit} characters"));
                }
                (document
                    && !value.trim().is_empty()
                    && serde_json::from_str::<Value>(value).is_err())
                .then(|| format!("{language} must be valid JSON"))
            }),
            _ => Some("must be an object of language codes and texts".to_owned()),
        };
        if let Some(problem) = problem {
            return Err(AppError::invalid_fields(
                "invalid translations",
                vec![FieldViolation::new(field.key.as_str(), problem)],
            ));
        }
    }
    Ok(())
}

async fn validate_files(
    tx: &mut MySqlTx,
    source: &EntityDefinition,
    record: &RecordRef,
    values: &Value,
) -> Result<(), AppError> {
    for field in source
        .system_fields
        .iter()
        .filter(|field| field.field_type.is_file() && !field.owner_managed)
    {
        let Some(value) = values
            .get(field.key.as_str())
            .filter(|value| !value.is_null())
        else {
            continue;
        };
        let id = value
            .as_str()
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
            .ok_or_else(|| invalid_file(field.key.as_str(), "must be a valid file id"))?;
        let mime: Option<String> = sqlx::query_scalar(
            "SELECT f.mime_type FROM attachments a JOIN files f ON f.id = a.file_id \
             WHERE a.file_id = ? AND a.record_type = ? AND a.record_id = ? AND f.deleted_at IS NULL",
        )
        .bind(id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(mime) = mime else {
            return Err(invalid_file(
                field.key.as_str(),
                "must be a file attached to this record",
            ));
        };
        if field.field_type.base_type() == crate::domain::fields::FieldType::Image
            && !mime.starts_with("image/")
        {
            return Err(invalid_file(field.key.as_str(), "must be an image"));
        }
    }
    Ok(())
}

fn invalid_file(field: &str, message: &str) -> AppError {
    AppError::invalid_fields("invalid file", vec![FieldViolation::new(field, message)])
}

fn invalid(field: &str) -> AppError {
    AppError::invalid_fields(
        "invalid soft reference",
        vec![FieldViolation::new(
            field,
            "referenced entity or record is unavailable",
        )],
    )
}

fn value_expression(query: &mut QueryBuilder<MySql>, source: &EntityDefinition, field: &str) {
    if let Some(field) = source.system_field_by_key(field) {
        query.push(system_expression(field, "r"));
    } else {
        query.push("(SELECT v.value_text FROM custom_field_values v JOIN field_definitions f ON f.id = v.field_id WHERE v.record_id = r.id AND v.record_type = ")
            .push_bind(source.record_type.as_str().to_owned()).push(" AND f.`key` = ").push_bind(field.to_owned()).push(")");
    }
}

fn matches_target(
    reference: &SoftReference,
    target: &EntityDefinition,
    registry: &RecordTypeRegistry,
) -> bool {
    match &reference.target {
        ReferenceTarget::Entity { entity } => registry
            .require(entity)
            .is_ok_and(|definition| definition.record_type == target.record_type),
        ReferenceTarget::Trait { trait_key, .. } => target.capabilities.has_trait(trait_key),
    }
}

pub async fn deleting(
    tx: &mut MySqlTx,
    pool: &MySqlPool,
    registry: &Arc<RecordTypeRegistry>,
    target: &RecordRef,
) -> Result<(), AppError> {
    if !tx
        .reference_deletions
        .insert((target.record_type.as_str().to_owned(), target.record_id))
    {
        return Ok(());
    }
    if tx.reference_deletions.len() > 1000 {
        return Err(AppError::Validation(
            "reference cascade exceeds 1000 records".to_owned(),
        ));
    }
    let target_definition = registry.require(target.record_type.as_str())?;
    let scope = AccessScope::system().acting_as("reference_policy", target_definition.key.clone());
    let ctx = crate::Ctx::from_parts(
        &scope,
        pool.clone(),
        registry.clone(),
        Arc::new(crate::infrastructure::events::MySqlEventPublisher::new()),
        Arc::new(crate::infrastructure::audit::MySqlAuditLog::new()),
    );
    let state = ctx.state();
    let record_ctx = RecordCtx::new(&scope, &state);
    for source in registry.definitions() {
        for reference in source
            .references
            .iter()
            .filter(|reference| matches_target(reference, target_definition, registry))
        {
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
            let ids = query
                .build_query_scalar::<String>()
                .fetch_all(&mut **tx)
                .await?;
            if ids.len() > 1000 {
                return Err(AppError::Validation(
                    "reference policy exceeds 1000 dependent records".to_owned(),
                ));
            }
            for id in ids {
                let id = Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal("invalid dependent record id".to_owned()))?;
                if tx
                    .reference_deletions
                    .contains(&(source.record_type.as_str().to_owned(), id))
                {
                    continue;
                }
                match reference.on_delete {
                    DeletePolicy::Restrict => {
                        return Err(AppError::conflict_code(
                            "record_referenced",
                            format!(
                                "{} is referenced by {}.{}",
                                target_definition.key, source.key, reference.field
                            ),
                        ));
                    }
                    DeletePolicy::Clear => {
                        let custom = source.system_field_by_key(&reference.field).is_none();
                        let mut values =
                            serde_json::Map::from_iter([(reference.field.clone(), Value::Null)]);
                        if let ReferenceTarget::Trait { type_field, .. } = &reference.target {
                            values.insert(type_field.clone(), Value::Null);
                        }
                        let body = if custom {
                            json!({"custom_fields": values})
                        } else {
                            Value::Object(values)
                        };
                        if let Some(pending) = registry
                            .handler(source.record_type.as_str())?
                            .update_in_tx(&record_ctx, tx, id, body)
                            .await?
                        {
                            let state = state.clone();
                            let scope = scope.clone();
                            tx.after_commit.push(Box::pin(async move {
                                if let Ok(ctx) = crate::Ctx::from_state(&state, &scope).await {
                                    pending.committed(&ctx).await;
                                }
                            }));
                        }
                    }
                    DeletePolicy::Cascade => {
                        registry
                            .handler(source.record_type.as_str())?
                            .delete_in_tx(&record_ctx, tx, id)
                            .await?
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn existing_tables(
    tx: &mut MySqlTx,
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
    tx: &mut MySqlTx,
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
    tx: &mut MySqlTx,
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
    tx: &mut MySqlTx,
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
