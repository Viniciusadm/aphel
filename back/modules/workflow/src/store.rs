use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlConnection, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{LostReason, Pipeline, PipelineStage, StageKind, StageUsage};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper::{
    invalid_column, naive, parse_uuid, utc,
};

const PIPELINE_COLUMNS: &str = "id, name, is_default, position, is_active, created_at, updated_at, \
     (SELECT GROUP_CONCAT(pt.team_id ORDER BY pt.team_id SEPARATOR ',') FROM workflow_teams pt \
     WHERE pt.pipeline_id = workflow.id) AS team_ids";
const STAGE_COLUMNS: &str = "id, pipeline_id, name, position, probability, rotting_days, kind, is_active, created_at, updated_at, \
     CAST(entry_required_fields AS CHAR) AS entry_required_fields, \
     CAST(exit_required_fields AS CHAR) AS exit_required_fields";
const LOST_REASON_COLUMNS: &str = "id, name, position, is_active, created_at, updated_at";

fn timestamp(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    Ok(utc(row.try_get::<NaiveDateTime, _>(column)?))
}

fn team_ids(row: &MySqlRow) -> Result<Vec<Uuid>, AppError> {
    row.try_get::<Option<String>, _>("team_ids")?
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| parse_uuid(value, "workflow_teams.team_id"))
        .collect()
}

fn field_keys(row: &MySqlRow, column: &str) -> Result<Vec<String>, AppError> {
    match row.try_get::<Option<String>, _>(column)? {
        None => Ok(Vec::new()),
        Some(text) => serde_json::from_str(&text).map_err(|_| invalid_column(column, &text)),
    }
}

fn keys_json(keys: &[String]) -> Option<String> {
    if keys.is_empty() {
        None
    } else {
        serde_json::to_string(keys).ok()
    }
}

fn pipeline_from_row(row: &MySqlRow) -> Result<Pipeline, AppError> {
    Ok(Pipeline {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "workflow.id")?,
        name: row.try_get("name")?,
        is_default: row.try_get("is_default")?,
        position: row.try_get("position")?,
        is_active: row.try_get("is_active")?,
        team_ids: team_ids(row)?,
        created_at: timestamp(row, "created_at")?,
        updated_at: timestamp(row, "updated_at")?,
    })
}

pub(crate) fn stage_from_row(row: &MySqlRow) -> Result<PipelineStage, AppError> {
    let kind: String = row.try_get("kind")?;
    Ok(PipelineStage {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "workflow_stages.id")?,
        pipeline_id: parse_uuid(
            &row.try_get::<String, _>("pipeline_id")?,
            "workflow_stages.pipeline_id",
        )?,
        name: row.try_get("name")?,
        position: row.try_get("position")?,
        probability: row.try_get("probability")?,
        rotting_days: row.try_get("rotting_days")?,
        kind: StageKind::parse(&kind)
            .ok_or_else(|| invalid_column("workflow_stages.kind", &kind))?,
        is_active: row.try_get("is_active")?,
        entry_required_fields: field_keys(row, "entry_required_fields")?,
        exit_required_fields: field_keys(row, "exit_required_fields")?,
        created_at: timestamp(row, "created_at")?,
        updated_at: timestamp(row, "updated_at")?,
    })
}

fn lost_reason_from_row(row: &MySqlRow) -> Result<LostReason, AppError> {
    Ok(LostReason {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "lost_reasons.id")?,
        name: row.try_get("name")?,
        position: row.try_get("position")?,
        is_active: row.try_get("is_active")?,
        created_at: timestamp(row, "created_at")?,
        updated_at: timestamp(row, "updated_at")?,
    })
}

async fn reorder(
    connection: &mut MySqlConnection,
    table: &str,
    scope: Option<(&str, String)>,
    ordered_ids: &[Uuid],
) -> Result<(), AppError> {
    if ordered_ids.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::<MySql>::new(format!("UPDATE {table} SET position = CASE id"));
    for (index, id) in ordered_ids.iter().enumerate() {
        builder
            .push(" WHEN ")
            .push_bind(id.to_string())
            .push(" THEN ")
            .push_bind(index as i32);
    }
    builder.push(" ELSE position END WHERE id IN (");
    let mut separated = builder.separated(", ");
    for id in ordered_ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
    if let Some((column, value)) = scope {
        builder.push(format!(" AND {column} = ")).push_bind(value);
    }
    builder.build().execute(connection).await?;
    Ok(())
}

#[derive(Clone)]
pub struct PipelineStore {
    pool: MySqlPool,
}

