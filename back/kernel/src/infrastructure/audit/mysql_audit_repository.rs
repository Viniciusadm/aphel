use async_trait::async_trait;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::audit::{AuditLogFilter, AuditLogView, AuditRepository};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::shared::{Actor, EventCausation, FieldChange};
use crate::errors::AppError;
use crate::infrastructure::events::rows::{actor_from_columns, record_from_columns, uuid_column};
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct MySqlAuditRepository {
    pool: MySqlPool,
}

impl MySqlAuditRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn push_filters(builder: &mut QueryBuilder<MySql>, filter: &AuditLogFilter) {
    builder.push(" WHERE 1 = 1");
    if let Some(record_type) = &filter.record_type {
        builder.push(" AND a.record_type = ");
        builder.push_bind(record_type.as_str().to_owned());
    }
    if let Some(record_id) = &filter.record_id {
        builder.push(" AND a.record_id = ");
        builder.push_bind(record_id.to_string());
    }
    if let Some(actor_id) = &filter.actor_id {
        builder.push(" AND a.actor_id = ");
        builder.push_bind(actor_id.clone());
    }
    if let Some(from) = filter.from {
        builder.push(" AND a.occurred_at >= ");
        builder.push_bind(mapper::naive(from));
    }
    if let Some(to) = filter.to {
        builder.push(" AND a.occurred_at <= ");
        builder.push_bind(mapper::naive(to));
    }
}

fn changes_from_json(value: Value) -> Result<Vec<FieldChange>, AppError> {
    let Value::Array(items) = value else {
        return Err(mapper::invalid_column(
            "audit_logs.changes",
            &value.to_string(),
        ));
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::Object(mut fields) => {
                let field = match fields.remove("field") {
                    Some(Value::String(field)) => field,
                    _ => {
                        return Err(AppError::Persistence(
                            "audit change without a field name".to_owned(),
                        ));
                    }
                };
                Ok(FieldChange::new(
                    field,
                    fields.remove("old").unwrap_or(Value::Null),
                    fields.remove("new").unwrap_or(Value::Null),
                ))
            }
            other => Err(mapper::invalid_column(
                "audit_logs.changes",
                &other.to_string(),
            )),
        })
        .collect()
}

fn audit_log_from_row(row: &MySqlRow) -> Result<AuditLogView, AppError> {
    let actor = actor_from_columns(row.try_get("actor_type")?, row.try_get("actor_id")?)?;
    let is_user = matches!(actor, Actor::User(_));
    let actor_name: Option<String> = row.try_get("actor_name")?;
    let actor_email: Option<String> = row.try_get("actor_email")?;
    let Json(changes): Json<Value> = row.try_get("changes")?;
    let causation: Option<Json<EventCausation>> = row.try_get("causation")?;
    Ok(AuditLogView {
        id: uuid_column("audit_logs.id", row.try_get("id")?)?,
        actor,
        actor_name: actor_name.filter(|_| is_user),
        actor_email: actor_email.filter(|_| is_user),
        action: row.try_get("action")?,
        record: record_from_columns(row.try_get("record_type")?, row.try_get("record_id")?)?,
        changes: changes_from_json(changes)?,
        ip: row.try_get("ip")?,
        causation: causation.map(|Json(causation)| causation),
        execution: row
            .try_get::<Option<Json<crate::application::shared::ExecutionContext>>, _>("execution")?
            .map(|Json(value)| value),
        occurred_at: mapper::utc(row.try_get("occurred_at")?),
    })
}

#[async_trait]
impl AuditRepository for MySqlAuditRepository {
    async fn list(
        &self,
        filter: &AuditLogFilter,
        window: ListWindow,
    ) -> Result<Listing<AuditLogView>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM audit_logs a");
        push_filters(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut select = QueryBuilder::<MySql>::new(
            "SELECT a.id, a.action, a.record_type, a.record_id, a.actor_type, a.actor_id, a.changes, a.ip, a.causation, a.execution, a.occurred_at, \
             u.display_name AS actor_name, u.email AS actor_email \
             FROM audit_logs a \
             LEFT JOIN users u ON u.id = a.actor_id AND a.actor_type = ",
        );
        select.push_bind(Actor::USER);
        push_filters(&mut select, filter);
        select.push(" ORDER BY a.occurred_at DESC, a.id DESC LIMIT ");
        select.push_bind(window.limit);
        select.push(" OFFSET ");
        select.push_bind(window.offset);
        let rows = select.build().fetch_all(&self.pool).await?;

        Ok(Listing {
            items: rows
                .iter()
                .map(audit_log_from_row)
                .collect::<Result<Vec<_>, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }
}
