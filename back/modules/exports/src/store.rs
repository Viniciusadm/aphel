use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{ExportFormat, ExportJob, ExportQuery, ExportStatus};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{FieldSet, RecordTypeRegistry, load_values};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::fields::MySqlCustomFieldValueRepository;
use aphel_sdk::platform::infrastructure::query::compiler::{push_partition, quoted};
use aphel_sdk::platform::infrastructure::query::computed::system_expression;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const EXPORT_COLUMNS: &str = "id, record_type, format, status, query, columns, \
     file_id, row_count, failure_reason, requested_by, created_at, updated_at, finished_at";
const LOAD_CHUNK: usize = 500;
const LABEL_CHUNK: usize = 500;

fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("failed to serialize export data: {error}")))
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn export_from_row(row: &MySqlRow) -> Result<ExportJob, AppError> {
    let format: String = row.try_get("format")?;
    let status: String = row.try_get("status")?;
    let Json(query): Json<Value> = row.try_get("query")?;
    let Json(columns): Json<Value> = row.try_get("columns")?;
    let file_id: Option<String> = row.try_get("file_id")?;
    let row_count: Option<u32> = row.try_get("row_count")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
    Ok(ExportJob {
        id: uuid_column(row, "id")?,
        record_type: row.try_get("record_type")?,
        format: ExportFormat::parse(&format)
            .ok_or_else(|| mapper::invalid_column("format", &format))?,
        status: ExportStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        query: serde_json::from_value::<ExportQuery>(query)
            .map_err(|_| mapper::invalid_column("query", "json"))?,
        columns: serde_json::from_value(columns)
            .map_err(|_| mapper::invalid_column("columns", "json"))?,
        file_id: file_id
            .map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("file_id", &id)))
            .transpose()?,
        row_count: row_count.map(u64::from),
        failure_reason: row.try_get("failure_reason")?,
        requested_by: mapper::typed_id(row.try_get("requested_by")?, UserId::new)?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
        finished_at: finished_at.map(mapper::utc),
    })
}

#[derive(Clone)]
pub(crate) struct ExportStore {
    pool: MySqlPool,
}

impl ExportStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert(&self, tx: &mut Tx, job: &ExportJob) -> Result<(), AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO export_jobs ({EXPORT_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )))
        .bind(job.id.to_string())
        .bind(&job.record_type)
        .bind(job.format.as_str())
        .bind(job.status.as_str())
        .bind(to_json(&job.query)?)
        .bind(to_json(&job.columns)?)
        .bind(job.file_id.map(|id| id.to_string()))
        .bind(job.row_count.map(|count| count as u32))
        .bind(&job.failure_reason)
        .bind(job.requested_by.as_str())
        .bind(mapper::naive(job.created_at))
        .bind(mapper::naive(job.updated_at))
        .bind(job.finished_at.map(mapper::naive))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save(&self, tx: &mut Tx, job: &ExportJob) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE export_jobs SET status = ?, file_id = ?, row_count = ?, failure_reason = ?, \
             updated_at = ?, finished_at = ? WHERE id = ?",
        )
        .bind(job.status.as_str())
        .bind(job.file_id.map(|id| id.to_string()))
        .bind(job.row_count.map(|count| count as u32))
        .bind(&job.failure_reason)
        .bind(mapper::naive(job.updated_at))
        .bind(job.finished_at.map(mapper::naive))
        .bind(job.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut Tx, job: &ExportJob) -> Result<(), AppError> {
        sqlx::query("DELETE FROM export_jobs WHERE id = ?")
            .bind(job.id.to_string())
            .execute(&mut **tx)
            .await?;
        if let Some(file_id) = job.file_id {
            sqlx::query("DELETE FROM files WHERE id = ?")
                .bind(file_id.to_string())
                .execute(&mut **tx)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn find(&self, export_id: Uuid) -> Result<Option<ExportJob>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {EXPORT_COLUMNS} FROM export_jobs WHERE id = ?"
        )))
        .bind(export_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(export_from_row)
        .transpose()
    }

    pub(crate) async fn list(
        &self,
        requested_by: &UserId,
        window: ListWindow,
    ) -> Result<Listing<ExportJob>, AppError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM export_jobs WHERE requested_by = ?")
                .bind(requested_by.as_str())
                .fetch_one(&self.pool)
                .await?;
        let items = sqlx::query(AssertSqlSafe(format!(
            "SELECT {EXPORT_COLUMNS} FROM export_jobs WHERE requested_by = ? \
             ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
        )))
        .bind(requested_by.as_str())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(export_from_row)
        .collect::<Result<_, _>>()?;
        Ok(Listing {
            items,
            total: total as u64,
        })
    }
}

