use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    RetentionAction, RetentionPolicy, RetentionRun, RetentionRunFilter, RetentionRunStatus,
    RetentionTrigger, StoredFileRef,
};
use crate::mysql::sql::{
    file_reference_columns, instant, optional_instant, optional_uuid, unreferenced_file_predicate,
    user_reference, uuid_column,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::EntityDefinition;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

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

async fn candidates(
    ctx: &Ctx<'_>,
    action: RetentionAction,
    definition: &EntityDefinition,
    threshold: DateTime<Utc>,
    projection: aphel_sdk::platform::infrastructure::query::compiler::Projection,
) -> Result<QueryBuilder<MySql>, AppError> {
    use aphel_kernel::{
        application::query::{
            ArchivedFilter, CompareOp, FieldTarget, PlanCondition, PlanNode, PlanValue, Predicate,
            QueryPlan,
        },
        domain::{
            access::{AccessScope, RecordScopeFilter},
            fields::FieldType,
        },
    };
    use aphel_sdk::privacy::PrivacyTraits;
    if !crate::domain::supports_retention(definition) {
        return Err(AppError::validation_code(
            "retention_not_supported",
            "this record type cannot be archived",
        ));
    }
    let condition = |column: &str, predicate: Predicate, negated: bool| {
        PlanNode::Condition(PlanCondition {
            target: FieldTarget::System {
                column: column.to_owned(),
            },
            field_type: FieldType::DateTime,
            predicate,
            negated,
        })
    };
    let mut filters = Vec::new();
    match action {
        RetentionAction::ArchiveInactive => {
            filters.push(condition(
                "updated_at",
                Predicate::Compare(CompareOp::Lt, PlanValue::DateTime(threshold)),
                false,
            ));
            for key in definition
                .capabilities
                .retention_mapping_data()?
                .recency_fields
            {
                let field = definition.system_field_by_key(&key).ok_or_else(|| {
                    AppError::Configuration("unknown retention recency field".to_owned())
                })?;
                filters.push(PlanNode::Or(vec![
                    condition(&field.column, Predicate::Present, true),
                    condition(
                        &field.column,
                        Predicate::Compare(CompareOp::Lt, PlanValue::DateTime(threshold)),
                        false,
                    ),
                ]));
            }
        }
        RetentionAction::PurgeArchived => filters.push(condition(
            definition
                .archived_column
                .as_deref()
                .ok_or_else(|| AppError::Configuration("missing archive column".to_owned()))?,
            Predicate::Compare(CompareOp::Lt, PlanValue::DateTime(threshold)),
            false,
        )),
        _ => {
            return Err(AppError::Internal(
                "record candidates requested for a non-record action".to_owned(),
            ));
        }
    }
    let plan = QueryPlan {
        record_type: definition.clone(),
        visibility: RecordScopeFilter::Organization,
        filter: Some(PlanNode::And(filters)),
        search: None,
        sort: Vec::new(),
        archived: if action == RetentionAction::PurgeArchived {
            ArchivedFilter::Only
        } else {
            ArchivedFilter::Exclude
        },
        window: ListWindow {
            offset: 0,
            limit: 1,
        },
    };
    let mut builder = aphel_sdk::platform::infrastructure::query::compiler::build_query_unwindowed(
        &AccessScope::system(),
        &plan,
        projection,
    )?;
    if action == RetentionAction::ArchiveInactive {
        for source in crate::service::sources::available(ctx).await? {
            source
                .restrict_inactive(ctx, &mut builder, &definition.record_type, threshold)
                .await?;
        }
    }
    Ok(builder)
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
        ctx: &Ctx<'_>,
        action: RetentionAction,
        definition: Option<&EntityDefinition>,
        threshold: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let count: i64 = match action {
            RetentionAction::ArchiveInactive | RetentionAction::PurgeArchived => {
                let definition = definition.ok_or_else(|| {
                    AppError::Internal("record type required for this action".to_owned())
                })?;
                let mut builder = candidates(
                    ctx,
                    action,
                    definition,
                    threshold,
                    aphel_sdk::platform::infrastructure::query::compiler::Projection::Count,
                )
                .await?;
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
        ctx: &Ctx<'_>,
        action: RetentionAction,
        definition: &EntityDefinition,
        threshold: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let mut builder = candidates(
            ctx,
            action,
            definition,
            threshold,
            aphel_sdk::platform::infrastructure::query::compiler::Projection::Ids,
        )
        .await?;
        builder
            .push(" ORDER BY r.updated_at, r.id LIMIT ")
            .push_bind(limit);
        let ids: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        let mut result = Vec::new();
        for id in ids {
            let id = Uuid::parse_str(&id)
                .map_err(|_| AppError::Internal("invalid stored record id".to_owned()))?;
            let record = aphel_sdk::platform::application::shared::RecordRef::new(
                definition.record_type.clone(),
                id,
            );
            let snapshot =
                aphel_sdk::records::inspect_entity(ctx.pool(), ctx.registry(), &record).await?;
            let title = snapshot
                .as_ref()
                .and_then(|snapshot| {
                    definition.title_field.as_deref().and_then(|key| {
                        snapshot
                            .get("values")
                            .and_then(|values| values.get(key))
                            .or_else(|| {
                                snapshot
                                    .get("custom_fields")
                                    .and_then(|values| values.get(key))
                            })
                    })
                })
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            result.push((id, title));
        }
        Ok(result)
    }

    pub(crate) async fn archive_record(
        &self,
        tx: &mut MySqlTx,
        definition: &EntityDefinition,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        aphel_sdk::records::archive_entity(tx, definition, id, now).await
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
