use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlConnection, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    DEFAULT_DISQUALIFICATION_REASONS, DealOutcomes, DealParties, DisqualificationCount,
    DisqualificationReason, HistorySpan, LIFECYCLE_STAGE_FIELD, LeadDisqualification,
    LifecycleSettings, ReportWindow, StageChange, StageHistoryEntry, StageOption, TransitionRule,
    TransitionTrigger,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{ArchivedFilter, QueryPlan};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::compiler::push_record_ids_subquery;
use aphel_kernel::infrastructure::repositories::mapper;

const REASON_COLUMNS: &str = "id, `key`, name, is_active, position, created_at, updated_at";

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|text| {
            Uuid::parse_str(&text)
                .map_err(|_| AppError::Internal(format!("invalid uuid in column {column}")))
        })
        .transpose()
}

fn extension_for(record_type: &RecordType) -> Result<&'static str, AppError> {
    if *record_type == RecordType::CONTACT {
        Ok("leads_contact_extension")
    } else if *record_type == RecordType::COMPANY {
        Ok("leads_company_extension")
    } else {
        Err(AppError::Validation(
            "record has no lifecycle extension".to_owned(),
        ))
    }
}

#[derive(Clone)]
pub(crate) struct HistoryStore {
    pool: MySqlPool,
}

