use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::call::{call_columns_sql, call_from_row};
use crate::domain::{
    ActivityCallView, CALL_ACTIVITY_TYPE_KEY, CallActivityType, CallDetails, CallOutcome,
    CallReportFilter, CallReportRow, CallSession, CallSessionItem, CallSessionStatus,
    SessionCounts, SessionItemStatus,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct CallStore {
    pool: MySqlPool,
}

impl CallStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl CallStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        activity_id: Uuid,
        details: &CallDetails,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO activity_calls (activity_id, direction, outcome_key, \
             duration_seconds, phone_number, recording_url, provider, provider_call_id, session_id, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(activity_id.to_string())
        .bind(details.direction.as_str())
        .bind(&details.outcome_key)
        .bind(details.duration_seconds)
        .bind(&details.phone_number)
        .bind(&details.recording_url)
        .bind(&details.provider)
        .bind(&details.provider_call_id)
        .bind(details.session_id.map(|id| id.to_string()))
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        activity_id: Uuid,
        details: &CallDetails,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE activity_calls SET direction = ?, outcome_key = ?, duration_seconds = ?, \
             phone_number = ?, recording_url = ?, updated_at = ? \
             WHERE activity_id = ?",
        )
        .bind(details.direction.as_str())
        .bind(&details.outcome_key)
        .bind(details.duration_seconds)
        .bind(&details.phone_number)
        .bind(&details.recording_url)
        .bind(mapper::naive(now))
        .bind(activity_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find(
        &self,
        activity_id: Uuid,
    ) -> Result<Option<ActivityCallView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {columns} FROM activity_calls a_call LEFT JOIN call_outcomes a_outcome ON a_outcome.`key` = a_call.outcome_key WHERE a_call.activity_id = ?",
            columns = call_columns_sql("a"),
        )))
        .bind(activity_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => call_from_row(&row),
            None => Ok(None),
        }
    }

    pub(crate) async fn find_many(
        &self,
        activity_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, ActivityCallView>, AppError> {
        if activity_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT a_call.activity_id, {columns} FROM activity_calls a_call LEFT JOIN call_outcomes a_outcome ON a_outcome.`key` = a_call.outcome_key WHERE a_call.activity_id IN (",
            columns = call_columns_sql("a"),
        ));
        let mut ids = builder.separated(", ");
        for id in activity_ids {
            ids.push_bind(id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut calls = HashMap::new();
        for row in &rows {
            if let Some(call) = call_from_row(row)? {
                calls.insert(uuid_column(row, "activity_id")?, call);
            }
        }
        Ok(calls)
    }

    pub(crate) async fn find_by_provider(
        &self,
        provider: &str,
        provider_call_id: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT activity_id FROM activity_calls WHERE provider = ? AND provider_call_id = ?",
        )
        .bind(provider)
        .bind(provider_call_id)
        .fetch_optional(&self.pool)
        .await?;
        id.map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("activity_id", &id)))
            .transpose()
    }

    pub(crate) async fn report(
        &self,
        ctx: &Ctx<'_>,
        filter: &CallReportFilter,
    ) -> Result<Vec<CallReportRow>, AppError> {
        let source = ctx
            .services()
            .require::<dyn aphel_sdk::tasks::TaskReportSubjects>()?;
        let mut after = None;
        let mut groups = HashMap::<(Option<String>, String), CallReportRow>::new();
        loop {
            let page = source
                .page(
                    ctx,
                    aphel_sdk::tasks::TaskReportQuery {
                        from: filter.from,
                        to: filter.to,
                        owner_id: filter.owner_id.clone(),
                        after,
                    },
                )
                .await?;
            if !page.rows.is_empty() {
                let mut subjects: HashMap<Uuid, _> = page
                    .rows
                    .into_iter()
                    .map(|subject| (subject.id, subject.user))
                    .collect();
                let mut query = QueryBuilder::<MySql>::new(
                    "SELECT activity_id, outcome_key, duration_seconds FROM activity_calls WHERE activity_id IN (",
                );
                let mut ids = query.separated(", ");
                for id in subjects.keys() {
                    ids.push_bind(id.to_string());
                }
                query.push(")");
                for row in query.build().fetch_all(&self.pool).await? {
                    let id = uuid_column(&row, "activity_id")?;
                    if let Some(user) = subjects.remove(&id) {
                        let outcome_key: String = row.try_get("outcome_key")?;
                        let key = (
                            user.as_ref().map(|user| user.id.as_str().to_owned()),
                            outcome_key.clone(),
                        );
                        let group = groups.entry(key).or_insert(CallReportRow {
                            user,
                            outcome_key,
                            calls: 0,
                            duration_seconds: 0,
                        });
                        group.calls = group.calls.saturating_add(1);
                        group.duration_seconds = group.duration_seconds.saturating_add(u64::from(
                            row.try_get::<Option<u32>, _>("duration_seconds")?
                                .unwrap_or_default(),
                        ));
                    }
                }
            }
            if !page.more {
                break;
            }
            if page.after.is_none() || page.after <= after {
                return Err(AppError::Configuration(
                    "task report source did not advance its cursor".into(),
                ));
            }
            after = page.after;
        }
        Ok(groups.into_values().collect())
    }
}

