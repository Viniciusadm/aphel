use crate::{
    AppError,
    access::AccessScope,
    fields::RecordTypeRegistry,
    shared::{RecordRef, RecordType},
};
use aphel_kernel::application::fields::{
    FieldDefinitionRepository,
    permissions::{FieldAccess, FieldAccessPolicy},
};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_kernel::database::Db;
use aphel_kernel::infrastructure::query::MySqlRecordQueryPorts;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
pub struct VisibleRecordLabel {
    pub title: Option<String>,
}

pub async fn visible_record_labels<D: FieldDefinitionRepository>(
    pool: &Db,
    registry: &Arc<RecordTypeRegistry>,
    definitions: &D,
    scope: &AccessScope,
    record_type: &RecordType,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, VisibleRecordLabel>, AppError> {
    if ids.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 record labels may be requested".into(),
        ));
    }
    let selected = visible_record_ids(pool, registry, scope, record_type, ids).await?;
    let Some(definition) = registry.get(record_type) else {
        return Ok(HashMap::new());
    };
    let mut labels: HashMap<_, _> = selected
        .iter()
        .map(|id| (*id, VisibleRecordLabel { title: None }))
        .collect();
    if selected.is_empty() {
        return Ok(labels);
    }
    let policy = FieldAccessPolicy::load(definitions, scope, record_type).await?;
    let Some(field) = definition
        .title_field
        .as_deref()
        .filter(|key| policy.key_access(key) != FieldAccess::Hidden)
        .and_then(|key| definition.system_field_by_key(key))
    else {
        return Ok(labels);
    };
    for (id, title) in
        aphel_kernel::records::record_titles(pool, definition, field, &selected).await?
    {
        if let Some(label) = labels.get_mut(&id) {
            label.title = title;
        }
    }
    Ok(labels)
}

pub async fn visible_record_label<D: FieldDefinitionRepository>(
    pool: &Db,
    registry: &Arc<RecordTypeRegistry>,
    definitions: &D,
    scope: &AccessScope,
    record: &RecordRef,
) -> Result<Option<VisibleRecordLabel>, AppError> {
    Ok(visible_record_labels(
        pool,
        registry,
        definitions,
        scope,
        &record.record_type,
        &[record.record_id],
    )
    .await?
    .remove(&record.record_id))
}

pub async fn visible_record_ids(
    pool: &Db,
    registry: &Arc<RecordTypeRegistry>,
    scope: &AccessScope,
    record_type: &RecordType,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 record identities may be requested".into(),
        ));
    }
    let app = aphel_kernel::modules::app();
    if app
        .record_contributions()
        .iter()
        .find(|entry| entry.registers.as_ref() == Some(record_type))
        .is_some_and(|entry| !app.is_enabled(entry.module))
    {
        return Ok(Vec::new());
    }
    let Some(definition) = registry.get(record_type) else {
        return Ok(Vec::new());
    };
    if scope.scope_for(&definition.read_permission).is_none() {
        return Ok(Vec::new());
    }
    let ports = MySqlRecordQueryPorts::new(pool.clone(), registry.clone());
    let selected = RecordQueryEngine::new(&ports)
        .query(
            scope,
            record_type.as_str(),
            RecordQuery {
                filter: Some(Filter::condition(
                    "id",
                    FilterOperator::In,
                    serde_json::json!(ids),
                )),
                archived: ArchivedFilter::Include,
                window: ListWindow {
                    offset: 0,
                    limit: u64::try_from(ids.len()).expect("bounded record identities"),
                },
                ..RecordQuery::default()
            },
        )
        .await?
        .items;
    Ok(selected)
}
