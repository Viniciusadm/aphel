use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::Segment;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::EntityDefinition;
use aphel_kernel::application::query::{Filter, FilterSource};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::quoted;
use aphel_kernel::infrastructure::repositories::mapper;

const SEGMENT_COLUMNS: &str =
    "id, record_type, name, description, filter, created_by, created_at, updated_at";

fn segment_from_row(row: &MySqlRow) -> Result<Segment, AppError> {
    let id: String = row.try_get("id")?;
    let record_type: String = row.try_get("record_type")?;
    let Json(filter): Json<Value> = row.try_get("filter")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(Segment {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        record_type: RecordType::parse(record_type.clone())
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        filter: serde_json::from_value::<Filter>(filter.clone())
            .map_err(|_| mapper::invalid_column("filter", &filter.to_string()))?,
        created_by: row
            .try_get::<Option<String>, _>("created_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn filter_json(filter: &Filter) -> Result<Json<Value>, AppError> {
    serde_json::to_value(filter)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("cannot serialize segment filter: {error}")))
}

#[derive(Clone)]
pub(crate) struct SegmentStore {
    pool: MySqlPool,
}

impl SegmentStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(format!("SELECT {SEGMENT_COLUMNS} FROM segments WHERE TRUE"))
    }

    pub(crate) async fn list(
        &self,
        record_type: Option<&RecordType>,
    ) -> Result<Vec<Segment>, AppError> {
        let mut builder = Self::select();
        if let Some(record_type) = record_type {
            builder
                .push(" AND record_type = ")
                .push_bind(record_type.as_str().to_owned());
        }
        builder.push(" ORDER BY record_type, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(segment_from_row).collect()
    }

    pub(crate) async fn find(&self, segment_id: Uuid) -> Result<Option<Segment>, AppError> {
        let mut builder = Self::select();
        builder.push(" AND id = ").push_bind(segment_id.to_string());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(segment_from_row)
            .transpose()
    }

    pub(crate) async fn find_by_name(
        &self,
        record_type: &RecordType,
        name: &str,
    ) -> Result<Option<Segment>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND record_type = ")
            .push_bind(record_type.as_str().to_owned())
            .push(" AND name = ")
            .push_bind(name.to_owned());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(segment_from_row)
            .transpose()
    }

    pub(crate) async fn source(&self, segment_id: Uuid) -> Result<Option<FilterSource>, AppError> {
        let row: Option<(String, Json<Value>)> =
            sqlx::query_as("SELECT record_type, filter FROM segments WHERE id = ?")
                .bind(segment_id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        let Some((record_type, Json(filter))) = row else {
            return Ok(None);
        };
        let filter = serde_json::from_value::<Filter>(filter).map_err(|_| {
            AppError::Persistence(format!("segment {segment_id} has an invalid filter"))
        })?;
        Ok(Some(FilterSource {
            record_type,
            filter,
        }))
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, segment: &Segment) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO segments (id, record_type, name, description, filter, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(segment.id.to_string())
        .bind(segment.record_type.as_str())
        .bind(&segment.name)
        .bind(segment.description.as_deref())
        .bind(filter_json(&segment.filter)?)
        .bind(segment.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(segment.created_at))
        .bind(mapper::naive(segment.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut MySqlTx, segment: &Segment) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE segments SET name = ?, description = ?, filter = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&segment.name)
        .bind(segment.description.as_deref())
        .bind(filter_json(&segment.filter)?)
        .bind(mapper::naive(segment.updated_at))
        .bind(segment.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, segment_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM segments WHERE id = ?")
            .bind(segment_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn labels(
        &self,
        definition: &EntityDefinition,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let mut labels = HashMap::new();
        let Some(title) = definition.title_column() else {
            return Ok(labels);
        };
        for chunk in record_ids.chunks(500) {
            let mut builder = QueryBuilder::<MySql>::new(format!(
                "SELECT r.id, CAST(r.{} AS CHAR) AS label FROM {} r WHERE ",
                quoted(title),
                quoted(&definition.table)
            ));
            if let Some(column) = &definition.deleted_column {
                builder.push(format!("r.{} IS NULL AND ", quoted(column)));
            }
            builder.push("r.id IN (");
            let mut ids = builder.separated(", ");
            for id in chunk {
                ids.push_bind(id.to_string());
            }
            builder.push(")");
            for row in builder.build().fetch_all(&self.pool).await? {
                let id: String = row.try_get("id")?;
                let label: Option<String> = row.try_get("label")?;
                if let (Ok(id), Some(label)) = (Uuid::parse_str(&id), label) {
                    labels.insert(id, label);
                }
            }
        }
        Ok(labels)
    }
}
