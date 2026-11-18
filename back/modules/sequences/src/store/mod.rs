pub(crate) mod config_references;
pub(crate) mod reference_cleanup;

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use sqlx::mysql::MySqlRow;
use sqlx::{AssertSqlSafe, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    EnrollmentSource, EnrollmentStatus, ExitReason, RunStatus, SendWindow, StepKind,
};
use crate::model::{
    ContactTarget, DueEnrollment, EnrollmentCounts, EnrollmentFilter, EnrollmentView,
    NewEnrollment, SequenceListQuery, SequenceStep, SequenceView, StepMetrics, StepRun,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;

fn uuid(value: String, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn opt_uuid(value: Option<String>, column: &str) -> Result<Option<Uuid>, AppError> {
    value.map(|value| uuid(value, column)).transpose()
}

fn opt_user(value: Option<String>) -> Result<Option<UserId>, AppError> {
    value
        .map(|value| mapper::typed_id(value, UserId::new))
        .transpose()
}

fn opt_time(value: Option<NaiveDateTime>) -> Option<DateTime<Utc>> {
    value.map(mapper::utc)
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

fn push_visibility(
    builder: &mut QueryBuilder<MySql>,
    visibility: &RecordScopeFilter,
    column: &str,
) {
    match visibility {
        RecordScopeFilter::Organization => {}
        RecordScopeFilter::Owner { user_id, .. } => {
            builder
                .push(format!(" AND {column} = "))
                .push_bind(user_id.as_str().to_owned());
        }
        RecordScopeFilter::Teams {
            user_id, team_ids, ..
        } => {
            builder
                .push(format!(" AND ({column} = "))
                .push_bind(user_id.as_str().to_owned());
            if !team_ids.is_empty() {
                builder.push(format!(
                    " OR {column} IN (SELECT tm.user_id FROM team_memberships tm WHERE tm.team_id IN ("
                ));
                let mut separated = builder.separated(", ");
                for team in team_ids {
                    separated.push_bind(team.as_str().to_owned());
                }
                builder.push("))");
            }
            builder.push(")");
        }
    }
}

const SEQUENCE_COLUMNS: &str = "id, `key`, name, description, is_active, window_start_minute, \
     window_end_minute, daily_limit, created_by, updated_by, created_at, updated_at";

fn sequence_from_row(row: &MySqlRow) -> Result<SequenceView, AppError> {
    let start: u16 = row.try_get("window_start_minute")?;
    let end: u16 = row.try_get("window_end_minute")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(SequenceView {
        id: uuid(row.try_get("id")?, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        is_active: row.try_get("is_active")?,
        window: SendWindow::new(start, end).unwrap_or_default(),
        daily_limit: row.try_get("daily_limit")?,
        steps: Vec::new(),
        created_by: opt_user(row.try_get("created_by")?)?,
        updated_by: opt_user(row.try_get("updated_by")?)?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn step_from_row(row: &MySqlRow) -> Result<(Uuid, SequenceStep), AppError> {
    let kind: String = row.try_get("kind")?;
    Ok((
        uuid(row.try_get("sequence_id")?, "sequence_id")?,
        SequenceStep {
            id: uuid(row.try_get("id")?, "id")?,
            position: row.try_get("position")?,
            kind: StepKind::parse(&kind).ok_or_else(|| mapper::invalid_column("kind", &kind))?,
            wait_days: row.try_get("wait_days")?,
            template_id: opt_uuid(row.try_get("template_id")?, "template_id")?,
            subject: row.try_get("subject")?,
            body_html: row.try_get("body_html")?,
            title: row.try_get("title")?,
        },
    ))
}

#[derive(Clone)]
pub(crate) struct SequenceStore {
    pool: MySqlPool,
}

impl SequenceStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn with_steps(
        &self,
        mut sequences: Vec<SequenceView>,
    ) -> Result<Vec<SequenceView>, AppError> {
        if sequences.is_empty() {
            return Ok(sequences);
        }
        let ids: Vec<Uuid> = sequences.iter().map(|sequence| sequence.id).collect();
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id, sequence_id, position, kind, wait_days, template_id, subject, body_html, \
             title FROM sequence_steps WHERE sequence_id",
        );
        push_ids(&mut builder, &ids);
        builder.push(" ORDER BY sequence_id, position");
        let mut steps: HashMap<Uuid, Vec<SequenceStep>> = HashMap::new();
        for row in builder.build().fetch_all(&self.pool).await? {
            let (sequence_id, step) = step_from_row(&row)?;
            steps.entry(sequence_id).or_default().push(step);
        }
        for sequence in &mut sequences {
            sequence.steps = steps.remove(&sequence.id).unwrap_or_default();
        }
        Ok(sequences)
    }

    async fn write_steps(&self, tx: &mut Tx, sequence: &SequenceView) -> Result<(), AppError> {
        let mut builder =
            QueryBuilder::<MySql>::new("DELETE FROM sequence_steps WHERE sequence_id = ");
        builder.push_bind(sequence.id.to_string());
        if !sequence.steps.is_empty() {
            let ids: Vec<Uuid> = sequence.steps.iter().map(|step| step.id).collect();
            builder.push(" AND id NOT");
            push_ids(&mut builder, &ids);
        }
        builder.build().execute(&mut **tx).await?;
        sqlx::query("UPDATE sequence_steps SET position = position + 100000 WHERE sequence_id = ?")
            .bind(sequence.id.to_string())
            .execute(&mut **tx)
            .await?;
        if sequence.steps.is_empty() {
            return Ok(());
        }
        let now = mapper::naive(sequence.updated_at);
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO sequence_steps (id, sequence_id, position, kind, \
             wait_days, template_id, subject, body_html, title, created_at, updated_at) ",
        );
        builder.push_values(&sequence.steps, |mut row, step| {
            row.push_bind(step.id.to_string())
                .push_bind(sequence.id.to_string())
                .push_bind(step.position)
                .push_bind(step.kind.as_str())
                .push_bind(step.wait_days)
                .push_bind(step.template_id.map(|id| id.to_string()))
                .push_bind(step.subject.clone())
                .push_bind(step.body_html.clone())
                .push_bind(step.title.clone())
                .push_bind(now)
                .push_bind(now);
        });
        builder.push(
            " ON DUPLICATE KEY UPDATE position = VALUES(position), kind = VALUES(kind), \
             wait_days = VALUES(wait_days), template_id = VALUES(template_id), \
             subject = VALUES(subject), body_html = VALUES(body_html), title = VALUES(title), \
             updated_at = VALUES(updated_at)",
        );
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut Tx,
        sequence: &SequenceView,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO sequences (id, `key`, name, description, is_active, \
             window_start_minute, window_end_minute, daily_limit, created_by, updated_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(sequence.id.to_string())
        .bind(&sequence.key)
        .bind(&sequence.name)
        .bind(&sequence.description)
        .bind(sequence.is_active)
        .bind(sequence.window.start_minute)
        .bind(sequence.window.end_minute)
        .bind(sequence.daily_limit)
        .bind(sequence.created_by.as_ref().map(UserId::as_str))
        .bind(sequence.updated_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(sequence.created_at))
        .bind(mapper::naive(sequence.updated_at))
        .execute(&mut **tx)
        .await?;
        self.write_steps(tx, sequence).await
    }

    pub(crate) async fn update(
        &self,
        tx: &mut Tx,
        sequence: &SequenceView,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sequences SET `key` = ?, name = ?, description = ?, is_active = ?, \
             window_start_minute = ?, window_end_minute = ?, daily_limit = ?, updated_by = ?, \
             updated_at = ? WHERE id = ?",
        )
        .bind(&sequence.key)
        .bind(&sequence.name)
        .bind(&sequence.description)
        .bind(sequence.is_active)
        .bind(sequence.window.start_minute)
        .bind(sequence.window.end_minute)
        .bind(sequence.daily_limit)
        .bind(sequence.updated_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(sequence.updated_at))
        .bind(sequence.id.to_string())
        .execute(&mut **tx)
        .await?;
        self.write_steps(tx, sequence).await
    }

    pub(crate) async fn delete(&self, tx: &mut Tx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM sequences WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<SequenceView>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!(
            "SELECT {SEQUENCE_COLUMNS} FROM sequences WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(self
            .with_steps(vec![sequence_from_row(&row)?])
            .await?
            .into_iter()
            .next())
    }

    pub(crate) async fn list(
        &self,
        query: &SequenceListQuery,
        window: ListWindow,
    ) -> Result<Listing<SequenceView>, AppError> {
        let filters = |builder: &mut QueryBuilder<MySql>| {
            if let Some(active) = query.active {
                builder.push(" AND is_active = ").push_bind(active);
            }
            if let Some(search) = query
                .search
                .as_deref()
                .map(str::trim)
                .filter(|search| !search.is_empty())
            {
                builder
                    .push(" AND name LIKE ")
                    .push_bind(format!("%{}%", search.replace(['%', '_'], "")));
            }
        };
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM sequences WHERE TRUE");
        filters(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT {SEQUENCE_COLUMNS} FROM sequences WHERE TRUE"
        ));
        filters(&mut builder);
        builder
            .push(" ORDER BY name, id LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let sequences = builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(sequence_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items: self.with_steps(sequences).await?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn all(&self) -> Result<Vec<SequenceView>, AppError> {
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT {SEQUENCE_COLUMNS} FROM sequences ORDER BY `key`"
        )))
        .fetch_all(&self.pool)
        .await?;
        let sequences = rows
            .iter()
            .map(sequence_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.with_steps(sequences).await
    }

    pub(crate) async fn key_taken(
        &self,
        key: &str,
        except: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sequences WHERE `key` = ? \
             AND id <> ?)",
        )
        .bind(key)
        .bind(except.map(|id| id.to_string()).unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    pub(crate) async fn template_keys(&self) -> Result<HashMap<Uuid, String>, AppError> {
        match aphel_sdk::email::automation_reader() {
            Some(reader) => reader.organization_template_keys(&self.pool).await,
            None => Ok(HashMap::new()),
        }
    }

    pub(crate) async fn template_ids_in(
        &self,
        tx: &mut Tx,
    ) -> Result<HashMap<String, Uuid>, AppError> {
        match aphel_sdk::email::automation_reader() {
            Some(reader) => reader.organization_template_ids_in(tx).await,
            None => Ok(HashMap::new()),
        }
    }
}

const ENROLLMENT_SELECT: &str = "SELECT e.id, e.sequence_id, s.name AS sequence_name, s.`key` AS sequence_key, \
     e.record_type, e.contact_id, e.enrolled_by, \
     u.display_name AS enrolled_by_name, e.mailbox_id, e.source, e.status, e.current_position, \
     e.next_run_at, e.last_email_id, e.exit_reason, e.exit_position, e.paused_at, \
     e.completed_at, e.exited_at, e.created_at, e.updated_at \
     FROM sequence_enrollments e \
     JOIN sequences s ON s.id = e.sequence_id \
     LEFT JOIN users u ON u.id = e.enrolled_by \
     WHERE TRUE";

fn enrollment_from_row(row: &MySqlRow) -> Result<EnrollmentView, AppError> {
    let source: String = row.try_get("source")?;
    let status: String = row.try_get("status")?;
    let exit_reason: Option<String> = row.try_get("exit_reason")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(EnrollmentView {
        id: uuid(row.try_get("id")?, "id")?,
        sequence_id: uuid(row.try_get("sequence_id")?, "sequence_id")?,
        sequence_key: row.try_get("sequence_key")?,
        sequence_name: row.try_get("sequence_name")?,
        record_type: row.try_get("record_type")?,
        contact_id: uuid(row.try_get("contact_id")?, "contact_id")?,
        contact_name: String::new(),
        contact_email: None,
        enrolled_by: mapper::typed_id(row.try_get("enrolled_by")?, UserId::new)?,
        enrolled_by_name: row.try_get("enrolled_by_name")?,
        mailbox_id: opt_uuid(row.try_get("mailbox_id")?, "mailbox_id")?,
        source: EnrollmentSource::parse(&source)
            .ok_or_else(|| mapper::invalid_column("source", &source))?,
        status: EnrollmentStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        current_position: row.try_get("current_position")?,
        next_run_at: opt_time(row.try_get("next_run_at")?),
        last_email_id: opt_uuid(row.try_get("last_email_id")?, "last_email_id")?,
        exit_reason: exit_reason
            .map(|reason| {
                ExitReason::parse(&reason)
                    .ok_or_else(|| mapper::invalid_column("exit_reason", &reason))
            })
            .transpose()?,
        exit_position: row.try_get("exit_position")?,
        paused_at: opt_time(row.try_get("paused_at")?),
        completed_at: opt_time(row.try_get("completed_at")?),
        exited_at: opt_time(row.try_get("exited_at")?),
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn run_from_row(row: &MySqlRow) -> Result<StepRun, AppError> {
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    let executed_at: NaiveDateTime = row.try_get("executed_at")?;
    Ok(StepRun {
        id: uuid(row.try_get("id")?, "id")?,
        sequence_id: uuid(row.try_get("sequence_id")?, "sequence_id")?,
        enrollment_id: uuid(row.try_get("enrollment_id")?, "enrollment_id")?,
        step_id: opt_uuid(row.try_get("step_id")?, "step_id")?,
        position: row.try_get("position")?,
        kind: StepKind::parse(&kind).ok_or_else(|| mapper::invalid_column("kind", &kind))?,
        status: RunStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        contact_id: uuid(row.try_get("contact_id")?, "contact_id")?,
        mailbox_id: opt_uuid(row.try_get("mailbox_id")?, "mailbox_id")?,
        email_id: opt_uuid(row.try_get("email_id")?, "email_id")?,
        thread_id: opt_uuid(row.try_get("thread_id")?, "thread_id")?,
        activity_id: opt_uuid(row.try_get("activity_id")?, "activity_id")?,
        failure_reason: row.try_get("failure_reason")?,
        executed_at: mapper::utc(executed_at),
    })
}

fn count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[derive(Clone)]
pub(crate) struct EnrollmentStore {
    pool: MySqlPool,
}

impl EnrollmentStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn enrollments_of_runs(
        &self,
        column: &str,
        value: Uuid,
    ) -> Result<Vec<(Uuid, u32)>, AppError> {
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT enrollment_id, MAX(position) AS position FROM sequence_step_runs \
             WHERE {column} = ? AND status = 'sent' GROUP BY enrollment_id"
        )))
        .bind(value.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok((
                    uuid(row.try_get("enrollment_id")?, "enrollment_id")?,
                    row.try_get::<u32, _>("position")?,
                ))
            })
            .collect()
    }

    async fn mark_runs(
        &self,
        column: &str,
        runs: &[(Uuid, u32)],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (enrollment_id, position) in runs {
            sqlx::query(AssertSqlSafe(format!(
                "UPDATE sequence_step_runs SET {column} = ?, updated_at = ? \
                 WHERE enrollment_id = ? AND position = ? \
                 AND {column} IS NULL"
            )))
            .bind(mapper::naive(at))
            .bind(mapper::naive(Utc::now()))
            .bind(enrollment_id.to_string())
            .bind(position)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut Tx,
        enrollment: &NewEnrollment,
    ) -> Result<bool, AppError> {
        let now = mapper::naive(enrollment.next_run_at);
        let result = sqlx::query(
            "INSERT INTO sequence_enrollments (id, sequence_id, record_type, contact_id, \
             enrolled_by, mailbox_id, source, status, current_position, next_run_at, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', 0, ?, ?, ?)",
        )
        .bind(enrollment.id.to_string())
        .bind(enrollment.sequence_id.to_string())
        .bind(&enrollment.record_type)
        .bind(enrollment.contact_id.to_string())
        .bind(enrollment.enrolled_by.as_str())
        .bind(enrollment.mailbox_id.map(|id| id.to_string()))
        .bind(enrollment.source.as_str())
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<EnrollmentView>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!("{ENROLLMENT_SELECT} AND e.id = ?")))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(enrollment_from_row).transpose()
    }

    pub(crate) async fn lock(
        &self,
        tx: &mut Tx,
        id: Uuid,
    ) -> Result<Option<EnrollmentView>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!(
            "{ENROLLMENT_SELECT} AND e.id = ? FOR UPDATE OF e"
        )))
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        row.as_ref().map(enrollment_from_row).transpose()
    }

    pub(crate) async fn save(
        &self,
        tx: &mut Tx,
        enrollment: &EnrollmentView,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sequence_enrollments SET mailbox_id = ?, status = ?, current_position = ?, \
             next_run_at = ?, last_email_id = ?, exit_reason = ?, exit_position = ?, \
             paused_at = ?, completed_at = ?, exited_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(enrollment.mailbox_id.map(|id| id.to_string()))
        .bind(enrollment.status.as_str())
        .bind(enrollment.current_position)
        .bind(enrollment.next_run_at.map(mapper::naive))
        .bind(enrollment.last_email_id.map(|id| id.to_string()))
        .bind(enrollment.exit_reason.map(|reason| reason.as_str()))
        .bind(enrollment.exit_position)
        .bind(enrollment.paused_at.map(mapper::naive))
        .bind(enrollment.completed_at.map(mapper::naive))
        .bind(enrollment.exited_at.map(mapper::naive))
        .bind(mapper::naive(enrollment.updated_at))
        .bind(enrollment.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn list(
        &self,
        filter: &EnrollmentFilter,
        visibility: &RecordScopeFilter,
        window: ListWindow,
    ) -> Result<Listing<EnrollmentView>, AppError> {
        let filters = |builder: &mut QueryBuilder<MySql>| {
            if let Some(id) = filter.id {
                builder.push(" AND e.id = ").push_bind(id.to_string());
            }
            if let Some(sequence_id) = filter.sequence_id {
                builder
                    .push(" AND e.sequence_id = ")
                    .push_bind(sequence_id.to_string());
            }
            if let Some(contact_id) = filter.contact_id {
                builder
                    .push(" AND e.contact_id = ")
                    .push_bind(contact_id.to_string());
            }
            if let Some(status) = filter.status {
                builder.push(" AND e.status = ").push_bind(status.as_str());
            }
            push_visibility(builder, visibility, "e.enrolled_by");
        };
        let mut counter =
            QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM sequence_enrollments e WHERE TRUE");
        filters(&mut counter);
        let total: i64 = counter.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::<MySql>::new(ENROLLMENT_SELECT);
        filters(&mut builder);
        builder
            .push(" ORDER BY e.created_at DESC, e.id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let items = builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(enrollment_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items,
            total: count(total),
        })
    }

    pub(crate) async fn live_for_contacts(
        &self,
        contact_ids: &[Uuid],
    ) -> Result<Vec<EnrollmentView>, AppError> {
        if contact_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(ENROLLMENT_SELECT);
        builder.push(" AND e.status IN ('active', 'paused') AND e.contact_id");
        push_ids(&mut builder, contact_ids);
        builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(enrollment_from_row)
            .collect()
    }

    pub(crate) async fn live_contact_emails(
        &self,
        ctx: &Ctx<'_>,
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows: Vec<(String, String)> = sqlx::query_as("SELECT DISTINCT record_type, contact_id FROM sequence_enrollments WHERE status IN ('active', 'paused')").fetch_all(&self.pool).await?;
        let mut by_type: HashMap<String, Vec<Uuid>> = HashMap::new();
        for (record_type, id) in rows {
            by_type
                .entry(record_type)
                .or_default()
                .push(uuid(id, "contact_id")?);
        }
        let mut emails = Vec::new();
        for (record_type, ids) in by_type {
            emails.extend(
                crate::contacts::list(
                    ctx,
                    &record_type,
                    &ids,
                    aphel_sdk::platform::application::query::ArchivedFilter::Include,
                )
                .await?
                .into_values()
                .filter_map(|contact| contact.email.map(|email| (contact.id, email))),
            );
        }
        Ok(emails)
    }

    pub(crate) async fn counts(
        &self,
        sequence_ids: &[Uuid],
        visibility: &RecordScopeFilter,
    ) -> Result<HashMap<Uuid, EnrollmentCounts>, AppError> {
        let mut counts: HashMap<Uuid, EnrollmentCounts> = HashMap::new();
        if sequence_ids.is_empty() {
            return Ok(counts);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT e.sequence_id, e.status, COUNT(*) AS total FROM sequence_enrollments e \
             WHERE e.sequence_id",
        );
        push_ids(&mut builder, sequence_ids);
        push_visibility(&mut builder, visibility, "e.enrolled_by");
        builder.push(" GROUP BY e.sequence_id, e.status");
        for row in builder.build().fetch_all(&self.pool).await? {
            let sequence_id = uuid(row.try_get("sequence_id")?, "sequence_id")?;
            let status: String = row.try_get("status")?;
            let total = count(row.try_get("total")?);
            let entry = counts.entry(sequence_id).or_default();
            match EnrollmentStatus::parse(&status) {
                Some(EnrollmentStatus::Active) => entry.active += total,
                Some(EnrollmentStatus::Paused) => entry.paused += total,
                Some(EnrollmentStatus::Completed) => entry.completed += total,
                Some(EnrollmentStatus::Exited) => entry.exited += total,
                None => {}
            }
        }
        Ok(counts)
    }

    pub(crate) async fn due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueEnrollment>, AppError> {
        let rows = sqlx::query(
            "SELECT e.id, e.current_position FROM sequence_enrollments e \
             JOIN sequences s ON s.id = e.sequence_id \
             WHERE e.status = 'active' AND e.next_run_at <= ? AND s.is_active = 1 \
             ORDER BY e.next_run_at LIMIT ?",
        )
        .bind(mapper::naive(now))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(DueEnrollment {
                    enrollment_id: uuid(row.try_get("id")?, "id")?,
                    position: row.try_get("current_position")?,
                })
            })
            .collect()
    }

    pub(crate) async fn claim_run(&self, tx: &mut Tx, run: &StepRun) -> Result<bool, AppError> {
        let now = mapper::naive(run.executed_at);
        let result = sqlx::query(
            "INSERT INTO sequence_step_runs (id, sequence_id, enrollment_id, \
             step_id, position, kind, status, contact_id, mailbox_id, executed_at, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run.id.to_string())
        .bind(run.sequence_id.to_string())
        .bind(run.enrollment_id.to_string())
        .bind(run.step_id.map(|id| id.to_string()))
        .bind(run.position)
        .bind(run.kind.as_str())
        .bind(run.status.as_str())
        .bind(run.contact_id.to_string())
        .bind(run.mailbox_id.map(|id| id.to_string()))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn release_run(
        &self,
        tx: &mut Tx,
        enrollment_id: Uuid,
        position: u32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM sequence_step_runs WHERE enrollment_id = ? \
             AND position = ? AND status = 'running'",
        )
        .bind(enrollment_id.to_string())
        .bind(position)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find_run(
        &self,
        enrollment_id: Uuid,
        position: u32,
    ) -> Result<Option<StepRun>, AppError> {
        let row = sqlx::query(
            "SELECT id, sequence_id, enrollment_id, step_id, position, kind, status, contact_id, \
             mailbox_id, email_id, thread_id, activity_id, failure_reason, executed_at \
             FROM sequence_step_runs WHERE enrollment_id = ? \
             AND position = ?",
        )
        .bind(enrollment_id.to_string())
        .bind(position)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(run_from_row).transpose()
    }

    pub(crate) async fn save_run(&self, tx: &mut Tx, run: &StepRun) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sequence_step_runs SET status = ?, mailbox_id = ?, email_id = ?, \
             thread_id = ?, activity_id = ?, failure_reason = ?, updated_at = ? \
             WHERE enrollment_id = ? AND position = ?",
        )
        .bind(run.status.as_str())
        .bind(run.mailbox_id.map(|id| id.to_string()))
        .bind(run.email_id.map(|id| id.to_string()))
        .bind(run.thread_id.map(|id| id.to_string()))
        .bind(run.activity_id.map(|id| id.to_string()))
        .bind(&run.failure_reason)
        .bind(mapper::naive(Utc::now()))
        .bind(run.enrollment_id.to_string())
        .bind(run.position)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn emails_since(
        &self,
        contact_id: Option<Uuid>,
        mailbox_id: Option<Uuid>,
        since: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT COUNT(*) FROM sequence_step_runs WHERE kind = 'auto_email' AND status IN ('running', 'sent') AND executed_at >= ",
        );
        builder.push_bind(mapper::naive(since));
        if let Some(contact_id) = contact_id {
            builder
                .push(" AND contact_id = ")
                .push_bind(contact_id.to_string());
        }
        if let Some(mailbox_id) = mailbox_id {
            builder
                .push(" AND mailbox_id = ")
                .push_bind(mailbox_id.to_string());
        }
        let total: i64 = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count(total))
    }

    pub(crate) async fn mark_opened(
        &self,
        email_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sequence_step_runs SET opened_at = ?, updated_at = ? \
             WHERE email_id = ? AND opened_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(mapper::naive(Utc::now()))
        .bind(email_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_replied(
        &self,
        thread_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, AppError> {
        let runs = self.enrollments_of_runs("thread_id", thread_id).await?;
        self.mark_runs("replied_at", &runs, at).await?;
        Ok(runs
            .into_iter()
            .map(|(enrollment_id, _)| enrollment_id)
            .collect())
    }

    pub(crate) async fn mark_bounced(
        &self,
        email_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, AppError> {
        let runs = self.enrollments_of_runs("email_id", email_id).await?;
        self.mark_runs("bounced_at", &runs, at).await?;
        Ok(runs
            .into_iter()
            .map(|(enrollment_id, _)| enrollment_id)
            .collect())
    }

    pub(crate) async fn metrics(
        &self,
        sequence_id: Uuid,
        visibility: &RecordScopeFilter,
    ) -> Result<Vec<StepMetrics>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT r.position, COUNT(*) AS executed, \
             CAST(SUM(r.status IN ('sent', 'created')) AS SIGNED) AS sent, \
             CAST(SUM(r.opened_at IS NOT NULL) AS SIGNED) AS opened, \
             CAST(SUM(r.replied_at IS NOT NULL) AS SIGNED) AS replied, \
             CAST(SUM(r.bounced_at IS NOT NULL) AS SIGNED) AS bounced, \
             CAST(SUM(r.status = 'failed') AS SIGNED) AS failed \
             FROM sequence_step_runs r JOIN sequence_enrollments e ON e.id = r.enrollment_id \
             WHERE r.sequence_id = ",
        );
        builder.push_bind(sequence_id.to_string());
        push_visibility(&mut builder, visibility, "e.enrolled_by");
        builder.push(" GROUP BY r.position");
        let mut metrics: HashMap<u32, StepMetrics> = HashMap::new();
        for row in builder.build().fetch_all(&self.pool).await? {
            let position: u32 = row.try_get("position")?;
            metrics.insert(
                position,
                StepMetrics {
                    position,
                    executed: count(row.try_get("executed")?),
                    sent: count(row.try_get("sent")?),
                    opened: count(row.try_get("opened")?),
                    replied: count(row.try_get("replied")?),
                    bounced: count(row.try_get("bounced")?),
                    failed: count(row.try_get("failed")?),
                    ..StepMetrics::default()
                },
            );
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT e.exit_position, COUNT(*) AS total FROM sequence_enrollments e \
             WHERE e.sequence_id = ",
        );
        builder
            .push_bind(sequence_id.to_string())
            .push(" AND e.status = 'exited' AND e.exit_position IS NOT NULL");
        push_visibility(&mut builder, visibility, "e.enrolled_by");
        builder.push(" GROUP BY e.exit_position");
        for row in builder.build().fetch_all(&self.pool).await? {
            let position: u32 = row.try_get("exit_position")?;
            let entry = metrics.entry(position).or_insert_with(|| StepMetrics {
                position,
                ..StepMetrics::default()
            });
            entry.exits = count(row.try_get("total")?);
        }
        let mut metrics: Vec<StepMetrics> = metrics.into_values().collect();
        metrics.sort_by_key(|metrics| metrics.position);
        Ok(metrics)
    }
}

#[derive(Clone)]
pub(crate) struct DirectoryStore {
    pool: MySqlPool,
}

impl DirectoryStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn timezone(&self) -> Result<Tz, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(timezone
            .and_then(|timezone| timezone.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC))
    }

    pub(crate) async fn contact(
        &self,
        ctx: &Ctx<'_>,
        record_type: &str,
        contact_id: Uuid,
    ) -> Result<Option<ContactTarget>, AppError> {
        Ok(crate::contacts::list(
            ctx,
            record_type,
            &[contact_id],
            aphel_sdk::platform::application::query::ArchivedFilter::Exclude,
        )
        .await?
        .remove(&contact_id))
    }

    pub(crate) async fn deal_contacts(
        &self,
        ctx: &Ctx<'_>,
        deal_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        match ctx.services().get::<dyn aphel_sdk::deals::DealLifecycle>() {
            Some(source) => source.lifecycle_people(ctx, deal_id).await,
            None => Ok(Vec::new()),
        }
    }

    pub(crate) async fn objection_hash(
        &self,
        objection_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        match aphel_sdk::email::automation_reader() {
            Some(reader) => reader.objection_hash(&self.pool, objection_id).await,
            None => Ok(None),
        }
    }

    pub(crate) async fn has_objection(&self, address_hash: &str) -> Result<bool, AppError> {
        match aphel_sdk::email::automation_reader() {
            Some(reader) => reader.has_objection(&self.pool, address_hash).await,
            None => Err(AppError::unavailable_code(
                "email_consent_unavailable",
                "email consent checks are not enabled",
            )),
        }
    }

    pub(crate) async fn sendable_mailbox(
        &self,
        owner: &UserId,
        mailbox_id: Option<Uuid>,
    ) -> Result<Option<Uuid>, AppError> {
        match aphel_sdk::email::automation_reader() {
            Some(reader) => reader.sendable_mailbox(&self.pool, owner, mailbox_id).await,
            None => Ok(None),
        }
    }

    pub(crate) async fn is_active_member(&self, user: &UserId) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = ? AND status = 'active')",
        )
        .bind(user.as_str())
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    pub(crate) async fn member_scope(
        &self,
        user: &UserId,
    ) -> Result<Option<AccessScope>, AppError> {
        let access = MySqlAccessRepository::new(self.pool.clone())
            .load_access(user)
            .await?;
        Ok(access.map(|access| build_access_scope(user.clone(), access)))
    }
}
