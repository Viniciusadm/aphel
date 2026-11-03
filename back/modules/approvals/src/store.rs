use aphel_contract_crm::CrmPermissions;
use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{Executor, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    ApprovalDecision, ApprovalMoment, ApprovalPolicy, ApprovalRequest, ApprovalRequestRecord,
    ApprovalRule, ApprovalStatus, ApproverKind, ConditionKind, DecisionEntry, PolicyCondition,
    PolicyListFilter, PolicyStep, RecordState, RequestApprover, RequestListFilter, StepSnapshot,
};
use ::formulas::application::{FormulaEngine, TypeFormulas, ValueKind, compile_formula};
use ::formulas::infrastructure::MySqlFormulaPorts;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{EntityDefinition, FieldSet, RecordTypeRegistry};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterExpansion, FilterOperator, ID_FIELD, RecordQuery,
    RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts, ResolveContext, load_related_sets,
    resolve_query_related, validate_view_query_related,
};
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, PermissionSet, RecordScopeFilter, RoleKey};
use aphel_kernel::domain::fields::{
    FieldConfig, FieldDefinition, FieldKey, FieldValue, FormulaResultType, FormulaSettings,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::infrastructure::query::compiler::{
    Projection, build_query, push_record_scope, quoted,
};
use aphel_kernel::infrastructure::query::mysql_executor::MySqlRecordQueryPorts;
use aphel_kernel::infrastructure::repositories::mapper::{invalid_column, naive, utc};

fn parse_uuid(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

const POLICY_COLUMNS: &str = "SELECT id, `key`, name, description, record_type, moment, target_id, \
     condition_kind, condition_filter, condition_formula, locked_fields, due_hours, is_active, \
     position, created_by, created_at, updated_at FROM approval_policies WHERE TRUE";

const REQUEST_COLUMNS: &str = "SELECT r.id, r.policy_id, p.`key` AS policy_key, r.policy_name, \
     r.record_type, r.record_id, r.moment, r.target_id, r.status, r.current_step, r.steps, \
     r.locked_fields, r.comment, r.requested_by, r.requested_at, r.due_at, r.resolved_at, \
     r.resolved_by, r.resolution_comment, r.consumed_at, r.updated_at FROM approval_requests r \
     LEFT JOIN approval_policies p ON p.id = r.policy_id WHERE TRUE";

const AWAITING_CONDITION: &str = " AND r.status = 'pending' AND EXISTS (SELECT 1 FROM \
     approval_request_approvers a WHERE a.request_id = r.id AND a.step_position = r.current_step \
     AND a.decision IS NULL AND a.user_id = ";

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|value| parse_uuid(&value, column))
        .transpose()
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|value| UserId::new(value).map_err(AppError::from))
        .transpose()
}

fn optional_time(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    Ok(row.try_get::<Option<NaiveDateTime>, _>(column)?.map(utc))
}

