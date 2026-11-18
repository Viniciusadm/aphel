use std::sync::Arc;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::RecordTypeRegistry;
use crate::application::fields::{DeletePolicy, EntityDefinition, ReferenceTarget, SoftReference};
use crate::application::shared::RecordRef;
use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::access::AccessScope;
use crate::errors::{AppError, FieldViolation};
use crate::records::RecordCtx;

pub async fn validate(
    tx: &mut Tx,
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
        if !super::store::entity::live_row_in_scope(
            tx,
            scope,
            target,
            &visibility,
            id,
            super::store::entity::RowLock::Share,
        )
        .await?
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
    tx: &mut Tx,
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
        let mime = super::store::references::attached_file_mime(tx, id, record).await?;
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
    tx: &mut Tx,
    pool: &Db,
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
    let Some(target_definition) = registry.get(&target.record_type) else {
        return Ok(());
    };
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
            let ids = super::store::references::dependent_ids(
                tx,
                source,
                reference,
                target_definition,
                target,
            )
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
