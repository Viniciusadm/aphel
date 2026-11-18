use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    EmailDirection, EmailKind, EmailStatus, EmailVisibility, ParticipantRole, effective_visibility,
    normalize_message_id, parse_references,
};
use crate::store::mailboxes::optional_utc;
use crate::{
    EmailAttachmentView, EmailMessageView, EngagementKind, MessageStatusChange, NewEmailMessage,
    NewThread, ParticipantView, ThreadView,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::{UserReference, sha256_hex};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const THREAD_SELECT: &str = "SELECT t.id, t.mailbox_id, m.user_id AS owner_id, u.display_name AS owner_name, \
     m.address AS mailbox_address, t.subject, t.visibility, m.default_visibility, t.message_count, t.snippet, \
     t.last_direction, t.first_message_at, t.last_message_at, t.created_at, t.updated_at \
     FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id LEFT JOIN users u ON u.id = m.user_id ";

const MESSAGE_SELECT: &str = "SELECT e.id, e.mailbox_id, e.thread_id, e.direction, e.kind, e.status, \
     e.message_id_header, e.in_reply_to, e.references_header, e.report_for_message_id, e.provider_message_id, \
     e.subject, e.preview, e.body_file_id, e.has_html, e.has_attachments, e.tracking_enabled, e.sent_at, \
     e.scheduled_at, e.opened_at, e.open_count, e.clicked_at, e.click_count, e.replied_at, e.bounced_at, \
     e.failure_reason, e.created_by, e.created_at, e.updated_at FROM email_messages e ";

const DELIVERED_STATUSES: &str = "('received', 'sent')";

pub(crate) fn message_id_hash(message_id: &str) -> String {
    let normalized =
        normalize_message_id(message_id).unwrap_or_else(|| message_id.trim().to_owned());
    sha256_hex(normalized.as_bytes())
}

fn thread_from_row(row: &MySqlRow) -> Result<ThreadView, AppError> {
    let owner_id: String = row.try_get("owner_id")?;
    let owner_name: Option<String> = row.try_get("owner_name")?;
    let visibility: Option<String> = row.try_get("visibility")?;
    let default_visibility: String = row.try_get("default_visibility")?;
    let last_direction: Option<String> = row.try_get("last_direction")?;
    let message_count: i32 = row.try_get("message_count")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let visibility_override = visibility
        .map(|value| {
            EmailVisibility::parse(&value)
                .ok_or_else(|| mapper::invalid_column("visibility", &value))
        })
        .transpose()?;
    let mailbox_default = EmailVisibility::parse(&default_visibility)
        .ok_or_else(|| mapper::invalid_column("default_visibility", &default_visibility))?;
    Ok(ThreadView {
        id: uuid_column(row, "id")?,
        mailbox_id: uuid_column(row, "mailbox_id")?,
        owner: UserReference {
            id: mapper::typed_id(owner_id, UserId::new)?,
            display_name: owner_name.unwrap_or_default(),
        },
        mailbox_address: row.try_get("mailbox_address")?,
        subject: row.try_get("subject")?,
        visibility: effective_visibility(visibility_override, mailbox_default),
        visibility_override,
        message_count: u32::try_from(message_count).unwrap_or_default(),
        snippet: row.try_get("snippet")?,
        last_direction: last_direction
            .map(|value| {
                EmailDirection::parse(&value)
                    .ok_or_else(|| mapper::invalid_column("last_direction", &value))
            })
            .transpose()?,
        first_message_at: optional_utc(row, "first_message_at")?,
        last_message_at: optional_utc(row, "last_message_at")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value)))
        .transpose()
}

fn count(row: &MySqlRow, column: &str) -> Result<u32, AppError> {
    let value: i32 = row.try_get(column)?;
    Ok(u32::try_from(value).unwrap_or_default())
}