const OUTCOME_COLUMNS: &str =
    "id, `key`, name, is_connected, is_system, is_active, position, created_at, updated_at";

#[derive(Clone)]
pub(crate) struct CallOutcomeStore {
    pool: MySqlPool,
}

impl CallOutcomeStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn outcome_from_row(row: &MySqlRow) -> Result<CallOutcome, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(CallOutcome {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        is_connected: row.try_get("is_connected")?,
        is_system: row.try_get("is_system")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

impl CallOutcomeStore {
    pub(crate) async fn list(&self, include_inactive: bool) -> Result<Vec<CallOutcome>, AppError> {
        let active = if include_inactive {
            ""
        } else {
            " AND is_active = TRUE"
        };
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {OUTCOME_COLUMNS} FROM call_outcomes WHERE TRUE{active} \
             ORDER BY position, created_at"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(outcome_from_row).collect()
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<CallOutcome>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {OUTCOME_COLUMNS} FROM call_outcomes WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(outcome_from_row)
        .transpose()
    }

    pub(crate) async fn find_by_key(&self, key: &str) -> Result<Option<CallOutcome>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {OUTCOME_COLUMNS} FROM call_outcomes WHERE `key` = ?"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(outcome_from_row)
        .transpose()
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        outcome: &CallOutcome,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO call_outcomes (id, `key`, name, is_connected, is_system, \
             is_active, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(outcome.id.to_string())
        .bind(&outcome.key)
        .bind(&outcome.name)
        .bind(outcome.is_connected)
        .bind(outcome.is_system)
        .bind(outcome.is_active)
        .bind(outcome.position)
        .bind(mapper::naive(outcome.created_at))
        .bind(mapper::naive(outcome.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        outcome: &CallOutcome,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE call_outcomes SET name = ?, is_connected = ?, is_active = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&outcome.name)
        .bind(outcome.is_connected)
        .bind(outcome.is_active)
        .bind(mapper::naive(outcome.updated_at))
        .bind(outcome.id.to_string())
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
                "UPDATE call_outcomes SET position = ?, updated_at = ? \
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

    pub(crate) async fn call_activity_type(
        &self,
        ctx: &Ctx<'_>,
    ) -> Result<Option<CallActivityType>, AppError> {
        Ok(aphel_sdk::tasks::transactions()?
            .type_for_creation(ctx, CALL_ACTIVITY_TYPE_KEY)
            .await?
            .map(|view| CallActivityType {
                id: view.id,
                name: view.name,
            }))
    }
}

const SESSION_COLUMNS: &str = "id, user_id, view_id, name, script, status, position, total, \
     completed_at, created_at, updated_at";

const ITEM_SELECT: &str = "SELECT i.position, i.contact_id, NULL AS contact_name, i.status, \
     i.activity_id, i.outcome_key, i.handled_at FROM call_session_items i \
";

#[derive(Clone)]
pub(crate) struct CallSessionStore {
    pool: MySqlPool,
}

impl CallSessionStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value)))
        .transpose()
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    let value: Option<NaiveDateTime> = row.try_get(column)?;
    Ok(value.map(mapper::utc))
}

