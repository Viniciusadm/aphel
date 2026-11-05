use aphel_kernel::domain::{UserId, access::AccessScope};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::infrastructure::query::compiler::{Projection, build_query};
use aphel_sdk::platform::infrastructure::repositories::mapper::{invalid_column, utc};
use aphel_sdk::platform::records::constrain_query;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use std::collections::HashMap;
use uuid::Uuid;

pub fn uuid_column(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

pub fn user_column(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    id.map(|id| {
        Ok(UserReference {
            id: UserId::new(id.clone()).map_err(|_| invalid_column(id_column, &id))?,
            display_name: name.unwrap_or_default(),
        })
    })
    .transpose()
}

pub fn link_column(
    row: &MySqlRow,
    type_column: &str,
    id_column: &str,
) -> Result<Option<(String, Uuid)>, AppError> {
    let record_type: Option<String> = row.try_get(type_column)?;
    let record_id: Option<String> = row.try_get(id_column)?;
    match (record_type, record_id) {
        (Some(record_type), Some(record_id)) => {
            Ok(Some((record_type, uuid_column(&record_id, id_column)?)))
        }
        _ => Ok(None),
    }
}

pub fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

pub fn in_order<T>(ids: &[Uuid], mut items: HashMap<Uuid, T>) -> Vec<T> {
    ids.iter().filter_map(|id| items.remove(id)).collect()
}

pub async fn record_ids(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<Uuid>, AppError> {
    let constrained = constrain_query(pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let mut builder = build_query(scope, plan, Projection::Ids)?;
    let rows: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    rows.iter().map(|id| uuid_column(id, "id")).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordLink {
    pub(crate) record_type: String,
    pub(crate) record_id: Uuid,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingActivity {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) priority: String,
    pub(crate) due_at: Option<DateTime<Utc>>,
    pub(crate) owner: Option<UserReference>,
    pub(crate) record: Option<RecordLink>,
}

pub(crate) async fn pending_activities<
    P: aphel_sdk::platform::application::query::RecordQueryPorts,
>(
    pool: &MySqlPool,
    scope: &AccessScope,
    query: &P,
    plan: &QueryPlan,
) -> Result<Vec<PendingActivity>, AppError> {
    let ids = record_ids(pool, scope, plan).await?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT a.id, a.title, a.priority, a.due_at, a.owner_id, u.display_name AS owner_name, \
         a.record_type, a.record_id FROM tasks a LEFT JOIN users u ON u.id = a.owner_id \
         WHERE a.id",
    );
    push_ids(&mut builder, &ids);
    let rows = builder.build().fetch_all(pool).await?;
    let mut items = HashMap::new();
    for row in &rows {
        let id: String = row.try_get("id")?;
        let id = uuid_column(&id, "tasks.id")?;
        let link = link_column(row, "record_type", "record_id")?;
        let due_at: Option<NaiveDateTime> = row.try_get("due_at")?;
        items.insert(
            id,
            (
                PendingActivity {
                    id,
                    title: row.try_get("title")?,
                    priority: row.try_get("priority")?,
                    due_at: due_at.map(utc),
                    owner: user_column(row, "owner_id", "owner_name")?,
                    record: None,
                },
                link,
            ),
        );
    }
    let registry = std::sync::Arc::new(query.registry().clone());
    let mut result = Vec::new();
    for (mut activity, link) in in_order(&ids, items) {
        if let Some((record_type, record_id)) = link {
            let reference = aphel_sdk::platform::application::shared::RecordRef::parse(
                &record_type,
                &record_id.to_string(),
            )?;
            if let Some(label) = aphel_sdk::records::visible_record_label(
                pool,
                &registry,
                query.definitions(),
                scope,
                &reference,
            )
            .await?
            {
                activity.record = Some(RecordLink {
                    record_type,
                    record_id,
                    title: label.title,
                });
            }
        }
        result.push(activity);
    }
    Ok(result)
}