fn message_from_row(row: &MySqlRow) -> Result<EmailMessageView, AppError> {
    let direction: String = row.try_get("direction")?;
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    let references: Option<String> = row.try_get("references_header")?;
    let created_by: Option<String> = row.try_get("created_by")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(EmailMessageView {
        id: uuid_column(row, "id")?,
        mailbox_id: uuid_column(row, "mailbox_id")?,
        thread_id: uuid_column(row, "thread_id")?,
        direction: EmailDirection::parse(&direction)
            .ok_or_else(|| mapper::invalid_column("direction", &direction))?,
        kind: EmailKind::parse(&kind).ok_or_else(|| mapper::invalid_column("kind", &kind))?,
        status: EmailStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        message_id: row.try_get("message_id_header")?,
        in_reply_to: row.try_get("in_reply_to")?,
        references: references
            .as_deref()
            .map(parse_references)
            .unwrap_or_default(),
        report_for_message_id: row.try_get("report_for_message_id")?,
        provider_message_id: row.try_get("provider_message_id")?,
        subject: row.try_get("subject")?,
        preview: row.try_get("preview")?,
        body_file_id: optional_uuid(row, "body_file_id")?,
        has_html: row.try_get("has_html")?,
        has_attachments: row.try_get("has_attachments")?,
        tracking_enabled: row.try_get("tracking_enabled")?,
        sent_at: optional_utc(row, "sent_at")?,
        scheduled_at: optional_utc(row, "scheduled_at")?,
        opened_at: optional_utc(row, "opened_at")?,
        open_count: count(row, "open_count")?,
        clicked_at: optional_utc(row, "clicked_at")?,
        click_count: count(row, "click_count")?,
        replied_at: optional_utc(row, "replied_at")?,
        bounced_at: optional_utc(row, "bounced_at")?,
        failure_reason: row.try_get("failure_reason")?,
        created_by: created_by
            .map(|value| mapper::typed_id(value, UserId::new))
            .transpose()?,
        participants: Vec::new(),
        attachments: Vec::new(),
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub(crate) struct MessageStore {
    pool: MySqlPool,
}

impl MessageStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn hydrate(
        &self,
        mut messages: Vec<EmailMessageView>,
    ) -> Result<Vec<EmailMessageView>, AppError> {
        if messages.is_empty() {
            return Ok(messages);
        }
        let ids: Vec<String> = messages
            .iter()
            .map(|message| message.id.to_string())
            .collect();

        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT message_id, role, address, name, contact_id FROM email_participants WHERE message_id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in &ids {
            separated.push_bind(id.as_str());
        }
        separated.push_unseparated(") ORDER BY message_id, role, position");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut participants: HashMap<Uuid, Vec<ParticipantView>> = HashMap::new();
        for row in &rows {
            let role: String = row.try_get("role")?;
            participants
                .entry(uuid_column(row, "message_id")?)
                .or_default()
                .push(ParticipantView {
                    role: ParticipantRole::parse(&role)
                        .ok_or_else(|| mapper::invalid_column("role", &role))?,
                    address: row.try_get("address")?,
                    name: row.try_get("name")?,
                    contact_id: optional_uuid(row, "contact_id")?,
                });
        }

        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id, message_id, file_id, name, mime_type, size, content_id, inline FROM email_attachments WHERE message_id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in &ids {
            separated.push_bind(id.as_str());
        }
        separated.push_unseparated(") ORDER BY message_id, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut attachments: HashMap<Uuid, Vec<EmailAttachmentView>> = HashMap::new();
        for row in &rows {
            let size: u64 = row.try_get("size")?;
            attachments
                .entry(uuid_column(row, "message_id")?)
                .or_default()
                .push(EmailAttachmentView {
                    id: uuid_column(row, "id")?,
                    file_id: uuid_column(row, "file_id")?,
                    name: row.try_get("name")?,
                    mime_type: row.try_get("mime_type")?,
                    size,
                    content_id: row.try_get("content_id")?,
                    inline: row.try_get("inline")?,
                });
        }

        for message in &mut messages {
            message.participants = participants.remove(&message.id).unwrap_or_default();
            message.attachments = attachments.remove(&message.id).unwrap_or_default();
        }
        Ok(messages)
    }

    async fn messages_where(
        &self,
        condition: &str,
        binds: Vec<String>,
        suffix: &str,
    ) -> Result<Vec<EmailMessageView>, AppError> {
        let sql = format!("{MESSAGE_SELECT} WHERE {condition} {suffix}");
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        for value in binds {
            query = query.bind(value);
        }
        let rows = query.fetch_all(&self.pool).await?;
        let messages = rows
            .iter()
            .map(message_from_row)
            .collect::<Result<_, _>>()?;
        self.hydrate(messages).await
    }
}

impl MessageStore {
    pub(crate) async fn insert_thread(
        &self,
        tx: &mut Tx,
        thread: &NewThread,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_threads (id, mailbox_id, subject, visibility, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(thread.id.to_string())
        .bind(thread.mailbox_id.to_string())
        .bind(&thread.subject)
        .bind(thread.visibility.map(|visibility| visibility.as_str()))
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find_thread_by_message_ids(
        &self,
        mailbox_id: Uuid,
        message_ids: &[String],
    ) -> Result<Option<Uuid>, AppError> {
        if message_ids.is_empty() {
            return Ok(None);
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT thread_id FROM email_messages WHERE mailbox_id = ");
        builder
            .push_bind(mailbox_id.to_string())
            .push(" AND message_id_hash IN (");
        let mut separated = builder.separated(", ");
        for message_id in message_ids {
            separated.push_bind(message_id_hash(message_id));
        }
        separated.push_unseparated(") ORDER BY created_at DESC LIMIT 1");
        let row = builder.build().fetch_optional(&self.pool).await?;
        row.as_ref()
            .map(|row| uuid_column(row, "thread_id"))
            .transpose()
    }

    pub(crate) async fn insert_message(
        &self,
        tx: &mut Tx,
        message: &NewEmailMessage,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let references = (!message.references.is_empty()).then(|| {
            message
                .references
                .iter()
                .map(|reference| format!("<{reference}>"))
                .collect::<Vec<_>>()
                .join(" ")
        });
        let result = sqlx::query(
            "INSERT INTO email_messages (id, mailbox_id, thread_id, direction, kind, status, \
             message_id_header, message_id_hash, in_reply_to, references_header, report_for_message_id, \
             provider_message_id, subject, preview, body_file_id, has_html, has_attachments, tracking_enabled, \
             sent_at, scheduled_at, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE id = id",
        )
        .bind(message.id.to_string())
        .bind(message.mailbox_id.to_string())
        .bind(message.thread_id.to_string())
        .bind(message.direction.as_str())
        .bind(message.kind.as_str())
        .bind(message.status.as_str())
        .bind(&message.message_id)
        .bind(message_id_hash(&message.message_id))
        .bind(&message.in_reply_to)
        .bind(references)
        .bind(&message.report_for_message_id)
        .bind(&message.provider_message_id)
        .bind(&message.subject)
        .bind(&message.preview)
        .bind(message.body_file_id.map(|id| id.to_string()))
        .bind(message.has_html)
        .bind(!message.attachments.is_empty())
        .bind(message.tracking_enabled)
        .bind(message.sent_at.map(mapper::naive))
        .bind(message.scheduled_at.map(mapper::naive))
        .bind(message.created_by.as_ref().map(|user| user.as_str().to_owned()))
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        if result.rows_affected() != 1 {
            return Ok(false);
        }

        if !message.participants.is_empty() {
            let mut builder = QueryBuilder::<MySql>::new(
                "INSERT INTO email_participants (id, message_id, role, address, name, contact_id, position) ",
            );
            builder.push_values(
                message.participants.iter().enumerate(),
                |mut values, (position, participant)| {
                    values
                        .push_bind(Uuid::now_v7().to_string())
                        .push_bind(message.id.to_string())
                        .push_bind(participant.role.as_str())
                        .push_bind(participant.address.clone())
                        .push_bind(participant.name.clone())
                        .push_bind(participant.contact_id.map(|id| id.to_string()))
                        .push_bind(u32::try_from(position).unwrap_or(u32::MAX));
                },
            );
            builder.build().execute(&mut **tx).await?;
        }

        if !message.attachments.is_empty() {
            let mut builder = QueryBuilder::<MySql>::new(
                "INSERT INTO email_attachments (id, message_id, file_id, name, mime_type, size, content_id, inline) ",
            );
            builder.push_values(&message.attachments, |mut values, attachment| {
                values
                    .push_bind(Uuid::now_v7().to_string())
                    .push_bind(message.id.to_string())
                    .push_bind(attachment.file_id.to_string())
                    .push_bind(attachment.name.clone())
                    .push_bind(attachment.mime_type.clone())
                    .push_bind(attachment.size)
                    .push_bind(attachment.content_id.clone())
                    .push_bind(attachment.inline);
            });
            builder.build().execute(&mut **tx).await?;
        }
        Ok(true)
    }

    pub(crate) async fn refresh_thread(
        &self,
        tx: &mut Tx,
        thread_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE email_threads t \
             LEFT JOIN ( \
               SELECT thread_id, COUNT(*) AS total, MIN(COALESCE(sent_at, created_at)) AS first_at, \
                      MAX(COALESCE(sent_at, created_at)) AS last_at \
               FROM email_messages WHERE thread_id = ? AND status IN {DELIVERED_STATUSES} \
               GROUP BY thread_id \
             ) s ON s.thread_id = t.id \
             LEFT JOIN ( \
               SELECT preview, direction, subject FROM email_messages \
               WHERE thread_id = ? AND status IN {DELIVERED_STATUSES} \
               ORDER BY COALESCE(sent_at, created_at) DESC, id DESC LIMIT 1 \
             ) l ON TRUE \
             SET t.message_count = COALESCE(s.total, 0), t.first_message_at = s.first_at, \
                 t.last_message_at = s.last_at, t.snippet = COALESCE(l.preview, ''), \
                 t.last_direction = l.direction, \
                 t.subject = CASE WHEN t.subject = '' THEN COALESCE(l.subject, '') ELSE t.subject END, \
                 t.updated_at = ? \
             WHERE t.id = ?"
        )))
        .bind(thread_id.to_string())
        .bind(thread_id.to_string())
        .bind(mapper::naive(now))
        .bind(thread_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_thread_visibility(
        &self,
        tx: &mut Tx,
        thread_id: Uuid,
        visibility: Option<EmailVisibility>,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE email_threads SET visibility = ?, updated_at = ? WHERE id = ?")
            .bind(visibility.map(|visibility| visibility.as_str()))
            .bind(mapper::naive(now))
            .bind(thread_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_message_status(
        &self,
        tx: &mut Tx,
        id: Uuid,
        change: &MessageStatusChange,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut builder = QueryBuilder::<MySql>::new("UPDATE email_messages SET updated_at = ");
        builder.push_bind(mapper::naive(now));
        if let Some(status) = change.status {
            builder.push(", status = ").push_bind(status.as_str());
        }
        if let Some(message_id) = &change.message_id {
            builder
                .push(", message_id_header = ")
                .push_bind(message_id.clone())
                .push(", message_id_hash = ")
                .push_bind(message_id_hash(message_id));
        }
        if let Some(provider_message_id) = &change.provider_message_id {
            builder
                .push(", provider_message_id = ")
                .push_bind(provider_message_id.clone());
        }
        if let Some(sent_at) = change.sent_at {
            builder
                .push(", sent_at = ")
                .push_bind(sent_at.map(mapper::naive));
        }
        if let Some(scheduled_at) = change.scheduled_at {
            builder
                .push(", scheduled_at = ")
                .push_bind(scheduled_at.map(mapper::naive));
        }
        if let Some(failure_reason) = &change.failure_reason {
            builder
                .push(", failure_reason = ")
                .push_bind(failure_reason.clone());
        }
        builder.push(" WHERE id = ").push_bind(id.to_string());
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn record_engagement(
        &self,
        tx: &mut Tx,
        id: Uuid,
        kind: EngagementKind,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let column = match kind {
            EngagementKind::Opened => "opened_at",
            EngagementKind::Clicked => "clicked_at",
            EngagementKind::Replied => "replied_at",
            EngagementKind::Bounced => "bounced_at",
        };
        let previous: Option<Option<NaiveDateTime>> = sqlx::query_scalar(sqlx::AssertSqlSafe(
            format!("SELECT {column} FROM email_messages WHERE id = ? FOR UPDATE"),
        ))
        .bind(id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(previous) = previous else {
            return Ok(false);
        };
        let counter = match kind {
            EngagementKind::Opened => ", open_count = open_count + 1",
            EngagementKind::Clicked => ", click_count = click_count + 1",
            EngagementKind::Replied | EngagementKind::Bounced => "",
        };
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE email_messages SET {column} = COALESCE({column}, ?){counter}, updated_at = ? \
             WHERE id = ?"
        )))
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(previous.is_none())
    }

    pub(crate) async fn set_participant_contact(
        &self,
        tx: &mut Tx,
        thread_id: Uuid,
        address: &str,
        contact_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE email_participants p JOIN email_messages e ON e.id = p.message_id \
             SET p.contact_id = ? \
             WHERE e.thread_id = ? AND p.address = ?",
        )
        .bind(contact_id.map(|id| id.to_string()))
        .bind(thread_id.to_string())
        .bind(address)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn message_exists(
        &self,
        mailbox_id: Uuid,
        message_id: &str,
    ) -> Result<bool, AppError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM email_messages WHERE mailbox_id = ? AND message_id_hash = ? LIMIT 1",
        )
        .bind(mailbox_id.to_string())
        .bind(message_id_hash(message_id))
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    pub(crate) async fn find_message(
        &self,
        id: Uuid,
    ) -> Result<Option<EmailMessageView>, AppError> {
        Ok(self
            .messages_where("e.id = ?", vec![id.to_string()], "")
            .await?
            .into_iter()
            .next())
    }

    pub(crate) async fn find_message_by_header(
        &self,
        message_id: &str,
    ) -> Result<Option<EmailMessageView>, AppError> {
        Ok(self
            .messages_where(
                "e.message_id_hash = ?",
                vec![message_id_hash(message_id)],
                "ORDER BY e.direction = 'outbound' DESC, e.created_at LIMIT 1",
            )
            .await?
            .into_iter()
            .next())
    }

    pub(crate) async fn find_thread(&self, id: Uuid) -> Result<Option<ThreadView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{THREAD_SELECT} WHERE t.id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(thread_from_row).transpose()
    }

    pub(crate) async fn thread_messages(
        &self,
        thread_id: Uuid,
    ) -> Result<Vec<EmailMessageView>, AppError> {
        self.messages_where(
            "e.thread_id = ?",
            vec![thread_id.to_string()],
            "ORDER BY COALESCE(e.sent_at, e.scheduled_at, e.created_at), e.id",
        )
        .await
    }

    pub(crate) async fn list_threads_for_record(
        &self,
        record: &RecordRef,
        viewer: &UserId,
        window: ListWindow,
    ) -> Result<Listing<ThreadView>, AppError> {
        let filter = "EXISTS (SELECT 1 FROM email_record_links l \
             WHERE l.thread_id = t.id AND l.record_type = ? AND l.record_id = ?) \
             AND (m.user_id = ? OR COALESCE(t.visibility, m.default_visibility) <> 'private')";
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM email_threads t JOIN mailboxes m ON m.id = t.mailbox_id WHERE {filter}"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(viewer.as_str())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{THREAD_SELECT} WHERE {filter} ORDER BY t.last_message_at IS NULL, t.last_message_at DESC, t.id DESC LIMIT ? OFFSET ?"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(viewer.as_str())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows.iter().map(thread_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn list_threads_for_mailbox(
        &self,
        mailbox_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<ThreadView>, AppError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM email_threads WHERE mailbox_id = ?")
                .bind(mailbox_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{THREAD_SELECT} WHERE t.mailbox_id = ? \
             ORDER BY t.last_message_at IS NULL, t.last_message_at DESC, t.id DESC LIMIT ? OFFSET ?"
        )))
        .bind(mailbox_id.to_string())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows.iter().map(thread_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }
}