impl PipelineStore {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub async fn list_pipelines(&self) -> Result<Vec<Pipeline>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {PIPELINE_COLUMNS} FROM workflow \
             ORDER BY position, id"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(pipeline_from_row).collect()
    }

    pub async fn find_pipeline(&self, pipeline_id: Uuid) -> Result<Option<Pipeline>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {PIPELINE_COLUMNS} FROM workflow WHERE id = ?"
        )))
        .bind(pipeline_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(pipeline_from_row).transpose()
    }

    pub async fn default_pipeline(&self) -> Result<Option<Pipeline>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {PIPELINE_COLUMNS} FROM workflow WHERE is_active \
             ORDER BY is_default DESC, position, id LIMIT 1"
        )))
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(pipeline_from_row).transpose()
    }

    pub async fn list_stages(&self, pipeline_id: Uuid) -> Result<Vec<PipelineStage>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {STAGE_COLUMNS} FROM workflow_stages WHERE pipeline_id = ? \
             ORDER BY position, id"
        )))
        .bind(pipeline_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(stage_from_row).collect()
    }

    pub(crate) async fn stages_for_pipelines(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<PipelineStage>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT {STAGE_COLUMNS} FROM workflow_stages WHERE pipeline_id IN ("
        ));
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(") ORDER BY pipeline_id, position, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(stage_from_row).collect()
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

    pub async fn find_stage(&self, stage_id: Uuid) -> Result<Option<PipelineStage>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {STAGE_COLUMNS} FROM workflow_stages WHERE id = ?"
        )))
        .bind(stage_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(stage_from_row).transpose()
    }

    pub(crate) async fn stage_record_counts(
        &self,
        ctx: &Ctx<'_>,
        pipeline_id: Uuid,
    ) -> Result<HashMap<Uuid, u64>, AppError> {
        let mut result = HashMap::new();
        for source in aphel_sdk::platform::modules::app()
            .contributions::<dyn aphel_sdk::workflow::StagedRecordSource>()
        {
            for (stage, count) in source.stage_counts(ctx, pipeline_id).await? {
                *result.entry(stage).or_insert(0) += count;
            }
        }
        Ok(result)
    }
    pub(crate) async fn stage_usage(&self, id: Uuid) -> Result<StageUsage, AppError> {
        self.usage(aphel_sdk::workflow::UsageTarget::Stage(id))
            .await
    }
    pub(crate) async fn pipeline_usage(&self, id: Uuid) -> Result<StageUsage, AppError> {
        self.usage(aphel_sdk::workflow::UsageTarget::Pipeline(id))
            .await
    }
    async fn usage(
        &self,
        target: aphel_sdk::workflow::UsageTarget,
    ) -> Result<StageUsage, AppError> {
        let mut total = StageUsage::default();
        for (_, source) in aphel_sdk::platform::modules::app()
            .all_contributions::<dyn aphel_sdk::workflow::StagedRecordSource>()
        {
            let usage = source.usage(&self.pool, target).await?;
            total.total += usage.total;
            total.active += usage.active;
        }
        Ok(total)
    }

    pub(crate) async fn existing_team_ids(&self, team_ids: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
        if team_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM teams WHERE ");
        builder.push("id IN (");
        let mut separated = builder.separated(", ");
        for id in team_ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let rows: Vec<(String,)> = builder.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(id,)| parse_uuid(&id, "teams.id"))
            .collect()
    }

    pub(crate) async fn insert_pipeline(
        &self,
        tx: &mut MySqlTx,
        pipeline: &Pipeline,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO workflow (id, name, is_default, position, is_active, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(pipeline.id.to_string())
        .bind(&pipeline.name)
        .bind(pipeline.is_default)
        .bind(pipeline.position)
        .bind(pipeline.is_active)
        .bind(naive(pipeline.created_at))
        .bind(naive(pipeline.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn replace_workflow_teams(
        &self,
        tx: &mut MySqlTx,
        pipeline_id: Uuid,
        team_ids: &[Uuid],
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM workflow_teams WHERE pipeline_id = ?")
            .bind(pipeline_id.to_string())
            .execute(&mut **tx)
            .await?;
        if team_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO workflow_teams (pipeline_id, team_id) \
             SELECT p.id, t.id FROM workflow p JOIN teams t WHERE p.id = ",
        );
        builder
            .push_bind(pipeline_id.to_string())
            .push(" AND t.id IN (");
        let mut separated = builder.separated(", ");
        for id in team_ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn set_default_pipeline(
        &self,
        tx: &mut MySqlTx,
        pipeline_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE workflow SET is_default = (id = ?)")
            .bind(pipeline_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn reorder_pipelines(
        &self,
        tx: &mut MySqlTx,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        reorder(tx, "workflow", None, ordered_ids).await
    }

    pub(crate) async fn delete_pipeline(
        &self,
        tx: &mut MySqlTx,
        pipeline_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM workflow WHERE id = ?")
            .bind(pipeline_id.to_string())
            .execute(&mut **tx)
            .await?;
        aphel_sdk::reference_cleanup::removed(tx, "workflow.pipeline", pipeline_id).await?;
        Ok(())
    }

    pub(crate) async fn update_pipeline(
        &self,
        tx: &mut MySqlTx,
        pipeline: &Pipeline,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE workflow SET name = ?, is_active = ?, updated_at = ? WHERE id = ?")
            .bind(&pipeline.name)
            .bind(pipeline.is_active)
            .bind(naive(pipeline.updated_at))
            .bind(pipeline.id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_stage(
        &self,
        tx: &mut MySqlTx,
        stage: &PipelineStage,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO workflow_stages (id, pipeline_id, name, position, probability, rotting_days, kind, is_active, \
             entry_required_fields, exit_required_fields, created_at, updated_at) \
             SELECT ?, p.id, ?, ?, ?, ?, ?, ?, CAST(? AS JSON), CAST(? AS JSON), ?, ? FROM workflow p WHERE p.id = ?",
        )
        .bind(stage.id.to_string())
        .bind(&stage.name)
        .bind(stage.position)
        .bind(stage.probability)
        .bind(stage.rotting_days)
        .bind(stage.kind.as_str())
        .bind(stage.is_active)
        .bind(keys_json(&stage.entry_required_fields))
        .bind(keys_json(&stage.exit_required_fields))
        .bind(naive(stage.created_at))
        .bind(naive(stage.updated_at))
        .bind(stage.pipeline_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn update_stage(
        &self,
        tx: &mut MySqlTx,
        stage: &PipelineStage,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE workflow_stages SET name = ?, probability = ?, rotting_days = ?, kind = ?, is_active = ?, \
             entry_required_fields = CAST(? AS JSON), exit_required_fields = CAST(? AS JSON), updated_at = ? \
             WHERE id = ?",
        )
        .bind(&stage.name)
        .bind(stage.probability)
        .bind(stage.rotting_days)
        .bind(stage.kind.as_str())
        .bind(stage.is_active)
        .bind(keys_json(&stage.entry_required_fields))
        .bind(keys_json(&stage.exit_required_fields))
        .bind(naive(stage.updated_at))
        .bind(stage.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn reorder_stages(
        &self,
        tx: &mut MySqlTx,
        pipeline_id: Uuid,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        reorder(
            tx,
            "workflow_stages",
            Some(("pipeline_id", pipeline_id.to_string())),
            ordered_ids,
        )
        .await
    }

    pub(crate) async fn delete_stage(
        &self,
        tx: &mut MySqlTx,
        stage_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM workflow_stages WHERE id = ?")
            .bind(stage_id.to_string())
            .execute(&mut **tx)
            .await?;
        aphel_sdk::reference_cleanup::removed(tx, "workflow.stage", stage_id).await?;
        Ok(())
    }

    pub async fn list_lost_reasons(&self) -> Result<Vec<LostReason>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {LOST_REASON_COLUMNS} FROM lost_reasons ORDER BY position, id"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(lost_reason_from_row).collect()
    }

    pub async fn find_lost_reason(&self, reason_id: Uuid) -> Result<Option<LostReason>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {LOST_REASON_COLUMNS} FROM lost_reasons WHERE id = ?"
        )))
        .bind(reason_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(lost_reason_from_row).transpose()
    }

    pub(crate) async fn insert_lost_reason(
        &self,
        tx: &mut MySqlTx,
        reason: &LostReason,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO lost_reasons (id, name, position, is_active, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(reason.id.to_string())
        .bind(&reason.name)
        .bind(reason.position)
        .bind(reason.is_active)
        .bind(naive(reason.created_at))
        .bind(naive(reason.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_lost_reason(
        &self,
        tx: &mut MySqlTx,
        reason: &LostReason,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE lost_reasons SET name = ?, is_active = ?, updated_at = ? WHERE id = ?")
            .bind(&reason.name)
            .bind(reason.is_active)
            .bind(naive(reason.updated_at))
            .bind(reason.id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn reorder_lost_reasons(
        &self,
        tx: &mut MySqlTx,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        reorder(tx, "lost_reasons", None, ordered_ids).await
    }
}
