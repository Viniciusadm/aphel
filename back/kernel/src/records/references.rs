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
    Ok(())
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
