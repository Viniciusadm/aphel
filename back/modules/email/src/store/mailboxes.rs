use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{EmailVisibility, MailboxStatus};
use crate::{DueMailbox, MailboxChanges, MailboxSyncState, MailboxView, NewMailbox};
use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper;

const MAILBOX_SELECT: &str = "SELECT m.id, m.user_id, m.provider, m.address, m.display_name, m.status, \
     m.status_reason, m.settings, m.sync_cursor, m.history_days, m.default_visibility, \
     m.sync_started_at, m.last_synced_at, m.created_at, m.updated_at FROM mailboxes m ";

pub(crate) fn json_column(row: &MySqlRow, column: &str) -> Result<Option<Value>, AppError> {
    let value: Option<sqlx::types::Json<Value>> = row.try_get(column)?;
    Ok(value.map(|value| value.0))
}

pub(crate) fn optional_utc(
    row: &MySqlRow,
    column: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let value: Option<NaiveDateTime> = row.try_get(column)?;
    Ok(value.map(mapper::utc))
}

fn mailbox_from_row(row: &MySqlRow) -> Result<MailboxView, AppError> {
    let user_id: String = row.try_get("user_id")?;
    let status: String = row.try_get("status")?;
    let visibility: String = row.try_get("default_visibility")?;
    let history_days: i32 = row.try_get("history_days")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(MailboxView {
        id: uuid_column(row, "id")?,
        owner: mapper::typed_id(user_id, UserId::new)?,
        provider: row.try_get("provider")?,
        address: row.try_get("address")?,
        display_name: row.try_get("display_name")?,
        status: MailboxStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        status_reason: row.try_get("status_reason")?,
        settings: json_column(row, "settings")?.unwrap_or(Value::Object(Default::default())),
        sync_cursor: json_column(row, "sync_cursor")?,
        history_days: u32::try_from(history_days).unwrap_or_default(),
        default_visibility: EmailVisibility::parse(&visibility)
            .ok_or_else(|| mapper::invalid_column("default_visibility", &visibility))?,
        sync_started_at: optional_utc(row, "sync_started_at")?,
        last_synced_at: optional_utc(row, "last_synced_at")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub(crate) struct MailboxStore {
    pool: MySqlPool,
}

impl MailboxStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl MailboxStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        mailbox: &NewMailbox,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO mailboxes (id, user_id, provider, address, display_name, status, \
             settings, history_days, default_visibility, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(mailbox.id.to_string())
        .bind(mailbox.owner.as_str())
        .bind(&mailbox.provider)
        .bind(&mailbox.address)
        .bind(&mailbox.display_name)
        .bind(MailboxStatus::Active.as_str())
        .bind(sqlx::types::Json(&mailbox.settings))
        .bind(mailbox.history_days)
        .bind(mailbox.default_visibility.as_str())
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        changes: &MailboxChanges,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut builder = QueryBuilder::<MySql>::new("UPDATE mailboxes SET updated_at = ");
        builder.push_bind(mapper::naive(now));
        if let Some(display_name) = &changes.display_name {
            builder
                .push(", display_name = ")
                .push_bind(display_name.clone());
        }
        if let Some(settings) = &changes.settings {
            builder
                .push(", settings = ")
                .push_bind(sqlx::types::Json(settings.clone()));
        }
        if let Some(history_days) = changes.history_days {
            builder.push(", history_days = ").push_bind(history_days);
        }
        if let Some(visibility) = changes.default_visibility {
            builder
                .push(", default_visibility = ")
                .push_bind(visibility.as_str());
        }
        builder.push(" WHERE id = ").push_bind(id.to_string());
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn save_sync_state(
        &self,
        id: Uuid,
        state: &MailboxSyncState,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE mailboxes SET status = ?, status_reason = ?, sync_cursor = COALESCE(?, sync_cursor), \
             sync_started_at = ?, last_synced_at = COALESCE(?, last_synced_at), updated_at = ? \
             WHERE id = ? AND status <> 'disconnected'",
        )
        .bind(state.status.as_str())
        .bind(&state.status_reason)
        .bind(state.cursor.as_ref().map(sqlx::types::Json))
        .bind(state.sync_started_at.map(mapper::naive))
        .bind(state.last_synced_at.map(mapper::naive))
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM mailboxes WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        aphel_sdk::reference_cleanup::removed(tx, "email.mailbox", id).await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<MailboxView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{MAILBOX_SELECT} WHERE m.id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(mailbox_from_row).transpose()
    }

    pub(crate) async fn find_by_address(
        &self,
        owner: &UserId,
        address: &str,
    ) -> Result<Option<MailboxView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{MAILBOX_SELECT} WHERE m.user_id = ? AND m.address = ?"
        )))
        .bind(owner.as_str())
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(mailbox_from_row).transpose()
    }

    pub(crate) async fn list_for_user(&self, owner: &UserId) -> Result<Vec<MailboxView>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{MAILBOX_SELECT} WHERE m.user_id = ? ORDER BY m.created_at, m.id"
        )))
        .bind(owner.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(mailbox_from_row).collect()
    }

    pub(crate) async fn organization_addresses(&self) -> Result<Vec<String>, AppError> {
        let addresses = sqlx::query_scalar(
            "SELECT DISTINCT address FROM mailboxes WHERE status <> 'disconnected'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(addresses)
    }
}

impl MailboxStore {
    pub(crate) async fn due_for_sync(
        &self,
        synced_before: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueMailbox>, AppError> {
        let rows = sqlx::query(
            "SELECT id, user_id FROM mailboxes \
             WHERE status IN ('active', 'error', 'syncing') \
             AND (last_synced_at IS NULL OR last_synced_at < ?) \
             ORDER BY last_synced_at IS NOT NULL, last_synced_at, id LIMIT ?",
        )
        .bind(mapper::naive(synced_before))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let owner: String = row.try_get("user_id")?;
                Ok(DueMailbox {
                    mailbox_id: uuid_column(row, "id")?,
                    owner: mapper::typed_id(owner, UserId::new)?,
                })
            })
            .collect()
    }
}
