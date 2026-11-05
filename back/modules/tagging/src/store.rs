use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::Tag;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const TAG_COLUMNS: &str =
    "t.id, t.name, t.color, t.record_types, t.created_by, t.created_at, t.updated_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tagging {
    pub(crate) tag_id: Uuid,
    pub(crate) record: RecordRef,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TagStore {
    pool: MySqlPool,
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn tag_from_row(row: &MySqlRow) -> Result<Tag, AppError> {
    let Json(record_types): Json<Value> = row.try_get("record_types")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(Tag {
        id: uuid_column(row, "id")?,
        name: row.try_get("name")?,
        color: row.try_get("color")?,
        record_types: serde_json::from_value(record_types.clone())
            .map_err(|_| mapper::invalid_column("record_types", &record_types.to_string()))?,
        created_by: row
            .try_get::<Option<String>, _>("created_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

impl TagStore {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub async fn list(&self) -> Result<Vec<Tag>, AppError> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT {TAG_COLUMNS} FROM tags t ORDER BY t.name, t.id"
        ));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(tag_from_row).collect()
    }

    pub(crate) async fn find(&self, tag_id: Uuid) -> Result<Option<Tag>, AppError> {
        let mut builder = QueryBuilder::new(format!("SELECT {TAG_COLUMNS} FROM tags t WHERE "));
        builder.push("t.id = ").push_bind(tag_id.to_string());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(tag_from_row)
            .transpose()
    }

    pub(crate) async fn find_by_name(&self, name: &str) -> Result<Option<Tag>, AppError> {
        let mut builder = QueryBuilder::new(format!("SELECT {TAG_COLUMNS} FROM tags t WHERE "));
        builder.push("t.name = ").push_bind(name.to_owned());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(tag_from_row)
            .transpose()
    }

    pub(crate) async fn usage(&self) -> Result<HashMap<Uuid, u64>, AppError> {
        let rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT tag_id, COUNT(*) FROM taggings GROUP BY tag_id")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|(id, count)| {
                Uuid::parse_str(&id)
                    .map(|id| (id, u64::try_from(count).unwrap_or_default()))
                    .map_err(|_| mapper::invalid_column("tag_id", &id))
            })
            .collect()
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, tag: &Tag) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO tags (id, name, color, record_types, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tag.id.to_string())
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(Json(&tag.record_types))
        .bind(tag.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(tag.created_at))
        .bind(mapper::naive(tag.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut MySqlTx, tag: &Tag) -> Result<u64, AppError> {
        sqlx::query(
            "UPDATE tags SET name = ?, color = ?, record_types = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(Json(&tag.record_types))
        .bind(mapper::naive(tag.updated_at))
        .bind(tag.id.to_string())
        .execute(&mut **tx)
        .await?;
        if tag.record_types.is_empty() {
            return Ok(0);
        }
        let mut builder = QueryBuilder::new("DELETE FROM taggings WHERE tag_id = ");
        builder
            .push_bind(tag.id.to_string())
            .push(" AND record_type NOT IN (");
        let mut separated = builder.separated(", ");
        for record_type in &tag.record_types {
            separated.push_bind(record_type.clone());
        }
        builder.push(")");
        Ok(builder.build().execute(&mut **tx).await?.rows_affected())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, tag_id: Uuid) -> Result<u64, AppError> {
        let removed = sqlx::query("DELETE FROM taggings WHERE tag_id = ?")
            .bind(tag_id.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected();
        sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(tag_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(removed)
    }

    pub(crate) async fn tags_of(
        &self,
        record_type: &str,
        record_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, Tag)>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new(format!(
            "SELECT tg.record_id, {TAG_COLUMNS} FROM taggings tg \
             JOIN tags t ON t.id = tg.tag_id \
             WHERE "
        ));
        builder
            .push("tg.record_type = ")
            .push_bind(record_type.to_owned())
            .push(" AND tg.record_id IN (");
        let mut ids = builder.separated(", ");
        for id in record_ids {
            ids.push_bind(id.to_string());
        }
        builder.push(") ORDER BY t.name, t.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| Ok((uuid_column(row, "record_id")?, tag_from_row(row)?)))
            .collect()
    }

    pub(crate) async fn add_tagging(
        &self,
        tx: &mut MySqlTx,
        tagging: &Tagging,
    ) -> Result<bool, AppError> {
        let inserted = sqlx::query(
            "INSERT IGNORE INTO taggings (tag_id, record_type, record_id, created_by, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(tagging.tag_id.to_string())
        .bind(tagging.record.record_type.as_str())
        .bind(tagging.record.record_id.to_string())
        .bind(tagging.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(tagging.created_at))
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(inserted > 0)
    }

    pub(crate) async fn remove_tagging(
        &self,
        tx: &mut MySqlTx,
        tag_id: Uuid,
        record: &RecordRef,
    ) -> Result<bool, AppError> {
        let removed = sqlx::query(
            "DELETE FROM taggings WHERE tag_id = ? AND record_type = ? AND record_id = ?",
        )
        .bind(tag_id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(removed > 0)
    }
}
