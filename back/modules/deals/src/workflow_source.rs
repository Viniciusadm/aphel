use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, RecordType,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::infrastructure::query::compiler::push_record_scope;
use aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid;
use aphel_sdk::workflow::{StageLocation, StageMove, StageUsage, StagedRecordSource, UsageTarget};
use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) struct DealStages;
#[async_trait]
impl StagedRecordSource for DealStages {
    fn record_type(&self) -> RecordType {
        RecordType::DEAL
    }
    async fn stage_counts(
        &self,
        ctx: &Ctx<'_>,
        pipeline_id: Uuid,
    ) -> Result<HashMap<Uuid, u64>, AppError> {
        let Ok(visibility) = ctx.scope.record_filter(&Permission::DEALS_READ) else {
            return Ok(HashMap::new());
        };
        let fields = ctx.field_set(ctx.scope, RecordType::DEAL.as_str()).await?;
        if fields
            .find_active("stage_id")
            .is_none_or(|field| field.is_hidden())
            || fields
                .find_active("pipeline_id")
                .is_none_or(|field| field.is_hidden())
        {
            return Ok(HashMap::new());
        }
        let definition = ctx.registry().require(RecordType::DEAL.as_str())?;
        let mut query = QueryBuilder::<MySql>::new(
            "SELECT r.stage_id, COUNT(*) FROM deals r WHERE r.deleted_at IS NULL AND r.pipeline_id = ",
        );
        query.push_bind(pipeline_id.to_string());
        push_record_scope(&mut query, ctx.scope, definition, &visibility);
        query.push(" GROUP BY r.stage_id");
        let rows = query
            .build_query_as::<(String, i64)>()
            .fetch_all(ctx.pool())
            .await?;
        rows.into_iter()
            .map(|(id, count)| {
                Ok((
                    parse_uuid(&id, "deals.stage_id")?,
                    u64::try_from(count).unwrap_or_default(),
                ))
            })
            .collect()
    }
    async fn usage(&self, pool: &MySqlPool, target: UsageTarget) -> Result<StageUsage, AppError> {
        let (sql, id) = match target {
            UsageTarget::Stage(id) => (
                "SELECT COUNT(*), CAST(COALESCE(SUM(deleted_at IS NULL), 0) AS UNSIGNED) FROM deals WHERE stage_id = ?",
                id,
            ),
            UsageTarget::Pipeline(id) => (
                "SELECT COUNT(*), CAST(COALESCE(SUM(deleted_at IS NULL), 0) AS UNSIGNED) FROM deals WHERE pipeline_id = ?",
                id,
            ),
        };
        let (total, active): (i64, u64) = sqlx::query_as(sql)
            .bind(id.to_string())
            .fetch_one(pool)
            .await?;
        Ok(StageUsage {
            total: u64::try_from(total).unwrap_or_default(),
            active,
        })
    }
    async fn location(
        &self,
        pool: &MySqlPool,
        id: Uuid,
    ) -> Result<Option<StageLocation>, AppError> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT pipeline_id, stage_id FROM deals WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;
        row.map(|(pipeline, stage)| {
            Ok(StageLocation {
                pipeline_id: parse_uuid(&pipeline, "deals.pipeline_id")?,
                stage_id: parse_uuid(&stage, "deals.stage_id")?,
            })
        })
        .transpose()
    }
    async fn move_records(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        movement: &StageMove,
    ) -> Result<u64, AppError> {
        ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
        let rows: Vec<(String,bool)> = sqlx::query_as("SELECT id, deleted_at IS NOT NULL FROM deals WHERE stage_id = ? ORDER BY id FOR UPDATE").bind(movement.from.id.to_string()).fetch_all(&mut **tx).await?;
        if rows.is_empty() {
            return Ok(0);
        }
        let now = ctx.now();
        sqlx::query("UPDATE deals SET stage_id = ?, pipeline_id = ?, stage_entered_at = ?, updated_at = ? WHERE stage_id = ?")
            .bind(movement.to.id.to_string()).bind(movement.target_pipeline_id.to_string()).bind(now.naive_utc()).bind(now.naive_utc()).bind(movement.from.id.to_string()).execute(&mut **tx).await?;
        for (id, deleted) in &rows {
            if *deleted {
                continue;
            }
            let record = RecordRef::new(RecordType::DEAL, parse_uuid(id, "deals.id")?);
            ctx.audit()
                .record(
                    tx,
                    AuditEntry::new(Actor::from_scope(ctx.scope), "deal.stage_changed", now)
                        .with_record(record.clone()),
                )
                .await?;
            ctx.events().publish(tx, DomainEvent::new(Actor::from_scope(ctx.scope),"deal.stage_changed",now).with_record(record.clone()).with_payload(serde_json::json!({ "from": movement.from, "to": movement.to, "stage_entered_at": now.to_rfc3339_opts(chrono::SecondsFormat::Micros,true) }))).await?;
            if let Some((from, to)) = &movement.pipelines {
                ctx.events().publish(tx,DomainEvent::new(Actor::from_scope(ctx.scope),"deal.pipeline_changed",now).with_record(record).with_payload(serde_json::json!({"from":from,"to":to,"from_stage":movement.from,"to_stage":movement.to}))).await?;
            }
        }
        Ok(rows.len() as u64)
    }
}
