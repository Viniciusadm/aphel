use aphel_sdk::platform::domain::users::UserStatus;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    DueReminder, DueTaskReminder, ReminderData, ReminderFilter, ReminderStatus, ReminderView,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::timeline::LinkedRecord;

pub(crate) const MAX_DUE_REMINDERS_PER_RUN: u32 = 1000;

#[derive(Clone)]
pub(crate) struct ReminderStore {
    pool: MySqlPool,
}

impl ReminderStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn reminder_select() -> &'static str {
    "SELECT r.id, r.record_type, r.record_id, NULL AS record_title, r.remind_at, r.note, r.status, r.sent_at, r.created_at, r.updated_at FROM reminders r "
}

fn linked_record(
    row: &MySqlRow,
    type_column: &str,
    id_column: &str,
    title_column: &str,
) -> Result<Option<LinkedRecord>, AppError> {
    let record_type: Option<String> = row.try_get(type_column)?;
    let record_id: Option<String> = row.try_get(id_column)?;
    let (Some(record_type), Some(record_id)) = (record_type, record_id) else {
        return Ok(None);
    };
    Ok(Some(LinkedRecord {
        record: RecordRef::parse(&record_type, &record_id)
            .map_err(|_| mapper::invalid_column(type_column, &record_type))?,
        title: row.try_get(title_column)?,
    }))
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    let value: Option<NaiveDateTime> = row.try_get(column)?;
    Ok(value.map(mapper::utc))
}

fn instant(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    let value: NaiveDateTime = row.try_get(column)?;
    Ok(mapper::utc(value))
}

fn reminder_from_row(row: &MySqlRow) -> Result<ReminderView, AppError> {
    let status: String = row.try_get("status")?;
    Ok(ReminderView {
        id: uuid_column(row, "id")?,
        record: linked_record(row, "record_type", "record_id", "record_title")?,
        remind_at: instant(row, "remind_at")?,
        note: row.try_get("note")?,
        status: ReminderStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        sent_at: optional_instant(row, "sent_at")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

fn push_owner(builder: &mut QueryBuilder<MySql>, scope: &AccessScope) {
    builder
        .push(" WHERE r.user_id = ")
        .push_bind(scope.user_id().as_str().to_owned());
}

fn push_filter(builder: &mut QueryBuilder<MySql>, filter: &ReminderFilter) {
    if let Some(status) = filter.status {
        builder
            .push(" AND r.status = ")
            .push_bind(status.as_str().to_owned());
    }
    if let Some(record) = &filter.record {
        builder
            .push(" AND r.record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(record.record_id.to_string());
    }
}

impl ReminderStore {
    pub(crate) async fn list(
        &self,
        scope: &AccessScope,
        filter: &ReminderFilter,
        window: ListWindow,
    ) -> Result<Listing<ReminderView>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM reminders r");
        push_owner(&mut count, scope);
        push_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::<MySql>::new(reminder_select());
        push_owner(&mut builder, scope);
        push_filter(&mut builder, filter);
        builder
            .push(" ORDER BY r.remind_at, r.id LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(reminder_from_row)
                .collect::<Result<Vec<_>, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn find(
        &self,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<Option<ReminderView>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(reminder_select());
        push_owner(&mut builder, scope);
        builder.push(" AND r.id = ").push_bind(id.to_string());
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref().map(reminder_from_row).transpose()
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        id: Uuid,
        data: &ReminderData,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO reminders (id, user_id, record_type, record_id, remind_at, note, status, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(scope.user_id().as_str())
        .bind(
            data.record
                .as_ref()
                .map(|record| record.record_type.as_str()),
        )
        .bind(
            data.record
                .as_ref()
                .map(|record| record.record_id.to_string()),
        )
        .bind(mapper::naive(data.remind_at))
        .bind(&data.note)
        .bind(data.status.as_str())
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        id: Uuid,
        data: &ReminderData,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE reminders SET sent_at = IF(? = 'pending', NULL, sent_at), record_type = ?, record_id = ?, \
             remind_at = ?, note = ?, status = ?, updated_at = ? \
             WHERE user_id = ? AND id = ?",
        )
        .bind(data.status.as_str())
        .bind(data.record.as_ref().map(|record| record.record_type.as_str()))
        .bind(data.record.as_ref().map(|record| record.record_id.to_string()))
        .bind(mapper::naive(data.remind_at))
        .bind(&data.note)
        .bind(data.status.as_str())
        .bind(mapper::naive(now))
        .bind(scope.user_id().as_str())
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM reminders WHERE user_id = ? AND id = ?")
            .bind(scope.user_id().as_str())
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}

impl ReminderStore {
    pub(crate) async fn due_task_reminders(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Vec<DueTaskReminder>, AppError> {
        let Some(source) = aphel_sdk::platform::modules::app()
            .services()
            .get::<dyn aphel_sdk::tasks::TaskReminderSource>()
        else {
            return Ok(Vec::new());
        };
        source
            .due(&self.pool, since, now, MAX_DUE_REMINDERS_PER_RUN)
            .await
    }
    pub(crate) async fn mark_task_reminder_sent(
        &self,
        reminder: &DueTaskReminder,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let source = aphel_sdk::platform::modules::app()
            .services()
            .get::<dyn aphel_sdk::tasks::TaskReminderSource>()
            .ok_or_else(|| {
                AppError::unavailable_code(
                    "task_reminders_unavailable",
                    "task reminders are not enabled",
                )
            })?;
        source.mark_sent(&self.pool, reminder, now).await
    }

    pub(crate) async fn due_reminders(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Vec<DueReminder>, AppError> {
        let rows = sqlx::query(
            "SELECT r.id, r.user_id, r.record_type, r.record_id, r.note, r.remind_at \
             FROM reminders r \
             JOIN users m ON m.id = r.user_id \
             WHERE r.status = 'pending' AND r.remind_at > ? AND r.remind_at <= ? AND m.status = ? \
             ORDER BY r.remind_at, r.id LIMIT ?",
        )
        .bind(mapper::naive(since))
        .bind(mapper::naive(now))
        .bind(UserStatus::Active.as_str())
        .bind(MAX_DUE_REMINDERS_PER_RUN)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let record_type: Option<String> = row.try_get("record_type")?;
                let record_id: Option<String> = row.try_get("record_id")?;
                Ok(DueReminder {
                    id: uuid_column(row, "id")?,
                    user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                    record: match (record_type, record_id) {
                        (Some(record_type), Some(record_id)) => {
                            RecordRef::parse(&record_type, &record_id).ok()
                        }
                        _ => None,
                    },
                    note: row.try_get("note")?,
                    remind_at: instant(row, "remind_at")?,
                })
            })
            .collect()
    }

    pub(crate) async fn mark_reminder_sent(
        &self,
        reminder: &DueReminder,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE reminders SET status = 'sent', sent_at = ?, updated_at = ? \
             WHERE id = ? AND status = 'pending' AND remind_at = ?",
        )
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .bind(reminder.id.to_string())
        .bind(mapper::naive(reminder.remind_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
