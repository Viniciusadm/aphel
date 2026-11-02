use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::EntityDefinition;
use crate::application::record_sharing::{RecordDirectory, RecordHeader};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::errors::AppError;
use crate::infrastructure::query::compiler::{push_record_scope, quoted};
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlRecordDirectory {
    pool: MySqlPool,
}

impl MySqlRecordDirectory {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn select(definition: &EntityDefinition) -> QueryBuilder<MySql> {
    let owner = definition.owner_column.as_deref().map_or_else(
        || "NULL".to_owned(),
        |column| format!("r.{}", quoted(column)),
    );
    let title = definition.title_column().map_or_else(
        || "NULL".to_owned(),
        |column| format!("CAST(r.{} AS CHAR)", quoted(column)),
    );
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id AS record_id, {owner} AS record_owner, {title} AS record_title FROM {} r \
         WHERE TRUE",
        quoted(&definition.table)
    ));
    if let Some(column) = &definition.deleted_column {
        builder.push(format!(" AND r.{} IS NULL", quoted(column)));
    }
    builder
}

fn clean_title(value: Option<String>) -> Option<String> {
    value
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
}

#[async_trait]
impl RecordDirectory for MySqlRecordDirectory {
    async fn header(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        visibility: &RecordScopeFilter,
        record_id: Uuid,
    ) -> Result<Option<RecordHeader>, AppError> {
        let mut builder = select(definition);
        builder
            .push(" AND r.id = ")
            .push_bind(record_id.to_string());
        push_record_scope(&mut builder, scope, definition, visibility);
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.map(|row| {
            let owner: Option<String> = row.try_get("record_owner")?;
            let title: Option<String> = row.try_get("record_title")?;
            Ok(RecordHeader {
                owner_id: owner
                    .filter(|owner| !owner.is_empty())
                    .map(|owner| mapper::typed_id(owner, UserId::new))
                    .transpose()?,
                title: clean_title(title),
            })
        })
        .transpose()
    }

    async fn visible_titles(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        visibility: &RecordScopeFilter,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Option<String>>, AppError> {
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = select(definition);
        builder.push(" AND r.id IN (");
        {
            let mut ids = builder.separated(", ");
            for id in record_ids {
                ids.push_bind(id.to_string());
            }
        }
        builder.push(")");
        push_record_scope(&mut builder, scope, definition, visibility);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut titles = HashMap::new();
        for row in rows {
            let id: String = row.try_get("record_id")?;
            let title: Option<String> = row.try_get("record_title")?;
            titles.insert(
                Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
                clean_title(title),
            );
        }
        Ok(titles)
    }
}
