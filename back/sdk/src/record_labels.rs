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
use aphel_kernel::infrastructure::query::{
    MySqlRecordQueryPorts,
    compiler::{push_partition, quoted},
    computed::system_expression,
};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
pub struct VisibleRecordLabel {
    pub title: Option<String>,
}

pub async fn visible_record_labels<D: FieldDefinitionRepository>(
    pool: &MySqlPool,
    registry: &Arc<RecordTypeRegistry>,
    definitions: &D,
    scope: &AccessScope,
    record_type: &RecordType,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, VisibleRecordLabel>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    if ids.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 record labels may be requested".into(),
        ));
    }
    let app = aphel_kernel::modules::app();
    if app
        .record_contributions()
        .iter()
        .find(|entry| entry.registers.as_ref() == Some(record_type))
        .is_some_and(|entry| !app.is_enabled(entry.module))
    {
        return Ok(HashMap::new());
    }
    let Some(definition) = registry.get(record_type) else {
        return Ok(HashMap::new());
    };
    if scope.scope_for(&definition.read_permission).is_none() {
        return Ok(HashMap::new());
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
                    limit: u64::try_from(ids.len()).expect("bounded labels"),
                },
                ..RecordQuery::default()
            },
        )
        .await?
        .items;
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
    let mut query = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id, {} AS record_label FROM {} r WHERE r.id IN (",
        system_expression(field, "r"),
        quoted(&definition.table)
    ));
    let mut separated = query.separated(", ");
    for id in &selected {
        separated.push_bind(id.to_string());
    }
    query.push(")");
    push_partition(&mut query, definition, "r");
    for row in query.build().fetch_all(pool).await? {
        let raw: String = row.try_get("id")?;
        let id = Uuid::parse_str(&raw)
            .map_err(|_| AppError::Internal("invalid stored record identity".into()))?;
        if let Some(label) = labels.get_mut(&id) {
            label.title =
                aphel_kernel::records::decode_record_field(&row, "record_label", field.field_type)?
                    .as_str()
                    .map(str::to_owned);
        }
    }
    Ok(labels)
}

pub async fn visible_record_label<D: FieldDefinitionRepository>(
    pool: &MySqlPool,
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