impl HistoryStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl HistoryStore {
    pub(crate) async fn record(
        &self,
        tx: &mut MySqlTx,
        change: &StageChange,
    ) -> Result<(), AppError> {
        let at = mapper::naive(change.at);
        sqlx::query(
            "UPDATE lifecycle_stage_history SET left_at = ? \
             WHERE record_type = ? AND record_id = ? AND left_at IS NULL",
        )
        .bind(at)
        .bind(change.record_type.as_str())
        .bind(change.record_id.to_string())
        .execute(&mut **tx)
        .await?;
        let Some(to_stage) = &change.to_stage else {
            return Ok(());
        };
        sqlx::query(
            "INSERT INTO lifecycle_stage_history (id, record_type, record_id, \
             from_stage, to_stage, entered_at, actor_type, actor_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(change.record_type.as_str())
        .bind(change.record_id.to_string())
        .bind(change.from_stage.as_deref())
        .bind(to_stage)
        .bind(at)
        .bind(&change.actor_type)
        .bind(change.actor_id.as_deref())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn list(
        &self,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<Vec<StageHistoryEntry>, AppError> {
        let rows = sqlx::query(
            "SELECT h.id, h.from_stage, h.to_stage, h.entered_at, h.left_at, h.actor_type, \
             h.actor_id, u.display_name AS actor_name \
             FROM lifecycle_stage_history h \
             LEFT JOIN users u ON h.actor_type = 'user' AND u.id = h.actor_id \
             WHERE h.record_type = ? AND h.record_id = ? \
             ORDER BY h.entered_at DESC, h.id DESC LIMIT 200",
        )
        .bind(record_type.as_str())
        .bind(record_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let entered_at: NaiveDateTime = row.try_get("entered_at")?;
                let left_at: Option<NaiveDateTime> = row.try_get("left_at")?;
                Ok(StageHistoryEntry {
                    id: uuid_column(row, "id")?,
                    from_stage: row.try_get("from_stage")?,
                    to_stage: row.try_get("to_stage")?,
                    entered_at: mapper::utc(entered_at),
                    left_at: left_at.map(mapper::utc),
                    actor_type: row.try_get("actor_type")?,
                    actor_id: row.try_get("actor_id")?,
                    actor_name: row.try_get("actor_name")?,
                })
            })
            .collect()
    }
}

#[derive(Clone)]
pub(crate) struct SettingsStore {
    pool: MySqlPool,
}

impl SettingsStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn rules_json(rules: &[TransitionRule]) -> Value {
    json!(
        rules
            .iter()
            .map(|rule| json!({
                "trigger": rule.trigger.as_str(),
                "enabled": rule.enabled,
                "target_stage": rule.target_stage,
                "from_stages": rule.from_stages,
                "from_empty": rule.from_empty,
            }))
            .collect::<Vec<_>>()
    )
}

fn rules_from_json(value: &Value) -> Vec<TransitionRule> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let trigger = TransitionTrigger::parse(item.get("trigger")?.as_str()?)?;
                    let default = TransitionRule::default_for(trigger);
                    Some(TransitionRule {
                        trigger,
                        enabled: item
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(default.enabled),
                        target_stage: item
                            .get("target_stage")
                            .and_then(Value::as_str)
                            .map_or(default.target_stage, str::to_owned),
                        from_stages: item.get("from_stages").and_then(Value::as_array).map_or(
                            default.from_stages,
                            |stages| {
                                stages
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_owned)
                                    .collect()
                            },
                        ),
                        from_empty: item
                            .get("from_empty")
                            .and_then(Value::as_bool)
                            .unwrap_or(default.from_empty),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

impl SettingsStore {
    pub(crate) async fn get(&self) -> Result<LifecycleSettings, AppError> {
        let row = sqlx::query(
            "SELECT rules, disqualified_stage, updated_at FROM lifecycle_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(LifecycleSettings::default());
        };
        let rules: Value = row.try_get("rules")?;
        let updated_at: NaiveDateTime = row.try_get("updated_at")?;
        Ok(LifecycleSettings {
            rules: rules_from_json(&rules),
            disqualified_stage: row.try_get("disqualified_stage")?,
            updated_at: Some(mapper::utc(updated_at)),
        }
        .normalized())
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        settings: &LifecycleSettings,
        updated_by: Option<&UserId>,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO lifecycle_settings (id, rules, disqualified_stage, updated_by, \
             created_at, updated_at) VALUES (1, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE rules = VALUES(rules), \
             disqualified_stage = VALUES(disqualified_stage), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(rules_json(&settings.rules))
        .bind(settings.disqualified_stage.as_deref())
        .bind(updated_by.map(UserId::as_str))
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct ReasonStore {
    pool: MySqlPool,
}

impl ReasonStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn reason_from_row(row: &MySqlRow) -> Result<DisqualificationReason, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(DisqualificationReason {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

pub(crate) async fn seed_disqualification_reasons(
    connection: &mut MySqlConnection,
) -> Result<(), AppError> {
    let mut builder = QueryBuilder::new(
        "INSERT IGNORE INTO lead_disqualification_reasons (id, `key`, name, \
         is_active, position) ",
    );
    builder.push_values(
        DEFAULT_DISQUALIFICATION_REASONS.iter().enumerate(),
        |mut values, (position, reason)| {
            values
                .push_bind(Uuid::now_v7().to_string())
                .push_bind(reason.key)
                .push_bind(reason.name)
                .push_bind(true)
                .push_bind(position as i32);
        },
    );
    builder.build().execute(&mut *connection).await?;
    Ok(())
}

impl ReasonStore {
    pub(crate) async fn list(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<DisqualificationReason>, AppError> {
        let active = if include_inactive {
            ""
        } else {
            " WHERE is_active = TRUE"
        };
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {REASON_COLUMNS} FROM lead_disqualification_reasons \
             {active} ORDER BY position, created_at"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(reason_from_row).collect()
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<DisqualificationReason>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {REASON_COLUMNS} FROM lead_disqualification_reasons \
             WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(reason_from_row)
        .transpose()
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        reason: &DisqualificationReason,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO lead_disqualification_reasons (id, `key`, name, \
             is_active, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(reason.id.to_string())
        .bind(&reason.key)
        .bind(&reason.name)
        .bind(reason.is_active)
        .bind(reason.position)
        .bind(mapper::naive(reason.created_at))
        .bind(mapper::naive(reason.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        reason: &DisqualificationReason,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE lead_disqualification_reasons SET name = ?, is_active = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&reason.name)
        .bind(reason.is_active)
        .bind(mapper::naive(reason.updated_at))
        .bind(reason.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM lead_disqualification_reasons WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn reorder(
        &self,
        tx: &mut MySqlTx,
        ordered_ids: &[Uuid],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (position, id) in ordered_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE lead_disqualification_reasons SET position = ?, updated_at = ? \
                 WHERE id = ?",
            )
            .bind(position as i32)
            .bind(mapper::naive(now))
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn record_disqualification(
        &self,
        tx: &mut MySqlTx,
        disqualification: &LeadDisqualification,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO lead_disqualifications (id, contact_id, reason_id, \
             reason_name, note, from_stage, actor_type, actor_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(disqualification.id.to_string())
        .bind(disqualification.contact_id.to_string())
        .bind(disqualification.reason_id.map(|id| id.to_string()))
        .bind(&disqualification.reason_name)
        .bind(disqualification.note.as_deref())
        .bind(disqualification.from_stage.as_deref())
        .bind(&disqualification.actor_type)
        .bind(disqualification.actor_id.as_deref())
        .bind(mapper::naive(disqualification.at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct LeadRecords {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl LeadRecords {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    async fn visible_plan(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        visibility: &RecordScopeFilter,
        archived: ArchivedFilter,
    ) -> Result<QueryPlan, AppError> {
        let plan = QueryPlan {
            record_type: self.registry.require(record_type.as_str())?.clone(),
            visibility: visibility.clone(),
            filter: None,
            search: None,
            sort: Vec::new(),
            archived,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        Ok(
            aphel_kernel::records::constrain_query(&self.pool, scope, &plan)
                .await?
                .unwrap_or(plan),
        )
    }
}

impl LeadRecords {
    pub async fn stage_options(
        &self,
        record_type: &RecordType,
    ) -> Result<Vec<StageOption>, AppError> {
        let rows = sqlx::query(
            "SELECT o.value, o.label, o.color, o.position, o.is_active FROM field_options o \
             JOIN field_definitions d ON d.id = o.field_id \
             WHERE d.record_type = ? AND d.`key` = ? \
             ORDER BY o.position, o.created_at",
        )
        .bind(record_type.as_str())
        .bind(LIFECYCLE_STAGE_FIELD)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(StageOption {
                    value: row.try_get("value")?,
                    label: row.try_get("label")?,
                    color: row.try_get("color")?,
                    position: row.try_get("position")?,
                    is_active: row.try_get("is_active")?,
                })
            })
            .collect()
    }

    pub(crate) async fn visible(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        use aphel_kernel::application::query::{
            Filter, FilterOperator, RecordQuery, RecordQueryEngine,
        };
        let ports = aphel_kernel::infrastructure::query::MySqlRecordQueryPorts::new(
            self.pool.clone(),
            self.registry.clone(),
        );
        Ok(!RecordQueryEngine::new(&ports)
            .query(
                scope,
                record_type.as_str(),
                RecordQuery {
                    filter: Some(Filter::condition(
                        "id",
                        FilterOperator::Eq,
                        json!(record_id),
                    )),
                    archived: ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: 1,
                    },
                    ..RecordQuery::default()
                },
            )
            .await?
            .items
            .is_empty())
    }

    pub(crate) async fn current_stage(
        &self,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<Option<Option<String>>, AppError> {
        let scope = AccessScope::system();
        if !self.visible(&scope, record_type, record_id).await? {
            return Ok(None);
        }
        let extension = extension_for(record_type)?;
        let row: Option<(Option<String>,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT lifecycle_stage FROM `{extension}` WHERE record_id = ?"
        )))
        .bind(record_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(Some(row.and_then(|row| row.0)))
    }

    pub async fn current_stage_in(
        &self,
        tx: &mut MySqlTx,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<Option<Option<String>>, AppError> {
        if aphel_sdk::records::lock_entity_version(
            tx,
            self.registry.require(record_type.as_str())?,
            record_id,
        )
        .await?
        .is_none()
        {
            return Ok(None);
        }
        let extension = extension_for(record_type)?;
        let row: Option<(Option<String>,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT lifecycle_stage FROM `{extension}` WHERE record_id = ?"
        )))
        .bind(record_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        Ok(Some(row.and_then(|row| row.0)))
    }

    pub(crate) async fn deal_parties(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<DealParties>, AppError> {
        let app = aphel_kernel::modules::app();
        let Some(source) = app.services().get::<dyn aphel_sdk::deals::DealLifecycle>() else {
            return Ok(None);
        };
        source.parties(&self.pool, deal_id).await
    }

    pub(crate) async fn deal_outcomes(
        &self,
        record_type: &RecordType,
        record_id: Uuid,
    ) -> Result<Option<DealOutcomes>, AppError> {
        let app = aphel_kernel::modules::app();
        let Some(source) = app.services().get::<dyn aphel_sdk::deals::DealLifecycle>() else {
            return Ok(None);
        };
        source
            .outcomes(&self.pool, record_type, record_id)
            .await
            .map(Some)
    }

    pub(crate) async fn report_spans(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        visibility: &RecordScopeFilter,
        since: DateTime<Utc>,
    ) -> Result<Vec<HistorySpan>, AppError> {
        let plan = self
            .visible_plan(scope, record_type, visibility, ArchivedFilter::Include)
            .await?;
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT h.record_id,h.to_stage,h.entered_at,h.left_at FROM lifecycle_stage_history h WHERE h.record_type = ",
        );
        builder
            .push_bind(record_type.as_str().to_owned())
            .push(" AND h.entered_at >= ")
            .push_bind(mapper::naive(since))
            .push(" AND h.record_id IN (");
        push_record_ids_subquery(&mut builder, scope, &plan)?;
        builder.push(") ORDER BY h.record_id,h.entered_at");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let entered_at: NaiveDateTime = row.try_get("entered_at")?;
                let left_at: Option<NaiveDateTime> = row.try_get("left_at")?;
                Ok(HistorySpan {
                    record_id: uuid_column(row, "record_id")?,
                    to_stage: row.try_get("to_stage")?,
                    entered_at: mapper::utc(entered_at),
                    left_at: left_at.map(mapper::utc),
                })
            })
            .collect()
    }

    pub(crate) async fn current_counts(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        visibility: &RecordScopeFilter,
    ) -> Result<Vec<(String, u64)>, AppError> {
        let plan = self
            .visible_plan(scope, record_type, visibility, ArchivedFilter::Exclude)
            .await?;
        let extension = extension_for(record_type)?;
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT e.lifecycle_stage AS stage,COUNT(*) AS total FROM `{extension}` e WHERE e.lifecycle_stage IS NOT NULL AND e.record_id IN ("
        ));
        push_record_ids_subquery(&mut builder, scope, &plan)?;
        builder.push(") GROUP BY e.lifecycle_stage");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let total: i64 = row.try_get("total")?;
                Ok((row.try_get("stage")?, total.max(0) as u64))
            })
            .collect()
    }

    pub(crate) async fn disqualification_counts(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        window: &ReportWindow,
    ) -> Result<Vec<DisqualificationCount>, AppError> {
        let plan = self
            .visible_plan(
                scope,
                &RecordType::CONTACT,
                visibility,
                ArchivedFilter::Include,
            )
            .await?;
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT q.reason_id,q.reason_name,COUNT(*) AS total FROM lead_disqualifications q WHERE q.created_at >= ",
        );
        builder
            .push_bind(mapper::naive(window.start()))
            .push(" AND q.created_at < ")
            .push_bind(mapper::naive(window.end()))
            .push(" AND q.contact_id IN (");
        push_record_ids_subquery(&mut builder, scope, &plan)?;
        builder.push(") GROUP BY q.reason_id,q.reason_name ORDER BY total DESC,q.reason_name");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let total: i64 = row.try_get("total")?;
                Ok(DisqualificationCount {
                    reason_id: optional_uuid(row, "reason_id")?,
                    reason_name: row.try_get("reason_name")?,
                    count: total.max(0) as u64,
                })
            })
            .collect()
    }
}
