use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::{ExportSummary, OrgExport, OrgExportStatus};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

const SELECT_COLUMNS: &str = "e.id, e.status, e.include_files, e.file_id, \
     e.summary, e.failure_reason, e.requested_by, u.display_name AS requested_by_name, \
     e.created_at, e.updated_at, e.started_at, e.finished_at";

#[derive(Clone)]
pub(crate) struct OrgExportStore {
    pool: MySqlPool,
}

impl OrgExportStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn summary_json(summary: &Option<ExportSummary>) -> Result<Option<Json<Value>>, AppError> {
    summary
        .as_ref()
        .map(|summary| {
            serde_json::to_value(summary)
                .map(Json)
                .map_err(|error| AppError::Internal(error.to_string()))
        })
        .transpose()
}

fn export_from_row(row: &MySqlRow) -> Result<OrgExport, AppError> {
    let status: String = row.try_get("status")?;
    let file_id: Option<String> = row.try_get("file_id")?;
    let summary: Option<Json<Value>> = row.try_get("summary")?;
    let include_files: bool = row.try_get("include_files")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let started_at: Option<NaiveDateTime> = row.try_get("started_at")?;
    let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
    Ok(OrgExport {
        id: uuid_column(row, "id")?,
        status: OrgExportStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        include_files,
        file_id: file_id
            .map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("file_id", &id)))
            .transpose()?,
        summary: summary.and_then(|Json(value)| serde_json::from_value(value).ok()),
        failure_reason: row.try_get("failure_reason")?,
        requested_by: mapper::typed_id(row.try_get("requested_by")?, UserId::new)?,
        requested_by_name: row.try_get("requested_by_name")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
        started_at: started_at.map(mapper::utc),
        finished_at: finished_at.map(mapper::utc),
    })
}

impl OrgExportStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        export: &OrgExport,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO organization_exports (id, status, include_files, file_id, \
             summary, failure_reason, requested_by, created_at, updated_at, started_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(export.id.to_string())
        .bind(export.status.as_str())
        .bind(export.include_files)
        .bind(export.file_id.map(|id| id.to_string()))
        .bind(summary_json(&export.summary)?)
        .bind(&export.failure_reason)
        .bind(export.requested_by.as_str())
        .bind(mapper::naive(export.created_at))
        .bind(mapper::naive(export.updated_at))
        .bind(export.started_at.map(mapper::naive))
        .bind(export.finished_at.map(mapper::naive))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save(&self, tx: &mut MySqlTx, export: &OrgExport) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE organization_exports SET status = ?, file_id = ?, summary = ?, failure_reason = ?, \
             updated_at = ?, started_at = ?, finished_at = ? WHERE id = ?",
        )
        .bind(export.status.as_str())
        .bind(export.file_id.map(|id| id.to_string()))
        .bind(summary_json(&export.summary)?)
        .bind(&export.failure_reason)
        .bind(mapper::naive(export.updated_at))
        .bind(export.started_at.map(mapper::naive))
        .bind(export.finished_at.map(mapper::naive))
        .bind(export.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find(&self, export_id: Uuid) -> Result<Option<OrgExport>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {SELECT_COLUMNS} FROM organization_exports e LEFT JOIN users u ON u.id = e.requested_by \
             WHERE e.id = ?"
        )))
        .bind(export_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(export_from_row)
        .transpose()
    }

    pub(crate) async fn list(&self, window: ListWindow) -> Result<Listing<OrgExport>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization_exports")
            .fetch_one(&self.pool)
            .await?;
        let items = sqlx::query(AssertSqlSafe(format!(
            "SELECT {SELECT_COLUMNS} FROM organization_exports e LEFT JOIN users u ON u.id = e.requested_by \
             ORDER BY e.created_at DESC, e.id DESC LIMIT ? OFFSET ?"
        )))
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

    pub(crate) async fn has_pending(&self, tx: &mut MySqlTx) -> Result<bool, AppError> {
        sqlx::query("SELECT id FROM organization_settings WHERE id = 1 FOR UPDATE")
            .fetch_optional(&mut **tx)
            .await?;
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM organization_exports \
             WHERE status IN ('queued', 'running') AND updated_at > UTC_TIMESTAMP(6) - INTERVAL 6 HOUR",
        )
        .fetch_one(&mut **tx)
        .await?;
        Ok(pending > 0)
    }
}
