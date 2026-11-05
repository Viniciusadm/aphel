use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::store::automations::to_json;
use crate::{AutomationRun, AutomationStats, RunListFilter, RunStep};
use aphel_contract_automation::domain::{RunStatus, SkipReason, StepKind, StepStatus};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const RUN_COLUMNS: &str = "id, automation_id, version, trigger_key, trigger_event_id, \
     trigger_type, record_type, record_id, status, depth, chain, reason, error, attempts, started_at, finished_at, \
     resume_at, waiting_step_id, trigger_context";
const STEP_COLUMNS: &str = "id, run_id, position, step_id, kind, action, input, \
     condition_result, next_step_id, output, status, error, started_at, duration_ms";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RunInsert {
    Created,
    Existing(Box<AutomationRun>),
}

#[derive(Clone)]
pub(crate) struct AutomationRunStore {
    pool: MySqlPool,
}

fn unsigned(row: &MySqlRow, column: &str) -> Result<u32, AppError> {
    let value: i32 = row.try_get(column)?;
    Ok(u32::try_from(value).unwrap_or_default())
}

fn optional_json(row: &MySqlRow, column: &str) -> Result<Option<Value>, AppError> {
    Ok(row
        .try_get::<Option<Json<Value>>, _>(column)?
        .map(|Json(value)| value))
}

fn run_from_row(row: &MySqlRow) -> Result<AutomationRun, AppError> {
    let status: String = row.try_get("status")?;
    let record_type: String = row.try_get("record_type")?;
    let record_id: String = row.try_get("record_id")?;
    let Json(chain): Json<Value> = row.try_get("chain")?;
    let reason: Option<String> = row.try_get("reason")?;
    Ok(AutomationRun {
        id: uuid_column(row, "id")?,
        automation_id: uuid_column(row, "automation_id")?,
        version: unsigned(row, "version")?,
        trigger_key: row.try_get("trigger_key")?,
        trigger_event_id: row
            .try_get::<Option<String>, _>("trigger_event_id")?
            .and_then(|id| Uuid::parse_str(&id).ok()),
        trigger_type: row.try_get("trigger_type")?,
        record: RecordRef::parse(&record_type, &record_id)?,
        status: RunStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        depth: unsigned(row, "depth")?,
        chain: serde_json::from_value(chain).unwrap_or_default(),
        reason: reason.as_deref().and_then(SkipReason::parse),
        error: row.try_get("error")?,
        attempts: unsigned(row, "attempts")?,
        started_at: mapper::utc(row.try_get::<NaiveDateTime, _>("started_at")?),
        finished_at: row
            .try_get::<Option<NaiveDateTime>, _>("finished_at")?
            .map(mapper::utc),
        resume_at: row
            .try_get::<Option<NaiveDateTime>, _>("resume_at")?
            .map(mapper::utc),
        waiting_step_id: row.try_get("waiting_step_id")?,
        context: optional_json(row, "trigger_context")?
            .and_then(|value| serde_json::from_value(value).ok()),
    })
}

pub(crate) fn automation_run_from_row(row: &MySqlRow) -> Result<AutomationRun, AppError> {
    run_from_row(row)
}

pub(crate) const AUTOMATION_RUN_COLUMNS: &str = RUN_COLUMNS;

fn step_from_row(row: &MySqlRow) -> Result<RunStep, AppError> {
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    Ok(RunStep {
        id: uuid_column(row, "id")?,
        run_id: uuid_column(row, "run_id")?,
        position: unsigned(row, "position")?,
        step_id: row.try_get("step_id")?,
        kind: StepKind::parse(&kind).ok_or_else(|| mapper::invalid_column("kind", &kind))?,
        action: row.try_get("action")?,
        input: optional_json(row, "input")?,
        condition_result: row.try_get("condition_result")?,
        next_step_id: row.try_get("next_step_id")?,
        output: optional_json(row, "output")?,
        status: StepStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        error: row.try_get("error")?,
        started_at: mapper::utc(row.try_get::<NaiveDateTime, _>("started_at")?),
        duration_ms: unsigned(row, "duration_ms")?,
    })
}