fn strings(value: Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn policy_from_row(row: &MySqlRow) -> Result<ApprovalPolicy, AppError> {
    let moment: String = row.try_get("moment")?;
    let kind: String = row.try_get("condition_kind")?;
    let filter: Option<Json<Value>> = row.try_get("condition_filter")?;
    let formula: Option<String> = row.try_get("condition_formula")?;
    let condition = match ConditionKind::parse(&kind) {
        Some(ConditionKind::Always) => PolicyCondition::Always,
        Some(ConditionKind::Filter) => {
            let filter = filter
                .and_then(|Json(value)| serde_json::from_value::<Filter>(value).ok())
                .ok_or_else(|| invalid_column("approval_policies.condition_filter", &kind))?;
            PolicyCondition::Filter(filter)
        }
        Some(ConditionKind::Formula) => PolicyCondition::Formula(formula.unwrap_or_default()),
        None => return Err(invalid_column("approval_policies.condition_kind", &kind)),
    };
    let Json(locked): Json<Value> = row.try_get("locked_fields")?;
    Ok(ApprovalPolicy {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "approval_policies.id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        record_type: row.try_get("record_type")?,
        moment: ApprovalMoment::parse(&moment)
            .ok_or_else(|| invalid_column("approval_policies.moment", &moment))?,
        target_id: optional_uuid(row, "target_id")?,
        condition,
        locked_fields: strings(locked),
        due_hours: row.try_get("due_hours")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        steps: Vec::new(),
        created_by: optional_user(row, "created_by")?,
        created_at: utc(row.try_get("created_at")?),
        updated_at: utc(row.try_get("updated_at")?),
    })
}

fn step_from_row(row: &MySqlRow) -> Result<(Uuid, PolicyStep), AppError> {
    let kind: String = row.try_get("approver_kind")?;
    let rule: String = row.try_get("rule")?;
    Ok((
        parse_uuid(
            &row.try_get::<String, _>("policy_id")?,
            "approval_policy_steps.policy_id",
        )?,
        PolicyStep {
            id: parse_uuid(&row.try_get::<String, _>("id")?, "approval_policy_steps.id")?,
            position: row.try_get("position")?,
            name: row.try_get("name")?,
            approver_kind: ApproverKind::parse(&kind)
                .ok_or_else(|| invalid_column("approval_policy_steps.approver_kind", &kind))?,
            approver_user_id: optional_user(row, "approver_user_id")?,
            approver_role_id: optional_uuid(row, "approver_role_id")?,
            rule: ApprovalRule::parse(&rule).unwrap_or(ApprovalRule::Any),
        },
    ))
}

fn request_from_row(row: &MySqlRow) -> Result<ApprovalRequest, AppError> {
    let moment: String = row.try_get("moment")?;
    let status: String = row.try_get("status")?;
    let record_type: String = row.try_get("record_type")?;
    let Json(steps): Json<Value> = row.try_get("steps")?;
    let Json(locked): Json<Value> = row.try_get("locked_fields")?;
    Ok(ApprovalRequest {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "approval_requests.id")?,
        policy_id: optional_uuid(row, "policy_id")?,
        policy_key: row.try_get("policy_key")?,
        policy_name: row.try_get("policy_name")?,
        record: RecordRef::new(
            RecordType::parse(record_type)?,
            parse_uuid(
                &row.try_get::<String, _>("record_id")?,
                "approval_requests.record_id",
            )?,
        ),
        moment: ApprovalMoment::parse(&moment)
            .ok_or_else(|| invalid_column("approval_requests.moment", &moment))?,
        target_id: optional_uuid(row, "target_id")?,
        status: ApprovalStatus::parse(&status)
            .ok_or_else(|| invalid_column("approval_requests.status", &status))?,
        current_step: row.try_get("current_step")?,
        steps: serde_json::from_value::<Vec<StepSnapshot>>(steps).unwrap_or_default(),
        locked_fields: strings(locked),
        comment: row.try_get("comment")?,
        requested_by: optional_user(row, "requested_by")?,
        requested_at: utc(row.try_get("requested_at")?),
        due_at: optional_time(row, "due_at")?,
        resolved_at: optional_time(row, "resolved_at")?,
        resolved_by: optional_user(row, "resolved_by")?,
        resolution_comment: row.try_get("resolution_comment")?,
        consumed_at: optional_time(row, "consumed_at")?,
        updated_at: utc(row.try_get("updated_at")?),
    })
}

fn approver_from_row(row: &MySqlRow) -> Result<(Uuid, RequestApprover), AppError> {
    let decision: Option<String> = row.try_get("decision")?;
    Ok((
        parse_uuid(
            &row.try_get::<String, _>("request_id")?,
            "approval_request_approvers.request_id",
        )?,
        RequestApprover {
            step_position: row.try_get("step_position")?,
            user_id: UserId::new(row.try_get::<String, _>("user_id")?)?,
            decision: decision.as_deref().and_then(ApprovalDecision::parse),
            comment: row.try_get("comment")?,
            decided_at: optional_time(row, "decided_at")?,
        },
    ))
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    separated.push_unseparated(")");
}

async fn load_steps<'e, E>(executor: E, policies: &mut [ApprovalPolicy]) -> Result<(), AppError>
where
    E: Executor<'e, Database = MySql>,
{
    if policies.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = policies.iter().map(|policy| policy.id).collect();
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT id, policy_id, position, name, approver_kind, approver_user_id, approver_role_id, \
         rule FROM approval_policy_steps WHERE policy_id IN (",
    );
    push_ids(&mut builder, &ids);
    builder.push(" ORDER BY position");
    let rows = builder.build().fetch_all(executor).await?;
    let mut by_policy: HashMap<Uuid, Vec<PolicyStep>> = HashMap::new();
    for row in &rows {
        let (policy_id, step) = step_from_row(row)?;
        by_policy.entry(policy_id).or_default().push(step);
    }
    for policy in policies {
        policy.steps = by_policy.remove(&policy.id).unwrap_or_default();
    }
    Ok(())
}

