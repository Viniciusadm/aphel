use aphel_contract_crm::CrmRecordTypes;
use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::*;
use aphel_kernel::Ctx;
use aphel_kernel::application::query::Filter;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper::parse_uuid;
use aphel_kernel::infrastructure::repositories::mapper::{invalid_column, naive, utc};

macro_rules! model_columns {
    () => {
        "id, `key`, record_type, name, description, is_primary, is_active, warm_from, hot_from, \
         recalculated_at, created_by, updated_by, created_at, updated_at"
    };
}

macro_rules! score_columns {
    () => {
        "model_id, record_type, record_id, score, band, explanation, calculated_at"
    };
}

const MODEL_COLUMNS: &str = model_columns!();
const CRITERION_COLUMNS: &str = "id, model_id, `key`, label, kind, filter, `signal`, points, \
     max_points, decay_days, position";
const SCORE_COLUMNS: &str = score_columns!();

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(UserId::new)
        .transpose()
        .map_err(Into::into)
}

fn model_from_row(row: &MySqlRow) -> Result<ScoringModel, AppError> {
    Ok(ScoringModel {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "scoring_models.id")?,
        key: row.try_get("key")?,
        record_type: row.try_get("record_type")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        is_primary: row.try_get("is_primary")?,
        is_active: row.try_get("is_active")?,
        bands: ScoreBands {
            warm_from: row.try_get("warm_from")?,
            hot_from: row.try_get("hot_from")?,
        },
        criteria: Vec::new(),
        recalculated_at: row
            .try_get::<Option<NaiveDateTime>, _>("recalculated_at")?
            .map(utc),
        created_by: optional_user(row, "created_by")?,
        updated_by: optional_user(row, "updated_by")?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

fn criterion_from_row(row: &MySqlRow) -> Result<(Uuid, ScoringCriterion), AppError> {
    let kind: String = row.try_get("kind")?;
    let rule = match CriterionKind::parse(&kind) {
        Some(CriterionKind::Profile) => {
            let filter: Option<Json<Value>> = row.try_get("filter")?;
            let filter = filter
                .and_then(|Json(value)| serde_json::from_value::<Filter>(value).ok())
                .ok_or_else(|| invalid_column("scoring_criteria.filter", &kind))?;
            CriterionRule::Profile { filter }
        }
        Some(CriterionKind::Behavior) => {
            let signal: Option<String> = row.try_get("signal")?;
            let signal = signal.unwrap_or_default();
            CriterionRule::Behavior {
                signal: BehaviorSignal::parse(&signal)
                    .ok_or_else(|| invalid_column("scoring_criteria.signal", &signal))?,
                max_points: row.try_get("max_points")?,
                decay_days: row.try_get("decay_days")?,
            }
        }
        None => return Err(invalid_column("scoring_criteria.kind", &kind)),
    };
    let model_id = parse_uuid(
        &row.try_get::<String, _>("model_id")?,
        "scoring_criteria.model_id",
    )?;
    Ok((
        model_id,
        ScoringCriterion {
            id: parse_uuid(&row.try_get::<String, _>("id")?, "scoring_criteria.id")?,
            key: row.try_get("key")?,
            label: row.try_get("label")?,
            points: row.try_get("points")?,
            rule,
            position: row.try_get("position")?,
        },
    ))
}

fn score_from_row(row: &MySqlRow) -> Result<RecordScore, AppError> {
    let band: String = row.try_get("band")?;
    let Json(explanation): Json<Value> = row.try_get("explanation")?;
    Ok(RecordScore {
        model_id: parse_uuid(
            &row.try_get::<String, _>("model_id")?,
            "record_scores.model_id",
        )?,
        record_type: row.try_get("record_type")?,
        record_id: parse_uuid(
            &row.try_get::<String, _>("record_id")?,
            "record_scores.record_id",
        )?,
        score: row.try_get("score")?,
        band: ScoreBand::parse(&band).ok_or_else(|| invalid_column("record_scores.band", &band))?,
        explanation: serde_json::from_value::<Vec<ScoreContribution>>(explanation)
            .unwrap_or_default(),
        calculated_at: utc(row.try_get::<NaiveDateTime, _>("calculated_at")?),
    })
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push("(");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

fn push_signals(builder: &mut QueryBuilder<MySql>, signals: &[BehaviorSignal]) {
    builder.push("(");
    let mut separated = builder.separated(", ");
    for signal in signals {
        separated.push_bind(signal.as_str());
    }
    builder.push(")");
}

#[derive(Clone)]
pub(crate) struct ModelStore {
    pool: MySqlPool,
}

impl ModelStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn with_criteria(
        &self,
        mut models: Vec<ScoringModel>,
    ) -> Result<Vec<ScoringModel>, AppError> {
        if models.is_empty() {
            return Ok(models);
        }
        let ids: Vec<Uuid> = models.iter().map(|model| model.id).collect();
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT {CRITERION_COLUMNS} FROM scoring_criteria WHERE "
        ));
        builder.push("model_id IN ");
        push_ids(&mut builder, &ids);
        builder.push(" ORDER BY position, `key`");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut grouped: HashMap<Uuid, Vec<ScoringCriterion>> = HashMap::new();
        for row in &rows {
            let (model_id, criterion) = criterion_from_row(row)?;
            grouped.entry(model_id).or_default().push(criterion);
        }
        for model in &mut models {
            model.criteria = grouped.remove(&model.id).unwrap_or_default();
        }
        Ok(models)
    }

    pub(crate) async fn save_criteria(
        &self,
        tx: &mut MySqlTx,
        model: &ScoringModel,
    ) -> Result<(), AppError> {
        let mut builder = QueryBuilder::<MySql>::new("DELETE FROM scoring_criteria WHERE ");
        builder.push("model_id = ").push_bind(model.id.to_string());
        let ids: Vec<Uuid> = model
            .criteria
            .iter()
            .map(|criterion| criterion.id)
            .collect();
        if !ids.is_empty() {
            builder.push(" AND id NOT IN ");
            push_ids(&mut builder, &ids);
        }
        builder.build().execute(&mut **tx).await?;
        for criterion in &model.criteria {
            let filter = criterion
                .rule
                .filter()
                .and_then(|filter| serde_json::to_value(filter).ok())
                .map(Json);
            sqlx::query(
                "INSERT INTO scoring_criteria (id, model_id, `key`, label, kind, \
                 filter, `signal`, points, max_points, decay_days, position) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
                 ON DUPLICATE KEY UPDATE `key` = VALUES(`key`), label = VALUES(label), \
                 kind = VALUES(kind), filter = VALUES(filter), `signal` = VALUES(`signal`), \
                 points = VALUES(points), max_points = VALUES(max_points), \
                 decay_days = VALUES(decay_days), position = VALUES(position)",
            )
            .bind(criterion.id.to_string())
            .bind(model.id.to_string())
            .bind(&criterion.key)
            .bind(&criterion.label)
            .bind(criterion.rule.kind().as_str())
            .bind(filter)
            .bind(criterion.rule.signal().map(|signal| signal.as_str()))
            .bind(criterion.points)
            .bind(criterion.rule.max_points())
            .bind(criterion.rule.decay_days())
            .bind(criterion.position)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

impl ModelStore {
    pub(crate) async fn list(
        &self,
        record_type: Option<&str>,
    ) -> Result<Vec<ScoringModel>, AppError> {
        let mut builder =
            QueryBuilder::<MySql>::new(format!("SELECT {MODEL_COLUMNS} FROM scoring_models"));
        if let Some(record_type) = record_type {
            builder
                .push(" WHERE record_type = ")
                .push_bind(record_type.to_owned());
        }
        builder.push(" ORDER BY record_type, is_primary DESC, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let models = rows
            .iter()
            .map(model_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.with_criteria(models).await
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<ScoringModel>, AppError> {
        let row = sqlx::query(concat!(
            "SELECT ",
            model_columns!(),
            " FROM scoring_models WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(self.with_criteria(vec![model_from_row(&row)?]).await?.pop())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        model: &ScoringModel,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO scoring_models (id, `key`, record_type, name, description, \
             is_primary, is_active, warm_from, hot_from, created_by, updated_by, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(model.id.to_string())
        .bind(&model.key)
        .bind(&model.record_type)
        .bind(&model.name)
        .bind(&model.description)
        .bind(model.is_primary)
        .bind(model.is_active)
        .bind(model.bands.warm_from)
        .bind(model.bands.hot_from)
        .bind(model.created_by.as_ref().map(UserId::as_str))
        .bind(model.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(model.created_at))
        .bind(naive(model.updated_at))
        .execute(&mut **tx)
        .await?;
        self.save_criteria(tx, model).await
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        model: &ScoringModel,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE scoring_models SET name = ?, description = ?, is_primary = ?, is_active = ?, \
             warm_from = ?, hot_from = ?, updated_by = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&model.name)
        .bind(&model.description)
        .bind(model.is_primary)
        .bind(model.is_active)
        .bind(model.bands.warm_from)
        .bind(model.bands.hot_from)
        .bind(model.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(model.updated_at))
        .bind(model.id.to_string())
        .execute(&mut **tx)
        .await?;
        self.save_criteria(tx, model).await
    }

    pub(crate) async fn clear_primary(
        &self,
        tx: &mut MySqlTx,
        record_type: &str,
        keep: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE scoring_models SET is_primary = FALSE \
             WHERE record_type = ? AND id <> ? AND is_primary = TRUE",
        )
        .bind(record_type)
        .bind(keep.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM scoring_models WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn mark_recalculated(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE scoring_models SET recalculated_at = ? WHERE id = ?")
            .bind(naive(at))
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct ScoreStore {
    pool: MySqlPool,
}

impl ScoreStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    fn rollup_query(
        record_type: &str,
        signals: &[BehaviorSignal],
        since: Option<DateTime<Utc>>,
        record_ids: Option<&[Uuid]>,
        columns: &str,
    ) -> QueryBuilder<MySql> {
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT s.record_id AS target_id, {columns} FROM scoring_signals s WHERE "
        ));
        builder
            .push("s.record_type = ")
            .push_bind(record_type.to_owned())
            .push(" AND s.`signal` IN ");
        push_signals(&mut builder, signals);
        if let Some(since) = since {
            builder
                .push(" AND s.occurred_at >= ")
                .push_bind(naive(since));
        }
        if let Some(ids) = record_ids {
            builder.push(" AND s.record_id IN ");
            push_ids(&mut builder, ids);
        }
        if record_type == RecordType::COMPANY.as_str() {
            builder.push(format!(
                " UNION ALL SELECT c.company_id AS target_id, {columns} FROM scoring_signals s \
                 JOIN people c ON c.id = s.record_id \
                 WHERE "
            ));
            builder
                .push("s.record_type = ")
                .push_bind(RecordType::CONTACT.as_str())
                .push(" AND c.deleted_at IS NULL AND c.company_id IS NOT NULL AND s.`signal` IN ");
            push_signals(&mut builder, signals);
            if let Some(since) = since {
                builder
                    .push(" AND s.occurred_at >= ")
                    .push_bind(naive(since));
            }
            if let Some(ids) = record_ids {
                builder.push(" AND c.company_id IN ");
                push_ids(&mut builder, ids);
            }
        }
        builder
    }
}

impl ScoreStore {
    pub(crate) async fn record_signal(
        &self,
        record: &RecordRef,
        signal: BehaviorSignal,
        source_key: &str,
        event_id: Option<Uuid>,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let mut tx = aphel_kernel::database::receipt::begin(&self.pool).await?;
        let recorded = self
            .record_signal_in_tx(&mut tx, record, signal, source_key, event_id, occurred_at)
            .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    pub(crate) async fn record_signal_in_tx(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        signal: BehaviorSignal,
        source_key: &str,
        event_id: Option<Uuid>,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let source_key: String = source_key.chars().take(100).collect();
        let result = sqlx::query(
            "INSERT IGNORE INTO scoring_signals (id, record_type, record_id, \
             `signal`, source_key, event_id, occurred_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(signal.as_str())
        .bind(source_key)
        .bind(event_id.map(|id| id.to_string()))
        .bind(naive(occurred_at))
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn occurrences(
        &self,
        record_type: &str,
        record_ids: &[Uuid],
        signals: &[BehaviorSignal],
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<SignalOccurrence>, AppError> {
        if record_ids.is_empty() || signals.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::rollup_query(
            record_type,
            signals,
            since,
            Some(record_ids),
            "s.`signal` AS signal_key, s.occurred_at AS occurred_at",
        );
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let signal: String = row.try_get("signal_key")?;
                Ok(SignalOccurrence {
                    record_id: parse_uuid(
                        &row.try_get::<String, _>("target_id")?,
                        "scoring_signals.record_id",
                    )?,
                    signal: BehaviorSignal::parse(&signal)
                        .ok_or_else(|| invalid_column("scoring_signals.signal", &signal))?,
                    occurred_at: utc(row.try_get::<NaiveDateTime, _>("occurred_at")?),
                })
            })
            .collect()
    }

    pub(crate) async fn signal_record_ids(
        &self,
        record_type: &str,
        signals: &[BehaviorSignal],
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<Uuid>, AppError> {
        if signals.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::rollup_query(record_type, signals, since, None, "1 AS one");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut ids: Vec<Uuid> = rows
            .iter()
            .map(|row| {
                parse_uuid(
                    &row.try_get::<String, _>("target_id")?,
                    "scoring_signals.record_id",
                )
            })
            .collect::<Result<_, _>>()?;
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub(crate) async fn scored_record_ids(&self, model_id: Uuid) -> Result<Vec<Uuid>, AppError> {
        let rows = sqlx::query("SELECT record_id FROM record_scores WHERE model_id = ?")
            .bind(model_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                parse_uuid(
                    &row.try_get::<String, _>("record_id")?,
                    "record_scores.record_id",
                )
            })
            .collect()
    }

    pub(crate) async fn lock_scores(
        &self,
        tx: &mut MySqlTx,
        model_id: Uuid,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, RecordScore>, AppError> {
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new(format!("SELECT {SCORE_COLUMNS} FROM record_scores WHERE "));
        builder
            .push("model_id = ")
            .push_bind(model_id.to_string())
            .push(" AND record_id IN ");
        push_ids(&mut builder, record_ids);
        builder.push(" FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        rows.iter()
            .map(|row| score_from_row(row).map(|score| (score.record_id, score)))
            .collect()
    }

    pub(crate) async fn save_score(
        &self,
        tx: &mut MySqlTx,
        score: &RecordScore,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO record_scores (model_id, record_type, record_id, score, \
             band, explanation, calculated_at) VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE score = VALUES(score), band = VALUES(band), \
             explanation = VALUES(explanation), calculated_at = VALUES(calculated_at)",
        )
        .bind(score.model_id.to_string())
        .bind(&score.record_type)
        .bind(score.record_id.to_string())
        .bind(score.score)
        .bind(score.band.as_str())
        .bind(Json(&score.explanation))
        .bind(naive(score.calculated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_scores(
        &self,
        tx: &mut MySqlTx,
        model_id: Uuid,
        record_ids: &[Uuid],
    ) -> Result<(), AppError> {
        if record_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new("DELETE FROM record_scores WHERE ");
        builder
            .push("model_id = ")
            .push_bind(model_id.to_string())
            .push(" AND record_id IN ");
        push_ids(&mut builder, record_ids);
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn delete_record_scores(&self, record: &RecordRef) -> Result<(), AppError> {
        sqlx::query("DELETE FROM record_scores WHERE record_type = ? AND record_id = ?")
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_record_signals(&self, record: &RecordRef) -> Result<(), AppError> {
        sqlx::query("DELETE FROM scoring_signals WHERE record_type = ? AND record_id = ?")
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn record_scores(
        &self,
        record: &RecordRef,
    ) -> Result<Vec<RecordScore>, AppError> {
        let rows = sqlx::query(concat!(
            "SELECT ",
            score_columns!(),
            " FROM record_scores WHERE record_type = ? AND record_id = ?"
        ))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(score_from_row).collect()
    }
}