fn is_duplicate(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}

impl AutomationRunStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert_run(&self, run: &AutomationRun) -> Result<RunInsert, AppError> {
        let result = sqlx::query(
            "INSERT INTO automation_runs (id, automation_id, version, trigger_key, trigger_event_id, \
             trigger_type, record_type, record_id, status, depth, chain, reason, error, attempts, started_at, finished_at, \
             resume_at, waiting_step_id, trigger_context) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run.id.to_string())
        .bind(run.automation_id.to_string())
        .bind(run.version)
        .bind(&run.trigger_key)
        .bind(run.trigger_event_id.map(|id| id.to_string()))
        .bind(&run.trigger_type)
        .bind(run.record.record_type.as_str())
        .bind(run.record.record_id.to_string())
        .bind(run.status.as_str())
        .bind(run.depth)
        .bind(to_json(&run.chain)?)
        .bind(run.reason.map(|reason| reason.as_str()))
        .bind(&run.error)
        .bind(run.attempts)
        .bind(mapper::naive(run.started_at))
        .bind(run.finished_at.map(mapper::naive))
        .bind(run.resume_at.map(mapper::naive))
        .bind(&run.waiting_step_id)
        .bind(run.context.as_ref().map(to_json).transpose()?)
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(RunInsert::Created),
            Err(error) if is_duplicate(&error) => self
                .find_by_trigger(run.automation_id, &run.trigger_key)
                .await?
                .map(|run| RunInsert::Existing(Box::new(run)))
                .ok_or_else(|| {
                    AppError::Internal(
                        "automation run vanished after a duplicate insert".to_owned(),
                    )
                }),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn find_run(&self, run_id: Uuid) -> Result<Option<AutomationRun>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RUN_COLUMNS} FROM automation_runs WHERE id = ?"
        )))
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(run_from_row).transpose()
    }

    pub(crate) async fn find_by_trigger(
        &self,
        automation_id: Uuid,
        trigger_key: &str,
    ) -> Result<Option<AutomationRun>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RUN_COLUMNS} FROM automation_runs \
             WHERE automation_id = ? AND trigger_key = ?"
        )))
        .bind(automation_id.to_string())
        .bind(trigger_key)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(run_from_row).transpose()
    }

    pub(crate) async fn update_run(&self, run: &AutomationRun) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE automation_runs SET status = ?, reason = ?, error = ?, attempts = ?, finished_at = ?, \
             resume_at = ?, waiting_step_id = ? WHERE id = ?",
        )
        .bind(run.status.as_str())
        .bind(run.reason.map(|reason| reason.as_str()))
        .bind(&run.error)
        .bind(run.attempts)
        .bind(run.finished_at.map(mapper::naive))
        .bind(run.resume_at.map(mapper::naive))
        .bind(&run.waiting_step_id)
        .bind(run.id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_step(&self, step: &RunStep) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO automation_run_steps (id, run_id, position, step_id, kind, action, input, \
             condition_result, next_step_id, output, status, error, started_at, duration_ms) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE position = VALUES(position), input = VALUES(input), \
             condition_result = VALUES(condition_result), next_step_id = VALUES(next_step_id), \
             output = VALUES(output), status = VALUES(status), error = VALUES(error), \
             started_at = VALUES(started_at), duration_ms = VALUES(duration_ms)",
        )
        .bind(step.id.to_string())
        .bind(step.run_id.to_string())
        .bind(step.position)
        .bind(&step.step_id)
        .bind(step.kind.as_str())
        .bind(&step.action)
        .bind(step.input.as_ref().map(Json))
        .bind(step.condition_result)
        .bind(&step.next_step_id)
        .bind(step.output.as_ref().map(Json))
        .bind(step.status.as_str())
        .bind(&step.error)
        .bind(mapper::naive(step.started_at))
        .bind(step.duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn steps(&self, run_id: Uuid) -> Result<Vec<RunStep>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {STEP_COLUMNS} FROM automation_run_steps \
             WHERE run_id = ? ORDER BY position, started_at"
        )))
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(step_from_row).collect()
    }

    pub(crate) async fn list_runs(
        &self,
        automation_id: Uuid,
        filter: &RunListFilter,
        window: ListWindow,
    ) -> Result<Listing<AutomationRun>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<sqlx::MySql>| {
            builder
                .push(" AND automation_id = ")
                .push_bind(automation_id.to_string());
            if let Some(status) = filter.status {
                builder
                    .push(" AND status = ")
                    .push_bind(status.as_str().to_owned());
            }
            if let Some(record_id) = filter.record_id {
                builder
                    .push(" AND record_id = ")
                    .push_bind(record_id.to_string());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM automation_runs WHERE 1 = 1");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::new(format!(
            "SELECT {RUN_COLUMNS} FROM automation_runs WHERE 1 = 1"
        ));
        push_filter(&mut builder);
        builder
            .push(" ORDER BY started_at DESC, id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(run_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn count_recent(
        &self,
        automation_id: Uuid,
        record_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM automation_runs WHERE automation_id = ? \
             AND record_id = ? AND started_at >= ? AND status <> 'skipped'",
        )
        .bind(automation_id.to_string())
        .bind(record_id.to_string())
        .bind(mapper::naive(since))
        .fetch_one(&self.pool)
        .await?;
        Ok(u64::try_from(total).unwrap_or_default())
    }

    pub(crate) async fn recently_skipped(
        &self,
        automation_id: Uuid,
        reason: SkipReason,
        since: DateTime<Utc>,
        except_run: Uuid,
    ) -> Result<bool, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM automation_runs WHERE automation_id = ? \
             AND status = 'skipped' AND reason = ? AND started_at >= ? AND id <> ?)",
        )
        .bind(automation_id.to_string())
        .bind(reason.as_str())
        .bind(mapper::naive(since))
        .bind(except_run.to_string())
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn stats(
        &self,
        automation_ids: &[Uuid],
        since: DateTime<Utc>,
    ) -> Result<HashMap<Uuid, AutomationStats>, AppError> {
        let mut stats = HashMap::new();
        if automation_ids.is_empty() {
            return Ok(stats);
        }
        let since = mapper::naive(since);
        let mut builder = QueryBuilder::new(
            "SELECT automation_id, MAX(started_at) AS last_run_at, \
             CAST(COALESCE(SUM(started_at >= ",
        );
        builder
            .push_bind(since)
            .push("), 0) AS SIGNED) AS runs_recent, CAST(COALESCE(SUM(status = 'failed' AND started_at >= ")
            .push_bind(since)
            .push(
                "), 0) AS SIGNED) AS failed_recent, CAST(COALESCE(SUM(status = 'waiting'), 0) AS SIGNED) AS waiting \
                 FROM automation_runs WHERE automation_id IN (",
            );
        let mut ids = builder.separated(", ");
        for id in automation_ids {
            ids.push_bind(id.to_string());
        }
        builder.push(") GROUP BY automation_id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        for row in &rows {
            let count = |column: &str| -> Result<u64, AppError> {
                let value: i64 = row.try_get(column)?;
                Ok(u64::try_from(value).unwrap_or_default())
            };
            stats.insert(
                uuid_column(row, "automation_id")?,
                AutomationStats {
                    last_run_at: row
                        .try_get::<Option<NaiveDateTime>, _>("last_run_at")?
                        .map(mapper::utc),
                    runs_30d: count("runs_recent")?,
                    failed_30d: count("failed_recent")?,
                    waiting: count("waiting")?,
                },
            );
        }
        Ok(stats)
    }
}
