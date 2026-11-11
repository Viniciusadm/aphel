use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use aphel_contract_tagging::{CLASSIFIABLE, ClassifiableDefinition};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{ChangedRecord, SavingRecord, query_ports};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine, RecordQueryPorts,
};
use aphel_sdk::platform::domain::access::AccessScope;
use serde_json::Value;
use uuid::Uuid;

use crate::service::TAG_NOT_ALLOWED;
use crate::store::TagStore;

pub(crate) fn declared() -> Vec<ClassifiableDefinition> {
    let Some(app) = aphel_sdk::platform::modules::try_app() else {
        return Vec::new();
    };
    let native = app
        .contributions::<ClassifiableDefinition>()
        .into_iter()
        .map(|definition: Arc<ClassifiableDefinition>| (*definition).clone());
    let runtime = app
        .runtime_contributions(CLASSIFIABLE)
        .into_iter()
        .filter_map(|contribution| {
            serde_json::from_value::<ClassifiableDefinition>(contribution.data.clone())
                .inspect_err(|error| {
                    tracing::warn!(name = contribution.name, %error, "ignoring an invalid classifiable contribution");
                })
                .ok()
        });
    native.chain(runtime).collect()
}

fn ids_of(value: Option<&Value>) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = Vec::new();
    for id in value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|text| Uuid::parse_str(text.trim()).ok())
    {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(crate) async fn saving(
    ctx: &mut HookCtx<'_>,
    mut saving: SavingRecord,
) -> Result<SavingRecord, AppError> {
    let definitions: Vec<ClassifiableDefinition> = declared()
        .into_iter()
        .filter(|definition| definition.entity == saving.record_type.as_str())
        .collect();
    for definition in definitions {
        let Some(value) = saving.draft.values.get(&definition.field) else {
            continue;
        };
        if !value.is_array() {
            continue;
        }
        let ids = ids_of(Some(value));
        let existing: Vec<Uuid> = TagStore::new(ctx.pool.clone())
            .find_many(&ids)
            .await?
            .into_iter()
            .map(|tag| tag.id)
            .collect();
        let kept: Vec<Value> = ids
            .into_iter()
            .filter(|id| existing.contains(id))
            .map(|id| Value::String(id.to_string()))
            .collect();
        saving
            .draft
            .values
            .insert(definition.field.clone(), Value::Array(kept));
    }
    Ok(saving)
}

pub(crate) async fn validating(
    ctx: &mut HookCtx<'_>,
    changed: &ChangedRecord,
) -> Result<(), AppError> {
    let definitions: Vec<ClassifiableDefinition> = declared()
        .into_iter()
        .filter(|definition| definition.entity == changed.record_type.as_str())
        .collect();
    if definitions.is_empty() {
        return Ok(());
    }
    let store = TagStore::new(ctx.pool.clone());
    let vocabularies: HashMap<Uuid, String> = store
        .vocabularies()
        .await?
        .into_iter()
        .map(|vocabulary| (vocabulary.id, vocabulary.key))
        .collect();
    for definition in definitions {
        if !changed
            .changes
            .iter()
            .any(|change| change.field == definition.field)
        {
            continue;
        }
        let ids = ids_of(changed.values.get(&definition.field));
        for tag in store.find_many(&ids).await? {
            let allowed = vocabularies
                .get(&tag.vocabulary_id)
                .is_some_and(|key| definition.vocabularies.contains(key));
            if !allowed {
                return Err(AppError::invalid_fields_code(
                    TAG_NOT_ALLOWED,
                    format!("{} cannot be selected here", tag.name),
                    vec![FieldViolation::new(
                        definition.field.clone(),
                        format!(
                            "{} belongs to a vocabulary this record does not use",
                            tag.name
                        ),
                    )],
                ));
            }
        }
    }
    Ok(())
}

pub(crate) async fn uses(tag_id: Uuid) -> Result<BTreeMap<String, u64>, AppError> {
    let state = aphel_sdk::runtime()?;
    let ports = query_ports(state.as_ref()).await?;
    let engine = RecordQueryEngine::new(&ports);
    let scope = AccessScope::system();
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for definition in declared() {
        let Ok(entity) = ports.registry().require(&definition.entity) else {
            continue;
        };
        let fields: Vec<&String> = std::iter::once(&definition.field)
            .chain(definition.published.as_ref())
            .filter(|field| entity.system_field_by_key(field).is_some())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let filter = Filter::or(
            fields
                .into_iter()
                .map(|field| {
                    Filter::condition(
                        field,
                        FilterOperator::In,
                        Value::Array(vec![Value::String(tag_id.to_string())]),
                    )
                })
                .collect(),
        );
        let listing = engine
            .query(
                &scope,
                &definition.entity,
                RecordQuery {
                    filter: Some(filter),
                    archived: ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: 1,
                    },
                    ..Default::default()
                },
            )
            .await?;
        if listing.total > 0 {
            *counts.entry(definition.entity.clone()).or_default() += listing.total;
        }
    }
    Ok(counts)
}
