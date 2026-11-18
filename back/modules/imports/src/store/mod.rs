pub(crate) mod lookups;

pub use lookups::RecordLookups;

use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{
    ImportJob, ImportJobFilter, ImportOptions, ImportRow, ImportStatus, ImportTotals, RowAction,
    RowError, RowOutcome, RowStatus, SourceRow, TextEncoding, delimiter_from_label,
    delimiter_label,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const ROW_INSERT_CHUNK: usize = 500;

const JOB_COLUMNS: &str = "id, record_type, file_id, file_name, status, encoding, \
     delimiter, headers, preview, row_count, mapping, options, totals, error_file_id, retry_of, \
     failure_reason, created_by, created_at, updated_at, started_at, finished_at, privacy_redacted_at";

#[derive(Clone)]
pub(crate) struct ImportStore {
    pool: MySqlPool,
}

impl ImportStore {
    pub(crate) async fn delete_job(&self, tx: &mut Tx, job: &ImportJob) -> Result<(), AppError> {
        sqlx::query("DELETE FROM import_jobs WHERE id = ?")
            .bind(job.id.to_string())
            .execute(&mut **tx)
            .await?;
        for file_id in [job.file_id, job.error_file_id].into_iter().flatten() {
            sqlx::query("DELETE FROM files WHERE id = ?")
                .bind(file_id.to_string())
                .execute(&mut **tx)
                .await?;
        }
        Ok(())
    }

    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("failed to serialize import data: {error}")))
}

fn from_json<T: serde::de::DeserializeOwned>(row: &MySqlRow, column: &str) -> Result<T, AppError> {
    let Json(value): Json<Value> = row.try_get(column)?;
    serde_json::from_value(value).map_err(|_| mapper::invalid_column(column, "json"))
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value)))
        .transpose()
}

fn preview_rows(value: Vec<(u32, Vec<String>)>) -> Vec<SourceRow> {
    value
        .into_iter()
        .map(|(row_index, cells)| SourceRow { row_index, cells })
        .collect()
}

fn job_from_row(row: &MySqlRow) -> Result<ImportJob, AppError> {
    let status: String = row.try_get("status")?;
    let encoding: String = row.try_get("encoding")?;
    let delimiter: String = row.try_get("delimiter")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let started_at: Option<NaiveDateTime> = row.try_get("started_at")?;
    let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
    let row_count: u32 = row.try_get("row_count")?;
    Ok(ImportJob {
        id: uuid_column(row, "id")?,
        record_type: row.try_get("record_type")?,
        file_id: optional_uuid(row, "file_id")?,
        privacy_redacted_at: row
            .try_get::<Option<NaiveDateTime>, _>("privacy_redacted_at")?
            .map(mapper::utc),
        file_name: row.try_get("file_name")?,
        status: ImportStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        encoding: TextEncoding::parse(&encoding)
            .ok_or_else(|| mapper::invalid_column("encoding", &encoding))?,
        delimiter: delimiter_from_label(&delimiter)
            .ok_or_else(|| mapper::invalid_column("delimiter", &delimiter))?,
        headers: from_json(row, "headers")?,
        preview: preview_rows(from_json(row, "preview")?),
        row_count,
        mapping: from_json(row, "mapping")?,
        options: from_json::<ImportOptions>(row, "options")?,
        totals: from_json::<ImportTotals>(row, "totals")?,
        error_file_id: optional_uuid(row, "error_file_id")?,
        retry_of: optional_uuid(row, "retry_of")?,
        failure_reason: row.try_get("failure_reason")?,
        created_by: mapper::typed_id(row.try_get("created_by")?, UserId::new)?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
        started_at: started_at.map(mapper::utc),
        finished_at: finished_at.map(mapper::utc),
    })
}

fn row_from_row(row: &MySqlRow) -> Result<ImportRow, AppError> {
    let status: String = row.try_get("status")?;
    let action: Option<String> = row.try_get("action")?;
    let errors: Option<Json<Value>> = row.try_get("errors")?;
    let errors: Vec<RowError> = match errors {
        Some(Json(value)) => {
            serde_json::from_value(value).map_err(|_| mapper::invalid_column("errors", "json"))?
        }
        None => Vec::new(),
    };
    let row_index: u32 = row.try_get("row_index")?;
    Ok(ImportRow {
        row_index,
        status: RowStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        action: action
            .map(|action| {
                RowAction::parse(&action).ok_or_else(|| mapper::invalid_column("action", &action))
            })
            .transpose()?,
        record_id: optional_uuid(row, "record_id")?,
        errors,
        cells: from_json(row, "raw")?,
    })
}

