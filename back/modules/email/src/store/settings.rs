use aphel_contract_crm::CrmRecordTypes;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::ParticipantView;
use crate::domain::{EmailDirection, ParticipantRole};
use crate::store::mailboxes::optional_utc;
use crate::sync::{
    BlockKind, BlockedEntry, CaptureSettings, StoredMailboxFile, SyncFailure, ThreadParticipant,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::files::NewFile;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::users::UserStatus;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper;

fn block_from_row(row: &sqlx::mysql::MySqlRow) -> Result<BlockedEntry, AppError> {
    let kind: String = row.try_get("kind")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    Ok(BlockedEntry {
        id: uuid_column(row, "id")?,
        kind: BlockKind::parse(&kind).ok_or_else(|| mapper::invalid_column("kind", &kind))?,
        value: row.try_get("value")?,
        reason: row.try_get("reason")?,
        created_at: mapper::utc(created_at),
    })
}

const BLOCK_SELECT: &str = "SELECT id, kind, value, reason, created_at FROM email_blocklist ";

#[derive(Clone)]
pub(crate) struct SettingsStore {
    pool: MySqlPool,
}

impl SettingsStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl SettingsStore {
    pub(crate) async fn capture_settings(&self) -> Result<CaptureSettings, AppError> {
        let row = sqlx::query(
            "SELECT create_contacts, create_companies, link_open_deals, initial_history_days \
             FROM email_capture_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(CaptureSettings::default());
        };
        let days: i32 = row.try_get("initial_history_days")?;
        Ok(CaptureSettings {
            create_contacts: row.try_get("create_contacts")?,
            create_companies: row.try_get("create_companies")?,
            link_open_deals: row.try_get("link_open_deals")?,
            initial_history_days: u32::try_from(days).unwrap_or(90),
        })
    }

    pub(crate) async fn save_capture_settings(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        settings: &CaptureSettings,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_capture_settings (id, create_contacts, create_companies, \
             link_open_deals, initial_history_days, updated_by, updated_at) VALUES (1, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE create_contacts = VALUES(create_contacts), \
             create_companies = VALUES(create_companies), link_open_deals = VALUES(link_open_deals), \
             initial_history_days = VALUES(initial_history_days), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(settings.create_contacts)
        .bind(settings.create_companies)
        .bind(settings.link_open_deals)
        .bind(settings.initial_history_days)
        .bind(scope.user_id().as_str())
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn blocklist(&self) -> Result<Vec<BlockedEntry>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{BLOCK_SELECT} ORDER BY kind, value"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(block_from_row).collect()
    }

    pub(crate) async fn find_block(&self, id: Uuid) -> Result<Option<BlockedEntry>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("{BLOCK_SELECT} WHERE id = ?")))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(block_from_row).transpose()
    }

    pub(crate) async fn find_block_by_value(
        &self,
        kind: BlockKind,
        value: &str,
    ) -> Result<Option<BlockedEntry>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{BLOCK_SELECT} WHERE kind = ? AND value = ?"
        )))
        .bind(kind.as_str())
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(block_from_row).transpose()
    }

    pub(crate) async fn insert_block(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        entry: &BlockedEntry,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_blocklist (id, kind, value, reason, created_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.id.to_string())
        .bind(entry.kind.as_str())
        .bind(&entry.value)
        .bind(&entry.reason)
        .bind(scope.user_id().as_str())
        .bind(mapper::naive(entry.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_block(&self, tx: &mut MySqlTx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM email_blocklist WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() == 1)
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
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: impl IntoIterator<Item = String>) {
    builder.push("(");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    builder.push(")");
}

const ENGAGEMENT_DELIVERED: &str = "e.status IN ('received', 'sent') AND e.kind = 'message'";

impl DirectoryStore {
    pub(crate) async fn member_addresses(&self) -> Result<Vec<String>, AppError> {
        let addresses: Vec<String> =
            sqlx::query_scalar("SELECT LOWER(email) FROM users WHERE status = ?")
                .bind(UserStatus::Active.as_str())
                .fetch_all(&self.pool)
                .await?;
        Ok(addresses)
    }

    pub(crate) async fn exchanged_with(
        &self,
        mailbox_id: Uuid,
        address: &str,
        direction: EmailDirection,
    ) -> Result<bool, AppError> {
        let roles = match direction {
            EmailDirection::Inbound => "('from')",
            EmailDirection::Outbound => "('to', 'cc', 'bcc')",
        };
        let exists: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT EXISTS(SELECT 1 FROM email_messages e JOIN email_participants p ON p.message_id = e.id \
             WHERE e.mailbox_id = ? AND e.direction = ? \
             AND e.status IN ('received', 'sent') AND p.address = ? AND p.role IN {roles})"
        )))
        .bind(mailbox_id.to_string())
        .bind(direction.as_str())
        .bind(address)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    pub(crate) async fn threads_with_address(
        &self,
        mailbox_id: Uuid,
        address: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        let rows = sqlx::query(
            "SELECT DISTINCT e.thread_id FROM email_participants p JOIN email_messages e ON e.id = p.message_id \
             WHERE p.address = ? AND e.mailbox_id = ?",
        )
        .bind(address)
        .bind(mailbox_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| uuid_column(row, "thread_id"))
            .collect()
    }

    pub(crate) async fn thread_participants(
        &self,
        thread_ids: &[Uuid],
    ) -> Result<Vec<ThreadParticipant>, AppError> {
        if thread_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT e.thread_id, p.role, p.address, MAX(p.name) AS name, MAX(p.contact_id) AS contact_id, \
             MIN(COALESCE(e.sent_at, e.created_at)) AS first_at, MIN(p.position) AS position \
             FROM email_participants p JOIN email_messages e ON e.id = p.message_id WHERE e.thread_id IN ",
        );
        push_ids(&mut builder, thread_ids.iter().map(Uuid::to_string));
        builder.push(
            " GROUP BY e.thread_id, p.role, p.address \
             ORDER BY e.thread_id, FIELD(p.role, 'from', 'to', 'cc', 'bcc', 'reply_to'), first_at, position",
        );
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let role: String = row.try_get("role")?;
                let contact_id: Option<String> = row.try_get("contact_id")?;
                Ok(ThreadParticipant {
                    thread_id: uuid_column(row, "thread_id")?,
                    participant: ParticipantView {
                        role: ParticipantRole::parse(&role)
                            .ok_or_else(|| mapper::invalid_column("role", &role))?,
                        address: row.try_get("address")?,
                        name: row.try_get("name")?,
                        contact_id: contact_id.and_then(|id| Uuid::parse_str(&id).ok()),
                    },
                })
            })
            .collect()
    }

    pub(crate) async fn insert_file(
        &self,
        tx: &mut MySqlTx,
        file: &NewFile,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO files (id, storage_key, original_name, mime_type, size, checksum, uploaded_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(file.id.to_string())
        .bind(&file.storage_key)
        .bind(&file.original_name)
        .bind(&file.mime_type)
        .bind(file.size)
        .bind(&file.checksum)
        .bind(file.uploaded_by.as_str())
        .bind(mapper::naive(file.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn file_storage_key(&self, file_id: Uuid) -> Result<Option<String>, AppError> {
        let key: Option<String> =
            sqlx::query_scalar("SELECT storage_key FROM files WHERE id = ? AND deleted_at IS NULL")
                .bind(file_id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(key)
    }

    pub(crate) async fn threads_for_owner(
        &self,
        owner: &UserId,
        window: ListWindow,
    ) -> Result<Listing<Uuid>, AppError> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id \
             WHERE m.user_id = ?",
        )
        .bind(owner.as_str())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(
            "SELECT t.id FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id \
             WHERE m.user_id = ? \
             ORDER BY t.last_message_at IS NULL, t.last_message_at DESC, t.id DESC LIMIT ? OFFSET ?",
        )
        .bind(owner.as_str())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(|row| uuid_column(row, "id"))
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn mailbox_files(
        &self,
        mailbox_id: Uuid,
    ) -> Result<Vec<StoredMailboxFile>, AppError> {
        let rows = sqlx::query(
            "SELECT f.id, f.storage_key FROM files f WHERE f.id IN ( \
               SELECT e.body_file_id FROM email_messages e WHERE e.mailbox_id = ? \
               AND e.body_file_id IS NOT NULL \
               UNION SELECT a.file_id FROM email_attachments a JOIN email_messages e ON e.id = a.message_id \
               WHERE e.mailbox_id = ? \
               AND NOT EXISTS (SELECT 1 FROM attachments ra WHERE ra.file_id = a.file_id) \
               AND NOT EXISTS (SELECT 1 FROM email_attachments oa JOIN email_messages oe ON oe.id = oa.message_id \
                 WHERE oa.file_id = a.file_id AND oe.mailbox_id <> e.mailbox_id))",
        )
        .bind(mailbox_id.to_string())
        .bind(mailbox_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(StoredMailboxFile {
                    id: uuid_column(row, "id")?,
                    storage_key: row.try_get("storage_key")?,
                })
            })
            .collect()
    }

    pub(crate) async fn delete_files(
        &self,
        tx: &mut MySqlTx,
        ids: &[Uuid],
    ) -> Result<(), AppError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new("DELETE FROM files WHERE id IN ");
        push_ids(&mut builder, ids.iter().map(Uuid::to_string));
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn sync_failure(
        &self,
        mailbox_id: Uuid,
    ) -> Result<Option<SyncFailure>, AppError> {
        let row = sqlx::query(
            "SELECT kind, failures, retry_after FROM mailbox_sync_failures \
             WHERE mailbox_id = ?",
        )
        .bind(mailbox_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let failures: i32 = row.try_get("failures")?;
            Ok(SyncFailure {
                kind: row.try_get("kind")?,
                failures: u32::try_from(failures).unwrap_or_default(),
                retry_after: optional_utc(&row, "retry_after")?,
            })
        })
        .transpose()
    }

    pub(crate) async fn record_sync_failure(
        &self,
        mailbox_id: Uuid,
        kind: &str,
        retry_after: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<u32, AppError> {
        sqlx::query(
            "INSERT INTO mailbox_sync_failures (mailbox_id, kind, failures, retry_after, updated_at) \
             SELECT id, ?, 1, ?, ? FROM mailboxes WHERE id = ? \
             ON DUPLICATE KEY UPDATE kind = VALUES(kind), failures = failures + 1, \
             retry_after = VALUES(retry_after), updated_at = VALUES(updated_at)",
        )
        .bind(kind)
        .bind(retry_after.map(mapper::naive))
        .bind(mapper::naive(now))
        .bind(mailbox_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(self
            .sync_failure(mailbox_id)
            .await?
            .map(|failure| failure.failures)
            .unwrap_or_default())
    }

    pub(crate) async fn clear_sync_failure(&self, mailbox_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM mailbox_sync_failures WHERE mailbox_id = ?")
            .bind(mailbox_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn paused_mailboxes(
        &self,
        mailbox_ids: &[Uuid],
        now: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, AppError> {
        if mailbox_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT mailbox_id FROM mailbox_sync_failures WHERE (retry_after IS NULL OR retry_after > ",
        );
        builder
            .push_bind(mapper::naive(now))
            .push(") AND mailbox_id IN ");
        push_ids(&mut builder, mailbox_ids.iter().map(Uuid::to_string));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| uuid_column(row, "mailbox_id"))
            .collect()
    }

    pub(crate) async fn refresh_engagement(&self, record: &RecordRef) -> Result<(), AppError> {
        let table = if record.record_type == RecordType::CONTACT {
            "people"
        } else if record.record_type == RecordType::COMPANY {
            "organizations"
        } else {
            return Ok(());
        };
        let instant = "COALESCE(e.sent_at, e.created_at)";
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} r \
             LEFT JOIN ( \
               SELECT MAX({instant}) AS last_at, \
                      MAX(CASE WHEN e.direction = 'inbound' THEN {instant} END) AS last_in, \
                      MAX(CASE WHEN e.direction = 'outbound' THEN {instant} END) AS last_out, \
                      COUNT(DISTINCT e.id) AS total \
               FROM email_record_links l JOIN email_messages e ON e.thread_id = l.thread_id \
               WHERE l.record_type = ? AND l.record_id = ? AND {ENGAGEMENT_DELIVERED} \
             ) s ON TRUE \
             LEFT JOIN ( \
               SELECT m.user_id FROM email_record_links l \
               JOIN email_messages e ON e.thread_id = l.thread_id \
               JOIN mailboxes m ON m.id = e.mailbox_id \
               WHERE l.record_type = ? AND l.record_id = ? AND {ENGAGEMENT_DELIVERED} \
               ORDER BY {instant} DESC, e.id DESC LIMIT 1 \
             ) w ON TRUE \
             SET r.last_contacted_at = s.last_at, r.last_email_received_at = s.last_in, \
                 r.last_email_sent_at = s.last_out, r.interaction_count = COALESCE(s.total, 0), \
                 r.last_contacted_by = w.user_id, r.updated_at = r.updated_at \
             WHERE r.id = ?"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(record.record_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
