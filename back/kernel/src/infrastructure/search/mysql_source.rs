use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::RecordTypeDefinition;
use crate::application::search::{SearchSource, SourceRow};
use crate::errors::AppError;
use crate::infrastructure::query::compiler::{push_partition, quoted};

#[derive(Clone)]
pub struct MySqlSearchSource {
    pool: MySqlPool,
}

impl MySqlSearchSource {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn select(definition: &RecordTypeDefinition, columns: &[(String, String)]) -> QueryBuilder<MySql> {
    let mut sql = String::from("SELECT r.id AS record_id");
    match &definition.owner_column {
        Some(owner) => sql.push_str(&format!(", r.{} AS record_owner", quoted(owner))),
        None => sql.push_str(", NULL AS record_owner"),
    }
    let mut removed: Vec<String> = Vec::new();
    for column in [&definition.deleted_column, &definition.archived_column]
        .into_iter()
        .flatten()
    {
        removed.push(format!("r.{} IS NOT NULL", quoted(column)));
    }
    if removed.is_empty() {
        sql.push_str(", FALSE AS record_removed");
    } else {
        sql.push_str(&format!(", ({}) AS record_removed", removed.join(" OR ")));
    }
    for (position, (_, column)) in columns.iter().enumerate() {
        sql.push_str(&format!(
            ", CAST(r.{} AS CHAR) AS v{position}",
            quoted(column)
        ));
    }
    sql.push_str(&format!(" FROM {} r WHERE TRUE", quoted(&definition.table)));
    let mut builder = QueryBuilder::<MySql>::new(sql);
    push_partition(&mut builder, definition, "r");
    builder
}

fn source_row(row: &MySqlRow, columns: &[(String, String)]) -> Result<SourceRow, AppError> {
    let id: String = row.try_get("record_id")?;
    let removed: i64 = row.try_get("record_removed")?;
    let owner_id: Option<String> = row.try_get("record_owner")?;
    let mut values = HashMap::new();
    for (position, (key, _)) in columns.iter().enumerate() {
        let value: Option<String> = row.try_get(format!("v{position}").as_str())?;
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            values.insert(key.clone(), value);
        }
    }
    Ok(SourceRow {
        id: Uuid::parse_str(&id)
            .map_err(|_| AppError::Persistence(format!("record id {id:?} is not a UUID")))?,
        owner_id,
        removed: removed != 0,
        values,
    })
}

#[async_trait]
impl SearchSource for MySqlSearchSource {
    async fn load(
        &self,
        definition: &RecordTypeDefinition,
        columns: &[(String, String)],
        record_ids: &[Uuid],
    ) -> Result<Vec<SourceRow>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = select(definition, columns);
        builder.push(" AND r.id IN (");
        {
            let mut ids = builder.separated(", ");
            for id in record_ids {
                ids.push_bind(id.to_string());
            }
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(|row| source_row(row, columns)).collect()
    }

    async fn scan(
        &self,
        definition: &RecordTypeDefinition,
        columns: &[(String, String)],
        after: Option<Uuid>,
        limit: u64,
    ) -> Result<Vec<SourceRow>, AppError> {
        let mut builder = select(definition, columns);
        if let Some(after) = after {
            builder.push(" AND r.id > ").push_bind(after.to_string());
        }
        builder.push(" ORDER BY r.id LIMIT ").push_bind(limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(|row| source_row(row, columns)).collect()
    }
}
