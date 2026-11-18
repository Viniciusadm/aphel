use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::custom::{
    DeliveryStatus, DueSchedule, ReportDelivery, ReportRecipient, ReportSchedule,
    ScheduleFrequency, ScheduleTiming,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::users::UserStatus;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const SCHEDULE_COLUMNS: &str = "id, report_id, created_by, frequency, weekday, day_of_month, hour, minute, \
     period_preset, is_active, next_run_at, last_run_at, created_at, updated_at";

const DELIVERY_COLUMNS: &str = "id, schedule_id, report_id, user_id, scheduled_for, status, row_count, \
     failure_reason, created_at, finished_at";

#[derive(Clone)]
pub(crate) struct ScheduleStore {
    pool: MySqlPool,
}

impl ScheduleStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn schedule_from_row(row: &MySqlRow) -> Result<ReportSchedule, AppError> {
    let frequency: String = row.try_get("frequency")?;
    let next_run_at: NaiveDateTime = row.try_get("next_run_at")?;
    let last_run_at: Option<NaiveDateTime> = row.try_get("last_run_at")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(ReportSchedule {
        id: uuid_column(row, "id")?,
        report_id: uuid_column(row, "report_id")?,
        created_by: row
            .try_get::<Option<String>, _>("created_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        timing: ScheduleTiming {
            frequency: ScheduleFrequency::parse(&frequency)
                .ok_or_else(|| mapper::invalid_column("frequency", &frequency))?,
            weekday: row.try_get("weekday")?,
            day_of_month: row.try_get("day_of_month")?,
            hour: row.try_get("hour")?,
            minute: row.try_get("minute")?,
        },
        period_preset: row.try_get("period_preset")?,
        recipients: Vec::new(),
        is_active: row.try_get("is_active")?,
        next_run_at: mapper::utc(next_run_at),
        last_run_at: last_run_at.map(mapper::utc),
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn delivery_from_row(row: &MySqlRow) -> Result<ReportDelivery, AppError> {
    let status: String = row.try_get("status")?;
    let scheduled_for: NaiveDateTime = row.try_get("scheduled_for")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
    Ok(ReportDelivery {
        id: uuid_column(row, "id")?,
        schedule_id: uuid_column(row, "schedule_id")?,
        report_id: uuid_column(row, "report_id")?,
        user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
        scheduled_for: mapper::utc(scheduled_for),
        status: DeliveryStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        row_count: row.try_get("row_count")?,
        failure_reason: row.try_get("failure_reason")?,
        created_at: mapper::utc(created_at),
        finished_at: finished_at.map(mapper::utc),
    })
}

impl ScheduleStore {
    async fn attach_recipients<'e, E>(
        &self,
        executor: E,
        schedules: &mut [ReportSchedule],
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = MySql>,
    {
        if schedules.is_empty() {
            return Ok(());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT schedule_id, user_id FROM report_schedule_recipients WHERE schedule_id IN (",
        );
        let mut ids = builder.separated(", ");
        for schedule in schedules.iter() {
            ids.push_bind(schedule.id.to_string());
        }
        builder.push(") ORDER BY user_id");
        let rows = builder.build().fetch_all(executor).await?;
        let mut recipients: HashMap<Uuid, Vec<UserId>> = HashMap::new();
        for row in &rows {
            let schedule_id = uuid_column(row, "schedule_id")?;
            let user_id = mapper::typed_id(row.try_get("user_id")?, UserId::new)?;
            recipients.entry(schedule_id).or_default().push(user_id);
        }
        for schedule in schedules.iter_mut() {
            schedule.recipients = recipients.remove(&schedule.id).unwrap_or_default();
        }
        Ok(())
    }

    async fn write_recipients(
        &self,
        tx: &mut Tx,
        schedule: &ReportSchedule,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM report_schedule_recipients WHERE schedule_id = ?")
            .bind(schedule.id.to_string())
            .execute(&mut **tx)
            .await?;
        if schedule.recipients.is_empty() {
            return Ok(());
        }
        let mut builder: QueryBuilder<MySql> =
            QueryBuilder::new("INSERT INTO report_schedule_recipients (schedule_id, user_id) ");
        builder.push_values(&schedule.recipients, |mut row, user_id| {
            row.push_bind(schedule.id.to_string())
                .push_bind(user_id.as_str().to_owned());
        });
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }
}

impl ScheduleStore {
    pub(crate) async fn list_for_report(
        &self,
        report_id: Uuid,
    ) -> Result<Vec<ReportSchedule>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SCHEDULE_COLUMNS} FROM report_schedules \
             WHERE report_id = ? ORDER BY created_at, id"
        )))
        .bind(report_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut schedules = rows
            .iter()
            .map(schedule_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.attach_recipients(&self.pool, &mut schedules).await?;
        Ok(schedules)
    }

    pub(crate) async fn find(&self, schedule_id: Uuid) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SCHEDULE_COLUMNS} FROM report_schedules WHERE id = ?"
        )))
        .bind(schedule_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut schedules = vec![schedule_from_row(&row)?];
        self.attach_recipients(&self.pool, &mut schedules).await?;
        Ok(schedules.pop())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut Tx,
        schedule: &ReportSchedule,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO report_schedules (id, report_id, created_by, frequency, weekday, \
             day_of_month, hour, minute, period_preset, is_active, next_run_at, last_run_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(schedule.id.to_string())
        .bind(schedule.report_id.to_string())
        .bind(schedule.created_by.as_ref().map(UserId::as_str))
        .bind(schedule.timing.frequency.as_str())
        .bind(schedule.timing.weekday)
        .bind(schedule.timing.day_of_month)
        .bind(schedule.timing.hour)
        .bind(schedule.timing.minute)
        .bind(schedule.period_preset.as_deref())
        .bind(schedule.is_active)
        .bind(mapper::naive(schedule.next_run_at))
        .bind(schedule.last_run_at.map(mapper::naive))
        .bind(mapper::naive(schedule.created_at))
        .bind(mapper::naive(schedule.updated_at))
        .execute(&mut **tx)
        .await?;
        self.write_recipients(tx, schedule).await
    }

    pub(crate) async fn update(
        &self,
        tx: &mut Tx,
        schedule: &ReportSchedule,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE report_schedules SET frequency = ?, weekday = ?, day_of_month = ?, hour = ?, minute = ?, \
             period_preset = ?, is_active = ?, next_run_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(schedule.timing.frequency.as_str())
        .bind(schedule.timing.weekday)
        .bind(schedule.timing.day_of_month)
        .bind(schedule.timing.hour)
        .bind(schedule.timing.minute)
        .bind(schedule.period_preset.as_deref())
        .bind(schedule.is_active)
        .bind(mapper::naive(schedule.next_run_at))
        .bind(mapper::naive(schedule.updated_at))
        .bind(schedule.id.to_string())
        .execute(&mut **tx)
        .await?;
        self.write_recipients(tx, schedule).await
    }

    pub(crate) async fn delete(&self, tx: &mut Tx, schedule_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM report_schedules WHERE id = ?")
            .bind(schedule_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn members(
        &self,
        user_ids: &[UserId],
        only_active: bool,
    ) -> Result<Vec<UserReference>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> =
            QueryBuilder::new("SELECT u.id, u.display_name FROM users u WHERE u.id IN (");
        let mut ids = builder.separated(", ");
        for user_id in user_ids {
            ids.push_bind(user_id.as_str().to_owned());
        }
        builder.push(")");
        if only_active {
            builder
                .push(" AND u.status = ")
                .push_bind(UserStatus::Active.as_str());
        }
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(UserReference {
                    id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                    display_name: row.try_get("display_name")?,
                })
            })
            .collect()
    }

    pub(crate) async fn recipient(
        &self,
        user_id: &UserId,
    ) -> Result<Option<ReportRecipient>, AppError> {
        let row = sqlx::query(
            "SELECT u.id, u.display_name, u.email, COALESCE(u.locale, o.locale) AS locale FROM users u \
             LEFT JOIN organization_settings o ON o.id = 1 \
             WHERE u.id = ? AND u.status = ?",
        )
        .bind(user_id.as_str())
        .bind(UserStatus::Active.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(ReportRecipient {
                user_id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                display_name: row.try_get("display_name")?,
                email: row.try_get("email")?,
                locale: row.try_get("locale")?,
            })
        })
        .transpose()
    }

    pub(crate) async fn list_deliveries(
        &self,
        schedule_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<ReportDelivery>, AppError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM report_deliveries WHERE schedule_id = ?")
                .bind(schedule_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {DELIVERY_COLUMNS} FROM report_deliveries \
             WHERE schedule_id = ? \
             ORDER BY scheduled_for DESC, id DESC LIMIT ? OFFSET ?"
        )))
        .bind(schedule_id.to_string())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(delivery_from_row)
                .collect::<Result<Vec<_>, _>>()?,
            total: u64::try_from(total).unwrap_or(0),
        })
    }

    pub(crate) async fn find_delivery(
        &self,
        schedule_id: Uuid,
        user_id: &UserId,
        scheduled_for: DateTime<Utc>,
    ) -> Result<Option<ReportDelivery>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {DELIVERY_COLUMNS} FROM report_deliveries \
             WHERE schedule_id = ? AND user_id = ? AND scheduled_for = ?"
        )))
        .bind(schedule_id.to_string())
        .bind(user_id.as_str())
        .bind(mapper::naive(scheduled_for))
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(delivery_from_row).transpose()
    }

    pub(crate) async fn claim_delivery(
        &self,
        tx: &mut Tx,
        delivery: &ReportDelivery,
    ) -> Result<bool, AppError> {
        let current: Option<String> = sqlx::query_scalar(
            "SELECT status FROM report_deliveries \
             WHERE schedule_id = ? AND user_id = ? AND scheduled_for = ? FOR UPDATE",
        )
        .bind(delivery.schedule_id.to_string())
        .bind(delivery.user_id.as_str())
        .bind(mapper::naive(delivery.scheduled_for))
        .fetch_optional(&mut **tx)
        .await?;
        if current
            .as_deref()
            .and_then(DeliveryStatus::parse)
            .is_some_and(|status| status != DeliveryStatus::Failed)
        {
            return Ok(false);
        }
        self.save_delivery(tx, delivery).await?;
        Ok(true)
    }

    pub(crate) async fn release_delivery(
        &self,
        schedule_id: Uuid,
        user_id: &UserId,
        scheduled_for: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM report_deliveries \
             WHERE schedule_id = ? AND user_id = ? AND scheduled_for = ? AND status = 'sending'",
        )
        .bind(schedule_id.to_string())
        .bind(user_id.as_str())
        .bind(mapper::naive(scheduled_for))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_delivery(
        &self,
        tx: &mut Tx,
        delivery: &ReportDelivery,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO report_deliveries (id, schedule_id, report_id, user_id, scheduled_for, \
             status, row_count, failure_reason, created_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) AS new \
             ON DUPLICATE KEY UPDATE status = new.status, row_count = new.row_count, \
             failure_reason = new.failure_reason, finished_at = new.finished_at",
        )
        .bind(delivery.id.to_string())
        .bind(delivery.schedule_id.to_string())
        .bind(delivery.report_id.to_string())
        .bind(delivery.user_id.as_str())
        .bind(mapper::naive(delivery.scheduled_for))
        .bind(delivery.status.as_str())
        .bind(delivery.row_count)
        .bind(delivery.failure_reason.as_deref())
        .bind(mapper::naive(delivery.created_at))
        .bind(delivery.finished_at.map(mapper::naive))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn due(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueSchedule>, AppError> {
        let rows = sqlx::query(
            "SELECT s.id, o.timezone FROM report_schedules s \
             LEFT JOIN organization_settings o ON o.id = 1 \
             WHERE s.is_active AND s.next_run_at <= ? ORDER BY s.next_run_at, s.id LIMIT ?",
        )
        .bind(mapper::naive(now))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let timezone: Option<String> = row.try_get("timezone")?;
                Ok(DueSchedule {
                    schedule_id: uuid_column(row, "id")?,
                    timezone: timezone
                        .and_then(|timezone| timezone.parse::<Tz>().ok())
                        .unwrap_or(chrono_tz::UTC),
                })
            })
            .collect()
    }

    pub(crate) async fn lock_due(
        &self,
        tx: &mut Tx,
        schedule_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Option<ReportSchedule>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {SCHEDULE_COLUMNS} FROM report_schedules \
             WHERE id = ? AND is_active AND next_run_at <= ? \
             FOR UPDATE SKIP LOCKED"
        )))
        .bind(schedule_id.to_string())
        .bind(mapper::naive(now))
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut schedules = vec![schedule_from_row(&row)?];
        self.attach_recipients(&mut **tx, &mut schedules).await?;
        Ok(schedules.pop())
    }

    pub(crate) async fn advance(
        &self,
        tx: &mut Tx,
        schedule_id: Uuid,
        next_run_at: DateTime<Utc>,
        last_run_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE report_schedules SET next_run_at = ?, last_run_at = ? \
             WHERE id = ?",
        )
        .bind(mapper::naive(next_run_at))
        .bind(mapper::naive(last_run_at))
        .bind(schedule_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
