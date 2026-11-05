use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::store::automations::{
    AUTOMATION_COLUMNS, VERSION_COLUMNS, automation_from_row, version_from_row,
};
use crate::store::runs::{AUTOMATION_RUN_COLUMNS, automation_run_from_row};
use crate::{ActiveAutomation, AutomationRun};
use aphel_contract_automation::domain::{RunStatus, SkipReason};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimeCursor {
    pub(crate) automation_id: Uuid,
    pub(crate) version: u32,
    pub(crate) cursor_at: DateTime<Utc>,
    pub(crate) page_offset: u64,
    pub(crate) occurrence_at: Option<DateTime<Utc>>,
}

impl TimeCursor {
    pub(crate) fn start(automation_id: Uuid, version: u32, at: DateTime<Utc>) -> Self {
        Self {
            automation_id,
            version,
            cursor_at: at,
            page_offset: 0,
            occurrence_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CancelTarget {
    Automation(Uuid),
    Record(RecordRef),
}

#[derive(Clone)]
pub(crate) struct AutomationTimeStore {
    pool: MySqlPool,
}

fn push_target(
    builder: &mut QueryBuilder<sqlx::MySql>,
    column_prefix: &str,
    target: &CancelTarget,
) {
    match target {
        CancelTarget::Automation(automation_id) => {
            builder
                .push(format!(" AND {column_prefix}automation_id = "))
                .push_bind(automation_id.to_string());
        }
        CancelTarget::Record(record) => {
            builder
                .push(format!(" AND {column_prefix}record_type = "))
                .push_bind(record.record_type.as_str().to_owned())
                .push(format!(" AND {column_prefix}record_id = "))
                .push_bind(record.record_id.to_string());
        }
    }
}

impl AutomationTimeStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn time_automations(
        &self,
        limit: u32,
    ) -> Result<Vec<ActiveAutomation>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_COLUMNS}, {VERSION_COLUMNS} FROM automations a \
             JOIN automation_versions v ON v.automation_id = a.id AND v.version = a.version \
             LEFT JOIN automation_time_cursors c ON c.automation_id = a.id \
             WHERE a.status = 'active' \
             AND v.trigger_type LIKE 'time.%' \
             ORDER BY (c.swept_at IS NOT NULL), c.swept_at, a.id LIMIT ?"
        )))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ActiveAutomation {
                    automation: automation_from_row(row)?,
                    version: version_from_row(row)?,
                })
            })
            .collect()
    }

    pub(crate) async fn cursor(&self, automation_id: Uuid) -> Result<Option<TimeCursor>, AppError> {
        let row = sqlx::query(
            "SELECT version, cursor_at, page_offset, occurrence_at FROM automation_time_cursors \
             WHERE automation_id = ?",
        )
        .bind(automation_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let version: i32 = row.try_get("version")?;
            let page_offset: i64 = row.try_get("page_offset")?;
            Ok(TimeCursor {
                automation_id,
                version: u32::try_from(version).unwrap_or_default(),
                cursor_at: mapper::utc(row.try_get::<NaiveDateTime, _>("cursor_at")?),
                page_offset: u64::try_from(page_offset).unwrap_or_default(),
                occurrence_at: row
                    .try_get::<Option<NaiveDateTime>, _>("occurrence_at")?
                    .map(mapper::utc),
            })
        })
        .transpose()
    }

    pub(crate) async fn save_cursor(
        &self,
        cursor: &TimeCursor,
        swept_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO automation_time_cursors (automation_id, version, cursor_at, \
             page_offset, occurrence_at, swept_at) VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE version = VALUES(version), cursor_at = VALUES(cursor_at), \
             page_offset = VALUES(page_offset), occurrence_at = VALUES(occurrence_at), swept_at = VALUES(swept_at)",
        )
        .bind(cursor.automation_id.to_string())
        .bind(cursor.version)
        .bind(mapper::naive(cursor.cursor_at))
        .bind(i64::try_from(cursor.page_offset).unwrap_or(i64::MAX))
        .bind(cursor.occurrence_at.map(mapper::naive))
        .bind(mapper::naive(swept_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn claim_due(
        &self,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AutomationRun>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_RUN_COLUMNS} FROM automation_runs \
             WHERE status = 'waiting' AND resume_at <= ? \
             AND automation_id IN (SELECT id FROM automations WHERE status = 'active') \
             ORDER BY resume_at LIMIT ? FOR UPDATE SKIP LOCKED"
        )))
        .bind(mapper::naive(now))
        .bind(limit)
        .fetch_all(&mut **tx)
        .await?;
        let mut runs = rows
            .iter()
            .map(automation_run_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        if runs.is_empty() {
            return Ok(runs);
        }
        let mut builder = QueryBuilder::new(
            "UPDATE automation_runs SET status = 'running', resume_at = NULL WHERE status = 'waiting' AND id IN (",
        );
        let mut ids = builder.separated(", ");
        for run in &runs {
            ids.push_bind(run.id.to_string());
        }
        builder.push(")");
        builder.build().execute(&mut **tx).await?;
        for run in &mut runs {
            run.status = RunStatus::Running;
            run.resume_at = None;
        }
        Ok(runs)
    }

    pub(crate) async fn cancel_waiting(
        &self,
        tx: &mut MySqlTx,
        target: &CancelTarget,
        reason: SkipReason,
        now: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut steps = QueryBuilder::new(
            "UPDATE automation_run_steps s JOIN automation_runs r ON r.id = s.run_id \
             SET s.status = 'skipped' WHERE s.status = 'waiting' AND r.status = 'waiting'",
        );
        push_target(&mut steps, "r.", target);
        steps.build().execute(&mut **tx).await?;
        let mut runs = QueryBuilder::new(
            "UPDATE automation_runs SET status = 'cancelled', resume_at = NULL, waiting_step_id = NULL, reason = ",
        );
        runs.push_bind(reason.as_str().to_owned())
            .push(", finished_at = ")
            .push_bind(mapper::naive(now))
            .push(" WHERE status = 'waiting'");
        push_target(&mut runs, "", target);
        let result = runs.build().execute(&mut **tx).await?;
        Ok(result.rows_affected())
    }
}
