use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::search::{
    IndexCandidate, IndexDocument, IndexEntry, IndexHeader, IndexQuery, SearchIndexRepository,
};
use crate::application::shared::RecordType;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::query::compiler::push_record_scope_keyed;
use crate::infrastructure::repositories::mapper;

const HEADER_ALIAS: &str = "s";
const OWNER_COLUMN: &str = "owner_id";

#[derive(Clone)]
pub struct MySqlSearchIndex {
    pool: MySqlPool,
}

impl MySqlSearchIndex {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn phrase(term: &str) -> String {
    format!("\"{}\"", term.replace('"', ""))
}

fn any_terms(terms: &[String]) -> String {
    terms
        .iter()
        .map(|term| phrase(term))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value)
        .map_err(|_| AppError::Persistence(format!("search index id {value:?} is not a UUID")))
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
}

#[async_trait]
impl SearchIndexRepository for MySqlSearchIndex {
    async fn upsert(
        &self,
        document: &IndexDocument,
        indexed_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let record_type = document.record.record_type.as_str();
        let record_id = document.record.record_id.to_string();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO search_index (record_type, record_id, owner_id, title, \
             title_field, subtitle, subtitle_field, indexed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE owner_id = VALUES(owner_id), title = VALUES(title), \
             title_field = VALUES(title_field), subtitle = VALUES(subtitle), \
             subtitle_field = VALUES(subtitle_field), indexed_at = VALUES(indexed_at)",
        )
        .bind(record_type)
        .bind(&record_id)
        .bind(document.owner_id.as_deref())
        .bind(document.display.title.as_deref().unwrap_or_default())
        .bind(document.display.title_field.as_deref())
        .bind(document.display.subtitle.as_deref())
        .bind(document.display.subtitle_field.as_deref())
        .bind(mapper::naive(indexed_at))
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM search_index_terms WHERE record_type = ? AND record_id = ?")
            .bind(record_type)
            .bind(&record_id)
            .execute(&mut *tx)
            .await?;
        if !document.entries.is_empty() {
            let mut builder = QueryBuilder::<MySql>::new(
                "INSERT INTO search_index_terms (record_type, record_id, field_key, \
                 position, weight, is_title_candidate, value, content) ",
            );
            builder.push_values(&document.entries, |mut row, entry| {
                row.push_bind(record_type)
                    .push_bind(&record_id)
                    .push_bind(&entry.field_key)
                    .push_bind(entry.position)
                    .push_bind(entry.weight)
                    .push_bind(entry.title_candidate)
                    .push_bind(&entry.value)
                    .push_bind(&entry.content);
            });
            builder.build().execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn remove(&self, record_type: &RecordType, record_ids: &[Uuid]) -> Result<(), AppError> {
        if record_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new("DELETE FROM search_index WHERE ");
        builder
            .push("record_type = ")
            .push_bind(record_type.as_str())
            .push(" AND record_id IN (");
        push_ids(&mut builder, record_ids);
        builder.push(")");
        builder.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn remove_stale(
        &self,
        record_type: &RecordType,
        indexed_before: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let result =
            sqlx::query("DELETE FROM search_index WHERE record_type = ? AND indexed_at < ?")
                .bind(record_type.as_str())
                .bind(mapper::naive(indexed_before))
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    async fn candidates(
        &self,
        scope: &AccessScope,
        query: &IndexQuery<'_>,
    ) -> Result<Vec<IndexCandidate>, AppError> {
        if query.terms.is_empty() || query.field_keys.is_empty() {
            return Ok(Vec::new());
        }
        let against = any_terms(query.terms);
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT t.record_id, SUM(MATCH(t.content) AGAINST(");
        builder
            .push_bind(against.clone())
            .push(
                " IN BOOLEAN MODE) * t.weight) AS score FROM search_index_terms t \
                 JOIN search_index s ON s.record_type = t.record_type AND s.record_id = t.record_id \
                 WHERE ",
            )
            .push("t.record_type = ")
            .push_bind(query.definition.record_type.as_str())
            .push(" AND t.field_key IN (");
        {
            let mut keys = builder.separated(", ");
            for key in query.field_keys {
                keys.push_bind(key.as_str());
            }
        }
        builder
            .push(") AND MATCH(t.content) AGAINST(")
            .push_bind(against)
            .push(" IN BOOLEAN MODE)");
        let mut header = query.definition.clone();
        header.owner_column = header.owner_column.map(|_| OWNER_COLUMN.to_owned());
        header.partition = None;
        if let Some(access) = header.member_access.as_mut() {
            access.record_key = "record_id".to_owned();
        }
        push_record_scope_keyed(
            &mut builder,
            scope,
            &header,
            query.visibility,
            HEADER_ALIAS,
            "record_id",
        );
        builder.push(" GROUP BY t.record_id");
        if query.terms.len() > 1 {
            builder.push(" HAVING ");
            for (position, term) in query.terms.iter().enumerate() {
                if position > 0 {
                    builder.push(" AND ");
                }
                builder
                    .push("MAX(MATCH(t.content) AGAINST(")
                    .push_bind(phrase(term))
                    .push(" IN BOOLEAN MODE)) > 0");
            }
        }
        builder
            .push(" ORDER BY score DESC, t.record_id LIMIT ")
            .push_bind(query.limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("record_id")?;
                let score: Option<f64> = row.try_get("score")?;
                Ok(IndexCandidate {
                    record_id: parse_uuid(&id)?,
                    score: score.unwrap_or_default(),
                })
            })
            .collect()
    }

    async fn headers(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<Vec<IndexHeader>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT record_id, title, title_field, subtitle, subtitle_field FROM search_index \
             WHERE ",
        );
        builder
            .push("record_type = ")
            .push_bind(record_type.as_str())
            .push(" AND record_id IN (");
        push_ids(&mut builder, record_ids);
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("record_id")?;
                Ok(IndexHeader {
                    record_id: parse_uuid(&id)?,
                    title: row.try_get("title")?,
                    title_field: row.try_get("title_field")?,
                    subtitle: row.try_get("subtitle")?,
                    subtitle_field: row.try_get("subtitle_field")?,
                })
            })
            .collect()
    }

    async fn entries(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
        field_keys: &[String],
    ) -> Result<Vec<(Uuid, IndexEntry)>, AppError> {
        if record_ids.is_empty() || field_keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT record_id, field_key, position, weight, is_title_candidate, value, content \
             FROM search_index_terms WHERE ",
        );
        builder
            .push("record_type = ")
            .push_bind(record_type.as_str())
            .push(" AND record_id IN (");
        push_ids(&mut builder, record_ids);
        builder.push(") AND field_key IN (");
        {
            let mut keys = builder.separated(", ");
            for key in field_keys {
                keys.push_bind(key.as_str());
            }
        }
        builder.push(") ORDER BY record_id, position");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("record_id")?;
                Ok((
                    parse_uuid(&id)?,
                    IndexEntry {
                        field_key: row.try_get("field_key")?,
                        position: row.try_get("position")?,
                        weight: row.try_get("weight")?,
                        title_candidate: row.try_get("is_title_candidate")?,
                        value: row.try_get("value")?,
                        content: row.try_get("content")?,
                    },
                ))
            })
            .collect()
    }
}