fn session_from_row(row: &MySqlRow) -> Result<CallSession, AppError> {
    let status: String = row.try_get("status")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(CallSession {
        id: uuid_column(row, "id")?,
        user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
        view_id: optional_uuid(row, "view_id")?,
        name: row.try_get("name")?,
        script: row.try_get("script")?,
        status: CallSessionStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        position: row.try_get("position")?,
        total: row.try_get("total")?,
        completed_at: optional_instant(row, "completed_at")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn item_from_row(row: &MySqlRow) -> Result<CallSessionItem, AppError> {
    let status: String = row.try_get("status")?;
    Ok(CallSessionItem {
        position: row.try_get("position")?,
        contact_id: uuid_column(row, "contact_id")?,
        contact_name: row.try_get("contact_name")?,
        status: SessionItemStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        activity_id: optional_uuid(row, "activity_id")?,
        outcome_key: row.try_get("outcome_key")?,
        handled_at: optional_instant(row, "handled_at")?,
    })
}

impl CallSessionStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        session: &CallSession,
        contact_ids: &[Uuid],
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO call_sessions (id, user_id, view_id, name, script, status, \
             position, total, completed_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.to_string())
        .bind(session.user_id.as_str())
        .bind(session.view_id.map(|id| id.to_string()))
        .bind(&session.name)
        .bind(&session.script)
        .bind(session.status.as_str())
        .bind(session.position)
        .bind(session.total)
        .bind(session.completed_at.map(mapper::naive))
        .bind(mapper::naive(session.created_at))
        .bind(mapper::naive(session.updated_at))
        .execute(&mut **tx)
        .await?;
        for chunk in contact_ids.chunks(200).enumerate() {
            let (chunk_index, ids) = chunk;
            let mut builder = QueryBuilder::new(
                "INSERT INTO call_session_items (session_id, position, contact_id, status) ",
            );
            builder.push_values(ids.iter().enumerate(), |mut values, (index, id)| {
                values
                    .push_bind(session.id.to_string())
                    .push_bind((chunk_index * 200 + index) as u32)
                    .push_bind(id.to_string())
                    .push_bind(SessionItemStatus::Pending.as_str());
            });
            builder.build().execute(&mut **tx).await?;
        }
        Ok(())
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<CallSession>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SESSION_COLUMNS} FROM call_sessions WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(session_from_row)
        .transpose()
    }

    pub(crate) async fn list(
        &self,
        scope: &AccessScope,
        active_only: bool,
        window: ListWindow,
    ) -> Result<Listing<CallSession>, AppError> {
        let status = if active_only {
            " AND status = 'active'"
        } else {
            ""
        };
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM call_sessions WHERE user_id = ?{status}"
        )))
        .bind(scope.user_id().as_str())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SESSION_COLUMNS} FROM call_sessions WHERE user_id = ?{status} \
             ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
        )))
        .bind(scope.user_id().as_str())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(session_from_row)
                .collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn items(
        &self,
        session_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<CallSessionItem>, AppError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM call_session_items WHERE session_id = ?")
                .bind(session_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{ITEM_SELECT}WHERE i.session_id = ? \
             ORDER BY i.position LIMIT ? OFFSET ?"
        )))
        .bind(session_id.to_string())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows.iter().map(item_from_row).collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn item(
        &self,
        session_id: Uuid,
        position: u32,
    ) -> Result<Option<CallSessionItem>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "{ITEM_SELECT}WHERE i.session_id = ? AND i.position = ?"
        )))
        .bind(session_id.to_string())
        .bind(position)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(item_from_row)
        .transpose()
    }

    pub(crate) async fn counts(&self, session_id: Uuid) -> Result<SessionCounts, AppError> {
        let rows = sqlx::query(
            "SELECT status, COUNT(*) AS total FROM call_session_items \
             WHERE session_id = ? GROUP BY status",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut counts = SessionCounts::default();
        for row in rows {
            let status: String = row.try_get("status")?;
            let total: i64 = row.try_get("total")?;
            let total = total.max(0) as u32;
            match SessionItemStatus::parse(&status) {
                Some(SessionItemStatus::Pending) => counts.pending = total,
                Some(SessionItemStatus::Logged) => counts.logged = total,
                Some(SessionItemStatus::Skipped) => counts.skipped = total,
                None => {}
            }
        }
        Ok(counts)
    }

    pub(crate) async fn lock(
        &self,
        tx: &mut MySqlTx,
        session_id: Uuid,
    ) -> Result<Option<CallSession>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SESSION_COLUMNS} FROM call_sessions WHERE id = ? FOR UPDATE"
        )))
        .bind(session_id.to_string())
        .fetch_optional(&mut **tx)
        .await?
        .as_ref()
        .map(session_from_row)
        .transpose()
    }

    pub(crate) async fn mark_item(
        &self,
        tx: &mut MySqlTx,
        session_id: Uuid,
        position: u32,
        status: SessionItemStatus,
        call: Option<(Uuid, &str)>,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let (activity_id, outcome_key) = match call {
            Some((activity_id, outcome_key)) => {
                (Some(activity_id.to_string()), Some(outcome_key.to_owned()))
            }
            None => (None, None),
        };
        sqlx::query(
            "UPDATE call_session_items SET status = ?, activity_id = COALESCE(?, activity_id), \
             outcome_key = COALESCE(?, outcome_key), handled_at = ? \
             WHERE session_id = ? AND position = ?",
        )
        .bind(status.as_str())
        .bind(activity_id)
        .bind(outcome_key)
        .bind(mapper::naive(now))
        .bind(session_id.to_string())
        .bind(position)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn next_pending(
        &self,
        tx: &mut MySqlTx,
        session_id: Uuid,
        after: u32,
    ) -> Result<Option<u32>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT position FROM call_session_items \
             WHERE session_id = ? AND status = 'pending' \
             ORDER BY position > ? DESC, position LIMIT 1",
        )
        .bind(session_id.to_string())
        .bind(after)
        .fetch_optional(&mut **tx)
        .await?)
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        session: &CallSession,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE call_sessions SET status = ?, position = ?, completed_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(session.status.as_str())
        .bind(session.position)
        .bind(session.completed_at.map(mapper::naive))
        .bind(mapper::naive(session.updated_at))
        .bind(session.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
