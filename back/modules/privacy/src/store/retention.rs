use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    RetentionAction, RetentionPolicy, RetentionRun, RetentionRunFilter, RetentionRunStatus,
    RetentionTrigger, StoredFileRef, tracks_last_activity,
};
use crate::mysql::sql::{
    file_reference_columns, instant, optional_instant, optional_uuid, record_title_sql,
    unreferenced_file_predicate, user_reference, uuid_column,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{EntityDefinition, is_sql_identifier};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::push_partition;
use aphel_kernel::infrastructure::repositories::mapper;

const POLICY_SELECT: &str = "SELECT p.*, u.display_name AS created_by_name FROM retention_policies p \
     LEFT JOIN users u ON u.id = p.created_by";
const RUN_SELECT: &str = "SELECT r.*, u.display_name AS requested_by_name FROM retention_runs r \
     LEFT JOIN users u ON u.id = r.requested_by";

fn action_column(row: &MySqlRow) -> Result<RetentionAction, AppError> {
    let action: String = row.try_get("action")?;
    RetentionAction::parse(&action).ok_or_else(|| mapper::invalid_column("action", &action))
}

fn policy_from_row(row: &MySqlRow) -> Result<RetentionPolicy, AppError> {
    Ok(RetentionPolicy {
        id: uuid_column(row, "id")?,
        name: row.try_get("name")?,
        action: action_column(row)?,
        record_type: row.try_get("record_type")?,
        threshold_days: row.try_get::<u32, _>("threshold_days")?,
        is_enabled: row.try_get("is_enabled")?,
        previewed_count: row.try_get::<Option<u64>, _>("previewed_count")?,
        previewed_at: optional_instant(row, "previewed_at")?,
        last_run_at: optional_instant(row, "last_run_at")?,
        created_by: user_reference(row, "created_by", "created_by_name")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

fn run_from_row(row: &MySqlRow) -> Result<RetentionRun, AppError> {
    let status: String = row.try_get("status")?;
    let trigger: String = row.try_get("trigger_kind")?;
    Ok(RetentionRun {
        id: uuid_column(row, "id")?,
        policy_id: optional_uuid(row, "policy_id")?,
        policy_name: row.try_get("policy_name")?,
        action: action_column(row)?,
        record_type: row.try_get("record_type")?,
        threshold_days: row.try_get::<u32, _>("threshold_days")?,
        trigger: RetentionTrigger::parse(&trigger),
        status: RetentionRunStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        affected: row.try_get::<u64, _>("affected")?,
        files_removed: row.try_get::<u64, _>("files_removed")?,
        errors: row.try_get::<u32, _>("errors")?,
        error_message: row.try_get("error_message")?,
        requested_by: user_reference(row, "requested_by", "requested_by_name")?,
        created_at: instant(row, "created_at")?,
        started_at: optional_instant(row, "started_at")?,
        finished_at: optional_instant(row, "finished_at")?,
    })
}

fn push_candidates(
    builder: &mut QueryBuilder<MySql>,
    action: RetentionAction,
    definition: &EntityDefinition,
    threshold: DateTime<Utc>,
) -> Result<(), AppError> {
    let (Some(deleted), Some(archived)) = (
        definition.deleted_column.as_deref(),
        definition.archived_column.as_deref(),
    ) else {
        return Err(AppError::validation_code(
            "retention_not_supported",
            "this record type cannot be archived",
        ));
    };
    if !is_sql_identifier(deleted) || !is_sql_identifier(archived) {
        return Err(AppError::Internal("invalid record type columns".to_owned()));
    }
    let threshold = mapper::naive(threshold);
    let record_type = definition.record_type.as_str().to_owned();
    builder.push(format!(
        " FROM {} r WHERE r.{deleted} IS NULL",
        definition.table
    ));
    push_partition(builder, definition, "r");
    match action {
        RetentionAction::ArchiveInactive => {
            builder
                .push(format!(" AND r.{archived} IS NULL AND r.updated_at < "))
                .push_bind(threshold);
            if tracks_last_activity(definition) {
                builder
                    .push(" AND COALESCE(r.last_activity_at, r.updated_at) < ")
                    .push_bind(threshold);
            }
            builder
                .push(
                    " AND NOT EXISTS (SELECT 1 FROM activities a WHERE a.record_type = ",
                )
                .push_bind(record_type.clone())
                .push(" AND a.record_id = r.id AND a.updated_at >= ")
                .push_bind(threshold)
                .push(
                    ") AND NOT EXISTS (SELECT 1 FROM activity_links l JOIN activities a ON a.id = l.activity_id \
                     WHERE l.record_type = ",
                )
                .push_bind(record_type.clone())
                .push(" AND l.record_id = r.id AND a.updated_at >= ")
                .push_bind(threshold)
                .push(
                    ") AND NOT EXISTS (SELECT 1 FROM notes n WHERE n.record_type = ",
                )
                .push_bind(record_type)
                .push(" AND n.record_id = r.id AND n.updated_at >= ")
                .push_bind(threshold)
                .push(")");
        }
        RetentionAction::PurgeArchived => {
            builder
                .push(format!(" AND r.{archived} IS NOT NULL AND r.{archived} < "))
                .push_bind(threshold);
        }
        _ => {
            return Err(AppError::Internal(
                "record candidates requested for a log action".to_owned(),
            ));
        }
    }
    Ok(())
}

fn log_table(action: RetentionAction) -> Option<(&'static str, &'static str)> {
    match action {
        RetentionAction::PurgeAuditLogs => Some(("audit_logs", "")),
        RetentionAction::PurgeSecurityEvents => Some(("security_events", "")),
        RetentionAction::PurgeDomainEvents => {
            Some(("domain_events", " AND dispatched_at IS NOT NULL"))
        }
        _ => None,
    }
}

#[derive(Clone)]
pub(crate) struct RetentionStore {
    pool: MySqlPool,
}

impl RetentionStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list_policies(&self) -> Result<Vec<RetentionPolicy>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{POLICY_SELECT} ORDER BY p.created_at, p.id"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(policy_from_row).collect()
    }

    pub(crate) async fn count_policies(&self) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM retention_policies")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    pub(crate) async fn find_policy(
        &self,
        policy_id: Uuid,
    ) -> Result<Option<RetentionPolicy>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{POLICY_SELECT} WHERE p.id = ?"
        )))
        .bind(policy_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(policy_from_row).transpose()
    }

    pub(crate) async fn insert_policy(
        &self,
        tx: &mut MySqlTx,
        policy: &RetentionPolicy,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO retention_policies (id, name, action, record_type, threshold_days, \
             is_enabled, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(policy.id.to_string())
        .bind(&policy.name)
        .bind(policy.action.as_str())
        .bind(&policy.record_type)
        .bind(policy.threshold_days)
        .bind(policy.is_enabled)
        .bind(
            policy
                .created_by
                .as_ref()
                .map(|user| user.id.as_str().to_owned()),
        )
        .bind(mapper::naive(policy.created_at))
        .bind(mapper::naive(policy.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_policy(
        &self,
        tx: &mut MySqlTx,
        policy: &RetentionPolicy,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE retention_policies SET name = ?, threshold_days = ?, is_enabled = ?, previewed_count = ?, \
             previewed_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&policy.name)
        .bind(policy.threshold_days)
        .bind(policy.is_enabled)
        .bind(policy.previewed_count)
        .bind(policy.previewed_at.map(mapper::naive))
        .bind(mapper::naive(policy.updated_at))
        .bind(policy.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_preview(
        &self,
        policy_id: Uuid,
        count: u64,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE retention_policies SET previewed_count = ?, previewed_at = ? WHERE id = ?",
        )
        .bind(count)
        .bind(mapper::naive(at))
        .bind(policy_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_policy_run(
        &self,
        tx: &mut MySqlTx,
        policy_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE retention_policies SET last_run_at = ? WHERE id = ?")
            .bind(mapper::naive(at))
            .bind(policy_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_policy(
        &self,
        tx: &mut MySqlTx,
        policy_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM retention_policies WHERE id = ?")
            .bind(policy_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn insert_run(
        &self,
        tx: &mut MySqlTx,
        run: &RetentionRun,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO retention_runs (id, policy_id, policy_name, action, record_type, \
             threshold_days, trigger_kind, status, requested_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run.id.to_string())
        .bind(run.policy_id.map(|id| id.to_string()))
        .bind(&run.policy_name)
        .bind(run.action.as_str())
        .bind(&run.record_type)
        .bind(run.threshold_days)
        .bind(run.trigger.as_str())
        .bind(run.status.as_str())
        .bind(run.requested_by.as_ref().map(|user| user.id.as_str().to_owned()))
        .bind(mapper::naive(run.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_run(
        &self,
        tx: &mut MySqlTx,
        run: &RetentionRun,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE retention_runs SET status = ?, affected = ?, files_removed = ?, errors = ?, error_message = ?, \
             started_at = ?, finished_at = ?, heartbeat_at = UTC_TIMESTAMP(6) WHERE id = ?",
        )
        .bind(run.status.as_str())
        .bind(run.affected)
        .bind(run.files_removed)
        .bind(run.errors)
        .bind(&run.error_message)
        .bind(run.started_at.map(mapper::naive))
        .bind(run.finished_at.map(mapper::naive))
        .bind(run.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn stale_runs(
        &self,
        stale_before: DateTime<Utc>,
    ) -> Result<Vec<RetentionRun>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{RUN_SELECT} WHERE r.status IN ('queued', 'running') \
             AND COALESCE(r.heartbeat_at, r.started_at, r.created_at) < ? ORDER BY r.created_at"
        )))
        .bind(mapper::naive(stale_before))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(run_from_row).collect()
    }

    pub(crate) async fn has_unfinished_run(&self, policy_id: Uuid) -> Result<bool, AppError> {
        let found: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM retention_runs WHERE policy_id = ? AND status IN ('queued', 'running') LIMIT 1",
        )
        .bind(policy_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    pub(crate) async fn touch_run(&self, run_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE retention_runs SET heartbeat_at = UTC_TIMESTAMP(6) WHERE id = ?")
            .bind(run_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn find_run(&self, run_id: Uuid) -> Result<Option<RetentionRun>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("{RUN_SELECT} WHERE r.id = ?")))
            .bind(run_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(run_from_row).transpose()
    }

    pub(crate) async fn list_runs(
        &self,
        filter: &RetentionRunFilter,
        window: ListWindow,
    ) -> Result<Listing<RetentionRun>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<MySql>| {
            builder.push(" WHERE TRUE");
            if let Some(policy_id) = filter.policy_id {
                builder
                    .push(" AND r.policy_id = ")
                    .push_bind(policy_id.to_string());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM retention_runs r");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::new(RUN_SELECT);
        push_filter(&mut builder);
        builder
            .push(" ORDER BY r.created_at DESC, r.id DESC LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(run_from_row).collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn count_candidates(
        &self,
        action: RetentionAction,
        definition: Option<&EntityDefinition>,
        threshold: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let count: i64 = match action {
            RetentionAction::ArchiveInactive | RetentionAction::PurgeArchived => {
                let definition = definition.ok_or_else(|| {
                    AppError::Internal("record type required for this action".to_owned())
                })?;
                let mut builder = QueryBuilder::new("SELECT COUNT(*)");
                push_candidates(&mut builder, action, definition, threshold)?;
                builder.build_query_scalar().fetch_one(&self.pool).await?
            }
            RetentionAction::PurgeOrphanFiles => {
                let mut builder = QueryBuilder::new("SELECT COUNT(*) FROM files f WHERE TRUE");
                builder
                    .push(" AND f.created_at < ")
                    .push_bind(mapper::naive(threshold))
                    .push(format!(
                        " AND {}",
                        unreferenced_file_predicate(&file_reference_columns(&self.pool).await?)
                    ));
                builder.build_query_scalar().fetch_one(&self.pool).await?
            }
            _ => {
                let Some((table, extra)) = log_table(action) else {
                    return Ok(0);
                };
                let mut builder =
                    QueryBuilder::new(format!("SELECT COUNT(*) FROM {table} WHERE TRUE"));
                builder
                    .push(" AND occurred_at < ")
                    .push_bind(mapper::naive(threshold))
                    .push(extra);
                builder.build_query_scalar().fetch_one(&self.pool).await?
            }
        };
        Ok(count.max(0) as u64)
    }

    pub(crate) async fn record_candidates(
        &self,
        action: RetentionAction,
        definition: &EntityDefinition,
        threshold: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let (title, _) = record_title_sql(definition);
        let mut builder = QueryBuilder::new(format!("SELECT r.id, {title} AS title"));
        push_candidates(&mut builder, action, definition, threshold)?;
        builder
            .push(" ORDER BY r.updated_at, r.id LIMIT ")
            .push_bind(i64::from(limit));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok((
                    uuid_column(row, "id")?,
                    row.try_get::<Option<String>, _>("title")?
                        .unwrap_or_default(),
                ))
            })
            .collect()
    }

    pub(crate) async fn archive_record(
        &self,
        tx: &mut MySqlTx,
        definition: &EntityDefinition,
        record_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let (Some(deleted), Some(archived)) = (
            definition.deleted_column.as_deref(),
            definition.archived_column.as_deref(),
        ) else {
            return Ok(false);
        };
        if !is_sql_identifier(deleted) || !is_sql_identifier(archived) {
            return Ok(false);
        }
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {} SET {archived} = ? WHERE id = ? AND {archived} IS NULL AND {deleted} IS NULL",
            definition.table
        )))
        .bind(mapper::naive(now))
        .bind(record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn purge_log_batch(
        &self,
        action: RetentionAction,
        threshold: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, AppError> {
        let Some((table, extra)) = log_table(action) else {
            return Ok(0);
        };
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE occurred_at < ?{extra} ORDER BY occurred_at LIMIT ?"
        )))
        .bind(mapper::naive(threshold))
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn orphan_files(
        &self,
        threshold: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<StoredFileRef>, AppError> {
        let mut builder = QueryBuilder::new("SELECT f.id, f.storage_key FROM files f WHERE TRUE");
        builder
            .push(" AND f.created_at < ")
            .push_bind(mapper::naive(threshold))
            .push(format!(
                " AND {}",
                unreferenced_file_predicate(&file_reference_columns(&self.pool).await?)
            ))
            .push(" ORDER BY f.created_at LIMIT ")
            .push_bind(i64::from(limit));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(StoredFileRef {
                    file_id: uuid_column(row, "id")?,
                    storage_key: row.try_get("storage_key")?,
                })
            })
            .collect()
    }
}