async fn load_approvers<'e, E>(
    executor: E,
    requests: Vec<ApprovalRequest>,
) -> Result<Vec<ApprovalRequestRecord>, AppError>
where
    E: Executor<'e, Database = MySql>,
{
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = requests.iter().map(|request| request.id).collect();
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT request_id, step_position, user_id, decision, comment, decided_at \
         FROM approval_request_approvers WHERE request_id IN (",
    );
    push_ids(&mut builder, &ids);
    builder.push(" ORDER BY step_position, decided_at IS NULL, decided_at, user_id");
    let rows = builder.build().fetch_all(executor).await?;
    let mut by_request: HashMap<Uuid, Vec<RequestApprover>> = HashMap::new();
    for row in &rows {
        let (request_id, approver) = approver_from_row(row)?;
        by_request.entry(request_id).or_default().push(approver);
    }
    Ok(requests
        .into_iter()
        .map(|request| ApprovalRequestRecord {
            approvers: by_request.remove(&request.id).unwrap_or_default(),
            request,
        })
        .collect())
}

fn push_request_filter(builder: &mut QueryBuilder<MySql>, filter: &RequestListFilter) {
    if let Some(record) = &filter.record {
        builder
            .push(" AND r.record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(record.record_id.to_string());
    }
    if let Some(user) = &filter.requested_by {
        builder
            .push(" AND r.requested_by = ")
            .push_bind(user.as_str().to_owned());
    }
    if let Some(user) = &filter.awaiting {
        builder
            .push(AWAITING_CONDITION)
            .push_bind(user.as_str().to_owned())
            .push(")");
    }
    if let Some(status) = filter.status {
        builder.push(" AND r.status = ").push_bind(status.as_str());
    }
}

#[derive(Clone)]
pub(crate) struct ApprovalStore {
    pool: MySqlPool,
}

impl ApprovalStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert_steps(
        &self,
        tx: &mut MySqlTx,
        policy: &ApprovalPolicy,
    ) -> Result<(), AppError> {
        for step in &policy.steps {
            sqlx::query(
                "INSERT INTO approval_policy_steps (id, policy_id, position, name, \
                 approver_kind, approver_user_id, approver_role_id, rule) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(step.id.to_string())
            .bind(policy.id.to_string())
            .bind(step.position)
            .bind(&step.name)
            .bind(step.approver_kind.as_str())
            .bind(step.approver_user_id.as_ref().map(UserId::as_str))
            .bind(step.approver_role_id.map(|id| id.to_string()))
            .bind(step.rule.as_str())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn request_records(
        &self,
        builder: &mut QueryBuilder<MySql>,
    ) -> Result<Vec<ApprovalRequestRecord>, AppError> {
        let rows = builder.build().fetch_all(&self.pool).await?;
        let requests = rows
            .iter()
            .map(request_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        load_approvers(&self.pool, requests).await
    }
}

fn condition_columns(condition: &PolicyCondition) -> (Option<Json<Value>>, Option<&str>) {
    match condition {
        PolicyCondition::Always => (None, None),
        PolicyCondition::Filter(filter) => (serde_json::to_value(filter).ok().map(Json), None),
        PolicyCondition::Formula(formula) => (None, Some(formula.as_str())),
    }
}

impl ApprovalStore {
    pub(crate) async fn policies(
        &self,
        filter: &PolicyListFilter,
    ) -> Result<Vec<ApprovalPolicy>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(POLICY_COLUMNS);
        if let Some(record_type) = &filter.record_type {
            builder
                .push(" AND record_type = ")
                .push_bind(record_type.clone());
        }
        if let Some(active) = filter.active {
            builder.push(" AND is_active = ").push_bind(active);
        }
        builder.push(" ORDER BY position, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut policies = rows
            .iter()
            .map(policy_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        load_steps(&self.pool, &mut policies).await?;
        Ok(policies)
    }

    pub(crate) async fn policy(&self, id: Uuid) -> Result<Option<ApprovalPolicy>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(POLICY_COLUMNS);
        builder.push(" AND id = ").push_bind(id.to_string());
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let mut policies = vec![policy_from_row(&row)?];
        load_steps(&self.pool, &mut policies).await?;
        Ok(policies.pop())
    }

    pub(crate) async fn policy_by_key(
        &self,
        key: &str,
    ) -> Result<Option<ApprovalPolicy>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(POLICY_COLUMNS);
        builder.push(" AND `key` = ").push_bind(key.to_owned());
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let mut policies = vec![policy_from_row(&row)?];
        load_steps(&self.pool, &mut policies).await?;
        Ok(policies.pop())
    }

    pub(crate) async fn active_policies(
        &self,
        tx: &mut MySqlTx,
        record_type: &str,
        moment: ApprovalMoment,
        target_id: Option<Uuid>,
    ) -> Result<Vec<ApprovalPolicy>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(POLICY_COLUMNS);
        builder
            .push(" AND is_active = TRUE AND record_type = ")
            .push_bind(record_type.to_owned())
            .push(" AND moment = ")
            .push_bind(moment.as_str());
        match target_id {
            Some(target) => {
                builder
                    .push(" AND target_id = ")
                    .push_bind(target.to_string());
            }
            None => {
                builder.push(" AND target_id IS NULL");
            }
        }
        builder.push(" ORDER BY position, name, id");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let mut policies = rows
            .iter()
            .map(policy_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        load_steps(&mut **tx, &mut policies).await?;
        Ok(policies)
    }

    pub(crate) async fn next_position(&self) -> Result<i32, AppError> {
        let position: Option<i32> =
            sqlx::query_scalar("SELECT MAX(position) FROM approval_policies")
                .fetch_one(&self.pool)
                .await?;
        Ok(position.map_or(0, |position| position.saturating_add(1)))
    }

    pub(crate) async fn insert_policy(
        &self,
        tx: &mut MySqlTx,
        policy: &ApprovalPolicy,
    ) -> Result<(), AppError> {
        let (filter, formula) = condition_columns(&policy.condition);
        sqlx::query(
            "INSERT INTO approval_policies (id, `key`, name, description, \
             record_type, moment, target_id, condition_kind, condition_filter, condition_formula, \
             locked_fields, due_hours, is_active, position, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(policy.id.to_string())
        .bind(&policy.key)
        .bind(&policy.name)
        .bind(&policy.description)
        .bind(&policy.record_type)
        .bind(policy.moment.as_str())
        .bind(policy.target_id.map(|id| id.to_string()))
        .bind(policy.condition.kind().as_str())
        .bind(filter)
        .bind(formula)
        .bind(Json(&policy.locked_fields))
        .bind(policy.due_hours)
        .bind(policy.is_active)
        .bind(policy.position)
        .bind(policy.created_by.as_ref().map(UserId::as_str))
        .bind(naive(policy.created_at))
        .bind(naive(policy.updated_at))
        .execute(&mut **tx)
        .await?;
        self.insert_steps(tx, policy).await
    }

    pub(crate) async fn update_policy(
        &self,
        tx: &mut MySqlTx,
        policy: &ApprovalPolicy,
    ) -> Result<(), AppError> {
        let (filter, formula) = condition_columns(&policy.condition);
        sqlx::query(
            "UPDATE approval_policies SET name = ?, description = ?, record_type = ?, moment = ?, \
             target_id = ?, condition_kind = ?, condition_filter = ?, condition_formula = ?, \
             locked_fields = ?, due_hours = ?, is_active = ?, position = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&policy.name)
        .bind(&policy.description)
        .bind(&policy.record_type)
        .bind(policy.moment.as_str())
        .bind(policy.target_id.map(|id| id.to_string()))
        .bind(policy.condition.kind().as_str())
        .bind(filter)
        .bind(formula)
        .bind(Json(&policy.locked_fields))
        .bind(policy.due_hours)
        .bind(policy.is_active)
        .bind(policy.position)
        .bind(naive(policy.updated_at))
        .bind(policy.id.to_string())
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM approval_policy_steps WHERE policy_id = ?")
            .bind(policy.id.to_string())
            .execute(&mut **tx)
            .await?;
        self.insert_steps(tx, policy).await
    }

    pub(crate) async fn delete_policy(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM approval_policies WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn set_positions(
        &self,
        tx: &mut MySqlTx,
        positions: &[(Uuid, i32)],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (id, position) in positions {
            sqlx::query(
                "UPDATE approval_policies SET position = ?, updated_at = ? \
                 WHERE id = ? AND position <> ?",
            )
            .bind(position)
            .bind(naive(at))
            .bind(id.to_string())
            .bind(position)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn open_requests(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        moment: ApprovalMoment,
        target_id: Option<Uuid>,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(" AND r.record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(record.record_id.to_string())
            .push(" AND r.moment = ")
            .push_bind(moment.as_str())
            .push(" AND r.target_marker = ")
            .push_bind(target_id.map(|id| id.to_string()).unwrap_or_default())
            .push(
                " AND (r.status = 'pending' OR (r.status = 'approved' AND r.consumed_at IS NULL)) \
                 ORDER BY r.requested_at, r.id FOR UPDATE",
            );
        let rows = builder.build().fetch_all(&mut **tx).await?;
        rows.iter().map(request_from_row).collect()
    }

    pub(crate) async fn pending_for_record(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(" AND r.record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(record.record_id.to_string())
            .push(" AND r.status = 'pending' ORDER BY r.requested_at, r.id FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        rows.iter().map(request_from_row).collect()
    }

    pub(crate) async fn pending_for_policy(
        &self,
        tx: &mut MySqlTx,
        policy_id: Uuid,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(" AND r.policy_id = ")
            .push_bind(policy_id.to_string())
            .push(" AND r.status = 'pending' ORDER BY r.requested_at, r.id FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        rows.iter().map(request_from_row).collect()
    }

    pub(crate) async fn insert_request(
        &self,
        tx: &mut MySqlTx,
        request: &ApprovalRequest,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO approval_requests (id, policy_id, policy_name, \
             record_type, record_id, moment, target_id, status, current_step, steps, locked_fields, \
             comment, requested_by, requested_at, due_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(request.id.to_string())
        .bind(request.policy_id.map(|id| id.to_string()))
        .bind(&request.policy_name)
        .bind(request.record.record_type.as_str())
        .bind(request.record.record_id.to_string())
        .bind(request.moment.as_str())
        .bind(request.target_id.map(|id| id.to_string()))
        .bind(request.status.as_str())
        .bind(request.current_step)
        .bind(Json(&request.steps))
        .bind(Json(&request.locked_fields))
        .bind(&request.comment)
        .bind(request.requested_by.as_ref().map(UserId::as_str))
        .bind(naive(request.requested_at))
        .bind(request.due_at.map(naive))
        .bind(naive(request.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_approvers(
        &self,
        tx: &mut MySqlTx,
        request_id: Uuid,
        step: u32,
        users: &[UserId],
    ) -> Result<(), AppError> {
        if users.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT IGNORE INTO approval_request_approvers (request_id, step_position, user_id) ",
        );
        builder.push_values(users, |mut row, user| {
            row.push_bind(request_id.to_string())
                .push_bind(step)
                .push_bind(user.as_str().to_owned());
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn lock_request(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
    ) -> Result<Option<ApprovalRequestRecord>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(" AND r.id = ")
            .push_bind(id.to_string())
            .push(" FOR UPDATE");
        let Some(row) = builder.build().fetch_optional(&mut **tx).await? else {
            return Ok(None);
        };
        let request = request_from_row(&row)?;
        Ok(load_approvers(&mut **tx, vec![request]).await?.pop())
    }

    pub(crate) async fn request(
        &self,
        id: Uuid,
    ) -> Result<Option<ApprovalRequestRecord>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder.push(" AND r.id = ").push_bind(id.to_string());
        Ok(self.request_records(&mut builder).await?.pop())
    }

    pub(crate) async fn record_decision(
        &self,
        tx: &mut MySqlTx,
        decision: &DecisionEntry<'_>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE approval_request_approvers SET decision = ?, comment = ?, decided_at = ? \
             WHERE request_id = ? AND step_position = ? AND user_id = ? \
             AND decision IS NULL",
        )
        .bind(decision.decision.as_str())
        .bind(decision.comment)
        .bind(naive(decision.at))
        .bind(decision.request_id.to_string())
        .bind(decision.step)
        .bind(decision.user_id.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn save_request_state(
        &self,
        tx: &mut MySqlTx,
        request: &ApprovalRequest,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE approval_requests SET status = ?, current_step = ?, resolved_at = ?, \
             resolved_by = ?, resolution_comment = ?, consumed_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(request.status.as_str())
        .bind(request.current_step)
        .bind(request.resolved_at.map(naive))
        .bind(request.resolved_by.as_ref().map(UserId::as_str))
        .bind(&request.resolution_comment)
        .bind(request.consumed_at.map(naive))
        .bind(naive(request.updated_at))
        .bind(request.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_consumed(
        &self,
        tx: &mut MySqlTx,
        ids: &[Uuid],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new("UPDATE approval_requests SET consumed_at = ");
        builder
            .push_bind(naive(at))
            .push(", updated_at = ")
            .push_bind(naive(at))
            .push(" WHERE consumed_at IS NULL AND id IN (");
        push_ids(&mut builder, ids);
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn requests(
        &self,
        filter: &RequestListFilter,
        window: ListWindow,
    ) -> Result<Listing<ApprovalRequestRecord>, AppError> {
        let mut count =
            QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM approval_requests r WHERE TRUE");
        push_request_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        push_request_filter(&mut builder, filter);
        builder
            .push(" ORDER BY r.requested_at DESC, r.id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        Ok(Listing {
            items: self.request_records(&mut builder).await?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn requests_for_record(
        &self,
        record: &RecordRef,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(" AND r.record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(record.record_id.to_string())
            .push(" ORDER BY r.requested_at DESC, r.id DESC");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(request_from_row).collect()
    }

    pub(crate) async fn pending_count(&self, user_id: &UserId) -> Result<u64, AppError> {
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM approval_requests r WHERE TRUE");
        builder
            .push(AWAITING_CONDITION)
            .push_bind(user_id.as_str().to_owned())
            .push(")");
        let total: i64 = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(u64::try_from(total).unwrap_or_default())
    }

    pub(crate) async fn overdue(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<ApprovalRequestRecord>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(REQUEST_COLUMNS);
        builder
            .push(
                " AND r.status = 'pending' AND r.due_at IS NOT NULL AND r.overdue_notified_at IS NULL \
                 AND r.due_at < ",
            )
            .push_bind(naive(now))
            .push(" ORDER BY r.due_at, r.id LIMIT ")
            .push_bind(limit);
        self.request_records(&mut builder).await
    }

    pub(crate) async fn mark_overdue(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE approval_requests SET overdue_notified_at = ? \
             WHERE id = ? AND status = 'pending' AND overdue_notified_at IS NULL",
        )
        .bind(naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}

const CONDITION_KEY: &str = "approval_condition";
const CUSTOM_FIELDS_PREFIX: &str = "custom_fields";

pub(crate) fn condition_scope(scope: &AccessScope) -> AccessScope {
    AccessScope::new(
        scope.user_id().clone(),
        RoleKey::system(),
        scope.team_ids().to_vec(),
        PermissionSet::everything(),
    )
    .with_enabled_modules(scope.enabled_modules().to_vec())
}

fn condition_definition(
    record_type: &EntityDefinition,
    expression: &str,
) -> Result<FieldDefinition, AppError> {
    Ok(FieldDefinition {
        id: Uuid::nil(),
        record_type: record_type.record_type.as_str().to_owned(),
        key: FieldKey::new(CONDITION_KEY)?,
        label: CONDITION_KEY.to_owned(),
        field_type: FormulaResultType::Boolean.field_type(),
        is_system: false,
        column_name: None,
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        module_key: None,
        is_active: true,
        position: i32::MAX,
        config: FieldConfig {
            formula: Some(FormulaSettings {
                expression: expression.to_owned(),
                result_type: FormulaResultType::Boolean,
            }),
            ..FieldConfig::default()
        },
        options: Vec::new(),
    })
}

#[derive(Clone)]
pub(crate) struct ApprovalDirectory {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl ApprovalDirectory {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    fn state_query(
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_id: Uuid,
        visibility: Option<&RecordScopeFilter>,
    ) -> QueryBuilder<MySql> {
        let owner = definition.owner_column.as_deref().map_or_else(
            || "NULL".to_owned(),
            |column| format!("r.{}", quoted(column)),
        );
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT {owner} AS owner_id FROM {} r WHERE r.id = ",
            quoted(&definition.table)
        ));
        builder.push_bind(record_id.to_string());
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(" AND r.{} IS NULL", quoted(deleted)));
        }
        push_record_scope(
            &mut builder,
            scope,
            definition,
            visibility.unwrap_or(&RecordScopeFilter::Organization),
        );
        builder
    }

    fn state_from_row(row: &sqlx::mysql::MySqlRow) -> Result<RecordState, AppError> {
        Ok(RecordState {
            owner_id: row
                .try_get::<Option<String>, _>("owner_id")?
                .map(|id| UserId::new(id).map_err(AppError::from))
                .transpose()?,
        })
    }

    pub(crate) async fn filter_query(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        filter: &Filter,
        record_id: Uuid,
    ) -> Result<Result<aphel_kernel::application::query::QueryPlan, AppError>, AppError> {
        let elevated = condition_scope(scope);
        let ports = MySqlRecordQueryPorts::new(self.pool.clone(), self.registry.clone());
        let fields = FieldSet::for_scope(ports.definitions(), definition, &elevated).await?;
        let context = ResolveContext {
            now: ports.clock().now(),
            timezone: ports.executor().timezone().await?,
            user_id: scope.user_id().clone(),
        };
        let engine = RecordQueryEngine::new(&ports);
        let segments = engine
            .expansions(definition.record_type.as_str(), Some(filter))
            .await?;
        let relations = load_related_sets(
            &self.registry,
            ports.definitions(),
            &elevated,
            definition,
            &filter.relation_keys(),
        )
        .await?;
        let query = RecordQuery {
            filter: Some(Filter::and(vec![
                filter.clone(),
                Filter::condition(ID_FIELD, FilterOperator::Eq, json!(record_id.to_string())),
            ])),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
            ..RecordQuery::default()
        };
        Ok(resolve_query_related(
            &fields,
            &context,
            RecordScopeFilter::Organization,
            query,
            FilterExpansion::Expand(&segments),
            &relations,
        ))
    }

    pub(crate) async fn formula_matches(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        expression: &str,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        let elevated = condition_scope(scope);
        let ports = MySqlFormulaPorts::new(self.pool.clone(), self.registry.clone());
        let engine = FormulaEngine::new(&ports, &elevated);
        let draft = condition_definition(definition, expression)?;
        let mut workspace = engine.workspace().with_override(draft.clone());
        let context = workspace.context(&definition.record_type).await?;
        if compile_formula(expression, &context).is_err() {
            return Ok(false);
        }
        let formulas = TypeFormulas::new(context);
        let mut evaluated = engine.evaluate(&formulas, &[record_id]).await?;
        let Some(record) = evaluated.remove(&record_id) else {
            return Ok(false);
        };
        Ok(record
            .results
            .iter()
            .find(|(field, _)| field.key == draft.key)
            .is_some_and(|(_, value)| matches!(value, Some(FieldValue::Bool(true)))))
    }

    pub(crate) async fn users_where(
        &self,
        condition: &str,
        value: String,
    ) -> Result<Vec<UserId>, AppError> {
        let sql = format!(
            "SELECT DISTINCT m.id FROM users m \
             WHERE m.status = 'active' AND {condition} \
             ORDER BY m.id"
        );
        let ids: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(value)
            .fetch_all(&self.pool)
            .await?;
        ids.into_iter()
            .map(|id| UserId::new(id).map_err(AppError::from))
            .collect()
    }

    pub(crate) async fn team_managers(
        &self,
        owner: &UserId,
        parents: bool,
    ) -> Result<Vec<UserId>, AppError> {
        let teams = if parents {
            "SELECT t.parent_team_id FROM team_memberships tm JOIN teams t ON t.id = tm.team_id \
             WHERE tm.user_id = ? AND t.parent_team_id IS NOT NULL"
        } else {
            "SELECT tm.team_id FROM team_memberships tm JOIN teams t ON t.id = tm.team_id \
             WHERE tm.user_id = ?"
        };
        self.users_where(
            &format!(
                "EXISTS (SELECT 1 FROM team_memberships manager WHERE manager.user_id = m.id \
                 AND manager.is_manager = TRUE AND manager.team_id IN ({teams}))"
            ),
            owner.as_str().to_owned(),
        )
        .await
    }
}

impl ApprovalDirectory {
    pub(crate) async fn record_state_in(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_id: Uuid,
        visibility: Option<&RecordScopeFilter>,
    ) -> Result<Option<RecordState>, AppError> {
        let mut builder = Self::state_query(scope, definition, record_id, visibility);
        let row = builder.build().fetch_optional(&mut **tx).await?;
        row.as_ref().map(Self::state_from_row).transpose()
    }

    pub(crate) async fn record_state(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_id: Uuid,
        visibility: Option<&RecordScopeFilter>,
    ) -> Result<Option<RecordState>, AppError> {
        let mut builder = Self::state_query(scope, definition, record_id, visibility);
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref().map(Self::state_from_row).transpose()
    }

    pub(crate) async fn condition_matches(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_id: Uuid,
        condition: &PolicyCondition,
    ) -> Result<bool, AppError> {
        match condition {
            PolicyCondition::Always => Ok(true),
            PolicyCondition::Filter(filter) => {
                let Ok(plan) = self
                    .filter_query(scope, definition, filter, record_id)
                    .await?
                else {
                    return Ok(false);
                };
                let elevated = condition_scope(scope);
                let mut builder = build_query(&elevated, &plan, Projection::Count)?;
                let total: i64 = builder.build_query_scalar().fetch_one(&mut **tx).await?;
                Ok(total > 0)
            }
            PolicyCondition::Formula(expression) => {
                self.formula_matches(scope, definition, expression, record_id)
                    .await
            }
        }
    }

    pub(crate) async fn condition_violations(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        condition: &PolicyCondition,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let elevated = condition_scope(scope);
        match condition {
            PolicyCondition::Always => Ok(Vec::new()),
            PolicyCondition::Filter(filter) => {
                let ports = MySqlRecordQueryPorts::new(self.pool.clone(), self.registry.clone());
                let fields =
                    FieldSet::for_scope(ports.definitions(), definition, &elevated).await?;
                let context = ResolveContext {
                    now: ports.clock().now(),
                    timezone: ports.executor().timezone().await?,
                    user_id: scope.user_id().clone(),
                };
                let relations = load_related_sets(
                    &self.registry,
                    ports.definitions(),
                    &elevated,
                    definition,
                    &filter.relation_keys(),
                )
                .await?;
                Ok(validate_view_query_related(
                    &fields,
                    &context,
                    Some(filter),
                    &[],
                    &relations,
                ))
            }
            PolicyCondition::Formula(expression) => {
                let ports = MySqlFormulaPorts::new(self.pool.clone(), self.registry.clone());
                let engine = FormulaEngine::new(&ports, &elevated);
                let draft = condition_definition(definition, expression)?;
                let mut workspace = engine.workspace().with_override(draft);
                let context = workspace.context(&definition.record_type).await?;
                match compile_formula(expression, &context) {
                    Err(errors) => Ok(errors
                        .into_iter()
                        .map(|error| FieldViolation::new("formula", error.describe()))
                        .collect()),
                    Ok(compiled) if compiled.kind != Some(ValueKind::Bool) => {
                        Ok(vec![FieldViolation::new(
                            "formula",
                            "must result in a boolean (true or false)",
                        )])
                    }
                    Ok(_) => Ok(Vec::new()),
                }
            }
        }
    }

    pub(crate) async fn lockable_fields(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
    ) -> Result<Vec<String>, AppError> {
        let elevated = condition_scope(scope);
        let definitions = MySqlFieldDefinitionRepository::new(self.pool.clone());
        let fields = FieldSet::for_scope(&definitions, definition, &elevated).await?;
        let mut keys = vec![CUSTOM_FIELDS_PREFIX.to_owned()];
        for field in fields.all() {
            if field.is_system() {
                keys.push(field.key().to_owned());
            } else {
                keys.push(format!("{CUSTOM_FIELDS_PREFIX}.{}", field.key()));
            }
        }
        Ok(keys)
    }

    pub(crate) async fn approvers(
        &self,
        _tx: &mut MySqlTx,
        step: &StepSnapshot,
        owner: Option<&UserId>,
    ) -> Result<Vec<UserId>, AppError> {
        match step.kind() {
            Some(ApproverKind::User) => match &step.approver_user_id {
                Some(user) => self.users_where("m.id = ?", user.clone()).await,
                None => Ok(Vec::new()),
            },
            Some(ApproverKind::Role) => match &step.approver_role_id {
                Some(role) => self.users_where("m.role_id = ?", role.clone()).await,
                None => Ok(Vec::new()),
            },
            Some(ApproverKind::OwnerTeamManager) => {
                let Some(owner) = owner else {
                    return Ok(Vec::new());
                };
                let managers: Vec<UserId> = self
                    .team_managers(owner, false)
                    .await?
                    .into_iter()
                    .filter(|manager| manager != owner)
                    .collect();
                if !managers.is_empty() {
                    return Ok(managers);
                }
                Ok(self
                    .team_managers(owner, true)
                    .await?
                    .into_iter()
                    .filter(|manager| manager != owner)
                    .collect())
            }
            None => Ok(Vec::new()),
        }
    }

    pub(crate) async fn managers(&self, _tx: &mut MySqlTx) -> Result<Vec<UserId>, AppError> {
        self.users_where(
            "EXISTS (SELECT 1 FROM role_permissions rp WHERE rp.role_id = m.role_id AND rp.permission = ?)",
            aphel_kernel::domain::access::Permission::APPROVALS_MANAGE
                .as_str()
                .to_owned(),
        )
        .await
    }

    pub(crate) async fn names(&self, ids: &[UserId]) -> Result<HashMap<String, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT id, display_name FROM users WHERE id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.as_str().to_owned());
        }
        separated.push_unseparated(")");
        let rows: Vec<(String, String)> = builder.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().collect())
    }

    pub(crate) async fn record_titles(
        &self,
        definition: &EntityDefinition,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let Some(column) = definition.title_column() else {
            return Ok(HashMap::new());
        };
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id, CAST(r.{} AS CHAR) FROM {} r WHERE r.id IN (",
            quoted(column),
            quoted(&definition.table)
        ));
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        separated.push_unseparated(")");
        let rows: Vec<(String, Option<String>)> =
            builder.build_query_as().fetch_all(&self.pool).await?;
        let mut titles = HashMap::new();
        for (id, title) in rows {
            if let Some(title) = title {
                titles.insert(parse_uuid(&id, "record.id")?, title);
            }
        }
        Ok(titles)
    }

    pub(crate) async fn stage_names(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT id, name FROM workflow_stages WHERE id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        separated.push_unseparated(")");
        let rows: Vec<(String, String)> = builder.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(id, name)| Ok((parse_uuid(&id, "workflow_stages.id")?, name)))
            .collect()
    }

    pub(crate) async fn role_exists(&self, id: Uuid) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE id = ?)")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await?;
        Ok(exists)
    }

    pub(crate) async fn is_member(&self, user_id: &UserId) -> Result<bool, AppError> {
        Ok(!self
            .users_where("m.id = ?", user_id.as_str().to_owned())
            .await?
            .is_empty())
    }

    pub(crate) async fn role_keys(&self) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, `key` FROM roles ORDER BY `key`")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|(id, key)| Ok((parse_uuid(&id, "roles.id")?, key)))
            .collect()
    }

    pub(crate) async fn member_emails(&self) -> Result<Vec<(UserId, String)>, AppError> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, email FROM users WHERE status = 'active' ORDER BY email")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|(id, email)| Ok((UserId::new(id)?, email)))
            .collect()
    }
}
