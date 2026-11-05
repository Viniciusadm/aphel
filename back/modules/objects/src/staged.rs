use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::domain::custom_objects::CustomObject;
use aphel_sdk::workflow::{
    StageLocation, StageMove, StageUsage, StagedRecordSource, UsageTarget, WorkflowCatalog,
};

pub(crate) const STAGED_TRAIT: &str = "workflow.staged";
pub(crate) const STAGE_FIELD_KEY: &str = "stage";
pub(crate) const STAGE_FIELD_LABEL: &str = "Etapa";

pub(crate) fn is_staged(object: &CustomObject) -> bool {
    object
        .entity_definition
        .as_ref()
        .and_then(|declaration| declaration.get("traits"))
        .is_some_and(|traits| traits.get(STAGED_TRAIT).is_some())
}

pub(crate) fn with_staged(object: &CustomObject, staged: bool) -> Option<Value> {
    if !staged {
        return None;
    }
    Some(json!({
        "key": object.key,
        "label": object.data.label_singular,
        "label_plural": object.data.label_plural,
        "traits": { STAGED_TRAIT: { "stage_field": STAGE_FIELD_KEY } },
    }))
}

const STAGED_VALUES: &str = "FROM custom_field_values v \
     JOIN field_definitions f ON f.id = v.field_id AND f.is_system = FALSE AND f.`key` = 'stage' \
     JOIN custom_objects o ON f.record_type = CONCAT('custom.', o.`key`) COLLATE utf8mb4_unicode_ci \
     JOIN custom_records r ON r.id = v.record_id AND r.object_id = o.id \
     WHERE o.module_key IS NULL AND JSON_CONTAINS_PATH(o.entity_definition, 'one', '$.traits.\"workflow.staged\"')";

fn catalog() -> Result<std::sync::Arc<dyn WorkflowCatalog>, AppError> {
    aphel_sdk::platform::modules::app()
        .services()
        .get::<dyn WorkflowCatalog>()
        .ok_or_else(|| AppError::Configuration("workflow is unavailable".to_owned()))
}

async fn stage_ids(pool: &MySqlPool, target: UsageTarget) -> Result<Vec<Uuid>, AppError> {
    match target {
        UsageTarget::Stage(id) => Ok(vec![id]),
        UsageTarget::Pipeline(id) => Ok(catalog()?
            .stages(pool, id)
            .await?
            .into_iter()
            .map(|stage| stage.id)
            .collect()),
    }
}

fn staged_query(select: &str, stages: &[Uuid]) -> QueryBuilder<MySql> {
    let mut query = QueryBuilder::new(format!(
        "SELECT {select} {STAGED_VALUES} AND v.value_text IN ("
    ));
    let mut separated = query.separated(", ");
    for stage in stages {
        separated.push_bind(stage.to_string());
    }
    query.push(")");
    query
}

pub(crate) struct CustomStagedRecords;

#[async_trait]
impl StagedRecordSource for CustomStagedRecords {
    fn record_type(&self) -> RecordType {
        RecordType::from_static("objects.staged_records")
    }

    async fn stage_counts(
        &self,
        ctx: &Ctx<'_>,
        pipeline_id: Uuid,
    ) -> Result<HashMap<Uuid, u64>, AppError> {
        let stages = stage_ids(ctx.pool(), UsageTarget::Pipeline(pipeline_id)).await?;
        if stages.is_empty() {
            return Ok(HashMap::new());
        }
        let mut query = staged_query("v.value_text, COUNT(*)", &stages);
        query.push(" AND r.deleted_at IS NULL GROUP BY v.value_text");
        let rows: Vec<(String, i64)> = query.build_query_as().fetch_all(ctx.pool()).await?;
        Ok(rows
            .into_iter()
            .filter_map(|(stage, count)| {
                Some((Uuid::parse_str(&stage).ok()?, u64::try_from(count).ok()?))
            })
            .collect())
    }

    async fn usage(&self, pool: &MySqlPool, target: UsageTarget) -> Result<StageUsage, AppError> {
        let stages = stage_ids(pool, target).await?;
        if stages.is_empty() {
            return Ok(StageUsage::default());
        }
        let mut query = staged_query(
            "COUNT(*), CAST(COALESCE(SUM(r.deleted_at IS NULL), 0) AS UNSIGNED)",
            &stages,
        );
        let (total, active): (i64, u64) = query.build_query_as().fetch_one(pool).await?;
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
        let mut query = QueryBuilder::<MySql>::new(format!(
            "SELECT v.value_text {STAGED_VALUES} AND r.deleted_at IS NULL AND r.id = "
        ));
        query.push_bind(id.to_string());
        let stage: Option<(String,)> = query.build_query_as().fetch_optional(pool).await?;
        let Some(stage_id) = stage.and_then(|(stage,)| Uuid::parse_str(&stage).ok()) else {
            return Ok(None);
        };
        Ok(catalog()?
            .stage(pool, stage_id)
            .await?
            .map(|stage| StageLocation {
                pipeline_id: stage.pipeline_id,
                stage_id: stage.id,
            }))
    }

    async fn move_records(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        movement: &StageMove,
    ) -> Result<u64, AppError> {
        ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
        let result = sqlx::query(
            "UPDATE custom_field_values v \
             JOIN field_definitions f ON f.id = v.field_id AND f.is_system = FALSE AND f.`key` = 'stage' \
             JOIN custom_objects o ON f.record_type = CONCAT('custom.', o.`key`) COLLATE utf8mb4_unicode_ci \
             SET v.value_text = ? \
             WHERE o.module_key IS NULL \
               AND JSON_CONTAINS_PATH(o.entity_definition, 'one', '$.traits.\"workflow.staged\"') \
               AND v.value_text = ?",
        )
        .bind(movement.to.id.to_string())
        .bind(movement.from.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }
}