fn preview_json(job: &ImportJob) -> Result<Json<Value>, AppError> {
    let preview: Vec<(u32, &Vec<String>)> = job
        .preview
        .iter()
        .map(|row| (row.row_index, &row.cells))
        .collect();
    to_json(&preview)
}

impl ImportStore {
    pub(crate) async fn insert_job(&self, tx: &mut Tx, job: &ImportJob) -> Result<(), AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO import_jobs ({JOB_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )))
        .bind(job.id.to_string())
        .bind(&job.record_type)
        .bind(job.file_id.map(|id| id.to_string()))
        .bind(&job.file_name)
        .bind(job.status.as_str())
        .bind(job.encoding.as_str())
        .bind(delimiter_label(job.delimiter))
        .bind(to_json(&job.headers)?)
        .bind(preview_json(job)?)
        .bind(job.row_count)
        .bind(to_json(&job.mapping)?)
        .bind(to_json(&job.options)?)
        .bind(to_json(&job.totals)?)
        .bind(job.error_file_id.map(|id| id.to_string()))
        .bind(job.retry_of.map(|id| id.to_string()))
        .bind(&job.failure_reason)
        .bind(job.created_by.as_str())
        .bind(mapper::naive(job.created_at))
        .bind(mapper::naive(job.updated_at))
        .bind(job.started_at.map(mapper::naive))
        .bind(job.finished_at.map(mapper::naive))
        .bind(job.privacy_redacted_at.map(mapper::naive))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_job(&self, tx: &mut Tx, job: &ImportJob) -> Result<(), AppError> {
        let redacted: Option<Option<NaiveDateTime>> = sqlx::query_scalar(
            "SELECT privacy_redacted_at FROM import_jobs WHERE id = ? FOR UPDATE",
        )
        .bind(job.id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        match redacted {
            Some(None) => {}
            Some(Some(_)) => {
                return Err(AppError::conflict_code(
                    crate::IMPORT_NOT_RETRYABLE,
                    "privacy-redacted imports cannot be changed or retried",
                ));
            }
            None => return Err(crate::import_not_found()),
        }
        sqlx::query(
            "UPDATE import_jobs SET status = ?, row_count = ?, mapping = ?, options = ?, totals = ?, \
             error_file_id = ?, failure_reason = ?, updated_at = ?, started_at = ?, finished_at = ? \
             WHERE id = ?",
        )
        .bind(job.status.as_str())
        .bind(job.row_count)
        .bind(to_json(&job.mapping)?)
        .bind(to_json(&job.options)?)
        .bind(to_json(&job.totals)?)
        .bind(job.error_file_id.map(|id| id.to_string()))
        .bind(&job.failure_reason)
        .bind(mapper::naive(job.updated_at))
        .bind(job.started_at.map(mapper::naive))
        .bind(job.finished_at.map(mapper::naive))
        .bind(job.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find_job(&self, import_id: Uuid) -> Result<Option<ImportJob>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {JOB_COLUMNS} FROM import_jobs WHERE id = ?"
        )))
        .bind(import_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(job_from_row)
        .transpose()
    }

    pub(crate) async fn list_jobs(
        &self,
        filter: &ImportJobFilter,
        window: ListWindow,
    ) -> Result<Listing<ImportJob>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<sqlx::MySql>| {
            builder.push(" WHERE 1 = 1");
            if let Some(user) = &filter.created_by {
                builder
                    .push(" AND created_by = ")
                    .push_bind(user.as_str().to_owned());
            }
            if let Some(record_type) = &filter.record_type {
                builder
                    .push(" AND record_type = ")
                    .push_bind(record_type.clone());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM import_jobs");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut select = QueryBuilder::new(format!("SELECT {JOB_COLUMNS} FROM import_jobs"));
        push_filter(&mut select);
        select
            .push(" ORDER BY created_at DESC, id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(job_from_row)
            .collect::<Result<_, _>>()?;
        Ok(Listing {
            items,
            total: total as u64,
        })
    }

    pub(crate) async fn insert_rows(
        &self,
        tx: &mut Tx,
        import_id: Uuid,
        rows: &[SourceRow],
    ) -> Result<(), AppError> {
        for chunk in rows.chunks(ROW_INSERT_CHUNK) {
            let mut builder = QueryBuilder::new(
                "INSERT INTO import_rows (import_job_id, row_index, status, raw) ",
            );
            let mut failure = None;
            builder.push_values(chunk, |mut values, row| {
                let raw = match to_json(&row.cells) {
                    Ok(raw) => raw,
                    Err(error) => {
                        failure = Some(error);
                        Json(Value::Null)
                    }
                };
                values
                    .push_bind(import_id.to_string())
                    .push_bind(row.row_index)
                    .push_bind(RowStatus::Pending.as_str())
                    .push_bind(raw);
            });
            if let Some(error) = failure {
                return Err(error);
            }
            builder.build().execute(&mut **tx).await?;
        }
        Ok(())
    }

    pub(crate) async fn pending_rows(
        &self,
        import_id: Uuid,
        limit: usize,
    ) -> Result<Vec<ImportRow>, AppError> {
        sqlx::query(
            "SELECT row_index, status, action, record_id, errors, raw FROM import_rows \
             WHERE import_job_id = ? AND status = 'pending' \
             ORDER BY row_index LIMIT ?",
        )
        .bind(import_id.to_string())
        .bind(limit as u64)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(row_from_row)
        .collect()
    }

    pub(crate) async fn finish_row(
        &self,
        import_id: Uuid,
        outcome: &RowOutcome,
    ) -> Result<(), AppError> {
        let errors = if outcome.errors.is_empty() {
            None
        } else {
            Some(to_json(&outcome.errors)?)
        };
        sqlx::query(
            "UPDATE import_rows SET status = ?, action = ?, record_id = ?, errors = ?, \
             processed_at = CURRENT_TIMESTAMP(6) \
             WHERE import_job_id = ? AND row_index = ?",
        )
        .bind(outcome.status.as_str())
        .bind(outcome.action.map(|action| action.as_str()))
        .bind(outcome.record_id.map(|id| id.to_string()))
        .bind(errors)
        .bind(import_id.to_string())
        .bind(outcome.row_index)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_row_written(
        tx: &mut Tx,
        import_id: Uuid,
        row_index: u32,
        action: RowAction,
        record_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE import_rows SET status = ?, action = ?, record_id = ?, errors = NULL, \
             processed_at = CURRENT_TIMESTAMP(6) \
             WHERE import_job_id = ? AND row_index = ? AND status = ?",
        )
        .bind(RowStatus::Imported.as_str())
        .bind(action.as_str())
        .bind(record_id.to_string())
        .bind(import_id.to_string())
        .bind(row_index)
        .bind(RowStatus::Pending.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn count_rows(&self, import_id: Uuid) -> Result<ImportTotals, AppError> {
        let rows = sqlx::query(
            "SELECT status, action, COUNT(*) AS total FROM import_rows \
             WHERE import_job_id = ? GROUP BY status, action",
        )
        .bind(import_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut totals = ImportTotals::default();
        for row in rows {
            let status: String = row.try_get("status")?;
            let action: Option<String> = row.try_get("action")?;
            let count: i64 = row.try_get("total")?;
            let count = count as u64;
            totals.total += count;
            match (status.as_str(), action.as_deref()) {
                ("pending", _) => totals.pending += count,
                ("imported", Some("updated")) => totals.updated += count,
                ("imported", _) => totals.imported += count,
                ("skipped", _) => totals.skipped += count,
                ("failed", _) => totals.failed += count,
                _ => return Err(mapper::invalid_column("status", &status)),
            }
        }
        Ok(totals)
    }

    pub(crate) async fn update_totals(
        &self,
        import_id: Uuid,
        totals: &ImportTotals,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE import_jobs SET totals = ?, updated_at = CURRENT_TIMESTAMP(6) \
             WHERE id = ?",
        )
        .bind(to_json(totals)?)
        .bind(import_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn list_rows(
        &self,
        import_id: Uuid,
        status: Option<RowStatus>,
        window: ListWindow,
    ) -> Result<Listing<ImportRow>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<sqlx::MySql>| {
            builder
                .push(" WHERE import_job_id = ")
                .push_bind(import_id.to_string());
            if let Some(status) = status {
                builder.push(" AND status = ").push_bind(status.as_str());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM import_rows");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut select = QueryBuilder::new(
            "SELECT row_index, status, action, record_id, errors, raw FROM import_rows",
        );
        push_filter(&mut select);
        select
            .push(" ORDER BY row_index LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(row_from_row)
            .collect::<Result<_, _>>()?;
        Ok(Listing {
            items,
            total: total as u64,
        })
    }

    pub(crate) async fn reset_failed_rows(
        &self,
        tx: &mut Tx,
        import_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE import_rows SET status = 'pending', action = NULL, record_id = NULL, \
             errors = NULL, processed_at = NULL \
             WHERE import_job_id = ? AND status = 'failed'",
        )
        .bind(import_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }
}