#[derive(Clone)]
pub(crate) struct RecordRowStore {
    pool: MySqlPool,
    values: MySqlCustomFieldValueRepository,
}

impl RecordRowStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            values: MySqlCustomFieldValueRepository::new(pool.clone()),
            pool,
        }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn load_rows(
        &self,
        fields: &FieldSet,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Map<String, Value>>, AppError> {
        let mut rows: HashMap<Uuid, Map<String, Value>> = HashMap::new();
        if record_ids.is_empty() {
            return Ok(rows);
        }
        let system: Vec<(String, String)> = fields
            .active()
            .filter_map(|field| {
                field
                    .system
                    .as_ref()
                    .map(|system| (field.key().to_owned(), system_expression(system, "r")))
            })
            .collect();
        let definition = &fields.record_type;
        for chunk in record_ids.chunks(LOAD_CHUNK) {
            let mut builder = QueryBuilder::new("SELECT r.id");
            for (index, (_, expression)) in system.iter().enumerate() {
                builder.push(format!(", CAST({expression} AS CHAR) AS c{index}"));
            }
            builder.push(format!(" FROM {} r WHERE 1 = 1", quoted(&definition.table)));
            push_partition(&mut builder, definition, "r");
            if let Some(column) = &definition.deleted_column {
                builder.push(format!(" AND r.{} IS NULL", quoted(column)));
            }
            builder.push(" AND r.id IN (");
            let mut ids = builder.separated(", ");
            for id in chunk {
                ids.push_bind(id.to_string());
            }
            builder.push(")");
            for row in builder.build().fetch_all(&self.pool).await? {
                let id = uuid_column(&row, "id")?;
                let mut values = Map::new();
                for (index, (key, _)) in system.iter().enumerate() {
                    let value: Option<String> = row.try_get(format!("c{index}").as_str())?;
                    values.insert(key.clone(), value.map_or(Value::Null, Value::String));
                }
                rows.insert(id, values);
            }
            for (id, custom) in load_values(&self.values, fields, chunk).await? {
                if let Some(values) = rows.get_mut(&id) {
                    values.extend(custom);
                }
            }
        }
        Ok(rows)
    }
}

#[derive(Clone)]
pub(crate) struct RecordLabelStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl RecordLabelStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    pub(crate) async fn user_labels(
        &self,
        user_ids: &[String],
    ) -> Result<HashMap<String, String>, AppError> {
        let mut labels = HashMap::new();
        for chunk in user_ids.chunks(LABEL_CHUNK) {
            let mut builder =
                QueryBuilder::new("SELECT u.id, u.email FROM users u WHERE u.id IN (");
            let mut ids = builder.separated(", ");
            for id in chunk {
                ids.push_bind(id.clone());
            }
            builder.push(")");
            for row in builder.build().fetch_all(&self.pool).await? {
                labels.insert(row.try_get("id")?, row.try_get("email")?);
            }
        }
        Ok(labels)
    }

    pub(crate) async fn record_labels(
        &self,
        record_type: &str,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let Ok(definition) = self.registry.require(record_type) else {
            return Ok(HashMap::new());
        };
        let Some(title) = definition
            .title_field
            .as_deref()
            .and_then(|key| definition.system_field_by_key(key))
        else {
            return Ok(HashMap::new());
        };
        let mut labels = HashMap::new();
        for chunk in record_ids.chunks(LABEL_CHUNK) {
            let mut builder = QueryBuilder::new(format!(
                "SELECT r.id, {} AS label FROM {} r WHERE TRUE",
                system_expression(title, "r"),
                quoted(&definition.table)
            ));
            push_partition(&mut builder, definition, "r");
            if let Some(column) = &definition.deleted_column {
                builder.push(format!(" AND r.{} IS NULL", quoted(column)));
            }
            builder.push(" AND r.id IN (");
            let mut ids = builder.separated(", ");
            for id in chunk {
                ids.push_bind(id.to_string());
            }
            builder.push(")");
            for row in builder.build().fetch_all(&self.pool).await? {
                let id: String = row.try_get("id")?;
                if let Ok(id) = Uuid::parse_str(&id) {
                    labels.insert(id, row.try_get("label")?);
                }
            }
        }
        Ok(labels)
    }
}
