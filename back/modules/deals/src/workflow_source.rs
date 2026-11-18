use crate::store::deal_rows as rows;
use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, RecordType,
};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid;
use aphel_sdk::workflow::{StageLocation, StageMove, StageUsage, StagedRecordSource, UsageTarget};
use async_trait::async_trait;
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
        let rows =
            rows::stage_counts(ctx.pool(), ctx.scope, definition, &visibility, pipeline_id).await?;
        rows.into_iter()
            .map(|(id, count)| {
                Ok((
                    parse_uuid(&id, "deals.stage_id")?,
                    u64::try_from(count).unwrap_or_default(),
                ))
            })
            .collect()
    }
    async fn usage(&self, pool: &Db, target: UsageTarget) -> Result<StageUsage, AppError> {
        let (column, id) = match target {
            UsageTarget::Stage(id) => (rows::UsageColumn::Stage, id),
            UsageTarget::Pipeline(id) => (rows::UsageColumn::Pipeline, id),
        };
        let (total, active) = rows::usage(pool, column, id).await?;
        Ok(StageUsage {
            total: u64::try_from(total).unwrap_or_default(),
            active,
        })
    }
    async fn location(&self, pool: &Db, id: Uuid) -> Result<Option<StageLocation>, AppError> {
        let row = rows::location(pool, id).await?;
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
        tx: &mut Tx,
        movement: &StageMove,
    ) -> Result<u64, AppError> {
        ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
        let rows = rows::lock_stage(tx, movement.from.id).await?;
        if rows.is_empty() {
            return Ok(0);
        }
        let now = ctx.now();
        rows::move_stage(
            tx,
            movement.from.id,
            movement.to.id,
            movement.target_pipeline_id,
            now.naive_utc(),
        )
        .await?;
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
