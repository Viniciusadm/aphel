use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{self, AssertSqlSafe, MySql, MySqlPool, Row};
use uuid::Uuid;

use crate::channels::ChannelKind;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone)]
pub(crate) struct StoredChannel {
    pub(crate) id: Uuid,
    pub(crate) kind: ChannelKind,
    pub(crate) name: String,
    pub(crate) config: Value,
    pub(crate) is_active: bool,
    pub(crate) status: String,
    pub(crate) last_error: Option<String>,
    pub(crate) last_checked_at: Option<NaiveDateTime>,
    pub(crate) last_polled_at: Option<NaiveDateTime>,
    pub(crate) imap_uid_validity: Option<u64>,
    pub(crate) imap_last_uid: Option<u64>,
    pub(crate) acting_user_id: Option<Uuid>,
    pub(crate) created_by: Uuid,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) updated_at: NaiveDateTime,
}

impl StoredChannel {
    pub(crate) fn address(&self) -> String {
        aphel_sdk::messaging::compiled_channel_provider(self.kind.as_str())
            .map(|provider| provider.address(&self.config))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoredConversation {
    pub(crate) id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) peer: String,
    pub(crate) peer_name: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) deal_id: Option<Uuid>,
    pub(crate) status: String,
    pub(crate) assignee_user_id: Option<Uuid>,
    pub(crate) unread_count: u32,
    pub(crate) last_message_at: NaiveDateTime,
    pub(crate) last_inbound_at: Option<NaiveDateTime>,
    pub(crate) last_message_preview: Option<String>,
    pub(crate) last_message_direction: Option<String>,
    pub(crate) created_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredMessage {
    pub(crate) id: Uuid,
    pub(crate) conversation_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) direction: String,
    pub(crate) external_id: Option<String>,
    pub(crate) sender: String,
    pub(crate) sender_name: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) body: String,
    pub(crate) message_type: String,
    pub(crate) status: String,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) thread_references: Option<String>,
    pub(crate) sent_by_user_id: Option<Uuid>,
    pub(crate) provider_timestamp: Option<NaiveDateTime>,
    pub(crate) sent_at: Option<NaiveDateTime>,
    pub(crate) delivered_at: Option<NaiveDateTime>,
    pub(crate) read_at: Option<NaiveDateTime>,
    pub(crate) created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConversationFilter {
    pub(crate) channel_id: Option<Uuid>,
    pub(crate) status: Option<String>,
    pub(crate) assignee: Option<AssigneeFilter>,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) deal_id: Option<Uuid>,
    pub(crate) unread: bool,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssigneeFilter {
    Unassigned,
    User(Uuid),
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value)
        .map_err(|_| AppError::Internal(format!("{column} {value} is not a UUID")))
}

fn optional_uuid_column(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| {
            Uuid::parse_str(&value)
                .map_err(|_| AppError::Internal(format!("{column} {value} is not a UUID")))
        })
        .transpose()
}

fn optional_id(value: Option<Uuid>) -> Option<String> {
    value.map(|id| id.to_string())
}

const CHANNEL_COLUMNS: &str = "id, kind, name, config, is_active, status, last_error, last_checked_at, last_polled_at, imap_uid_validity, imap_last_uid, acting_user_id, created_by, created_at, updated_at";

fn channel_from_row(row: &MySqlRow) -> Result<StoredChannel, AppError> {
    let kind: String = row.try_get("kind")?;
    let Json(config): Json<Value> = row.try_get("config")?;
    Ok(StoredChannel {
        id: uuid_column(row, "id")?,
        kind: ChannelKind::parse(&kind)
            .ok_or_else(|| AppError::Internal(format!("unknown channel kind {kind}")))?,
        name: row.try_get("name")?,
        config,
        is_active: row.try_get("is_active")?,
        status: row.try_get("status")?,
        last_error: row.try_get("last_error")?,
        last_checked_at: row.try_get("last_checked_at")?,
        last_polled_at: row.try_get("last_polled_at")?,
        imap_uid_validity: row.try_get("imap_uid_validity")?,
        imap_last_uid: row.try_get("imap_last_uid")?,
        acting_user_id: optional_uuid_column(row, "acting_user_id")?,
        created_by: uuid_column(row, "created_by")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub(crate) async fn list_channels(pool: &MySqlPool) -> Result<Vec<StoredChannel>, AppError> {
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CHANNEL_COLUMNS} FROM messaging_channels ORDER BY name, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(channel_from_row).collect()
}

pub(crate) async fn find_channel<'e, E>(
    executor: E,
    id: Uuid,
) -> Result<Option<StoredChannel>, AppError>
where
    E: sqlx::Executor<'e, Database = MySql>,
{
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CHANNEL_COLUMNS} FROM messaging_channels WHERE id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(executor)
    .await?;
    row.as_ref().map(channel_from_row).transpose()
}

pub(crate) async fn find_channel_for_webhook(
    pool: &MySqlPool,
    id: Uuid,
    kind: ChannelKind,
) -> Result<Option<StoredChannel>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CHANNEL_COLUMNS} FROM messaging_channels WHERE id = ? AND kind = ?"
    )))
    .bind(id.to_string())
    .bind(kind.as_str())
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(channel_from_row).transpose()
}

pub(crate) async fn polling_channels(pool: &MySqlPool) -> Result<Vec<StoredChannel>, AppError> {
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CHANNEL_COLUMNS} FROM messaging_channels WHERE kind = 'email' AND is_active = 1"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(channel_from_row).collect()
}

pub(crate) async fn insert_channel(
    tx: &mut MySqlTx,
    channel: &StoredChannel,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO messaging_channels (id, kind, name, config, is_active, status, acting_user_id, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(channel.id.to_string())
    .bind(channel.kind.as_str())
    .bind(&channel.name)
    .bind(Json(&channel.config))
    .bind(channel.is_active)
    .bind(&channel.status)
    .bind(optional_id(channel.acting_user_id))
    .bind(channel.created_by.to_string())
    .bind(channel.created_at)
    .bind(channel.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn update_channel(
    tx: &mut MySqlTx,
    channel: &StoredChannel,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_channels SET name = ?, config = ?, is_active = ?, status = ?, last_error = ?, acting_user_id = ?, imap_uid_validity = ?, imap_last_uid = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&channel.name)
    .bind(Json(&channel.config))
    .bind(channel.is_active)
    .bind(&channel.status)
    .bind(&channel.last_error)
    .bind(optional_id(channel.acting_user_id))
    .bind(channel.imap_uid_validity)
    .bind(channel.imap_last_uid)
    .bind(channel.updated_at)
    .bind(channel.id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn delete_channel(tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM messaging_channels WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn count_channels(pool: &MySqlPool) -> Result<u64, AppError> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messaging_channels")
        .fetch_one(pool)
        .await?;
    Ok(u64::try_from(total).unwrap_or_default())
}

pub(crate) async fn count_channel_conversations(
    pool: &MySqlPool,
    channel_id: Uuid,
) -> Result<u64, AppError> {
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messaging_conversations WHERE channel_id = ?")
            .bind(channel_id.to_string())
            .fetch_one(pool)
            .await?;
    Ok(u64::try_from(total).unwrap_or_default())
}

pub(crate) async fn record_channel_check(
    pool: &MySqlPool,
    channel: &StoredChannel,
    error: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_channels SET status = ?, last_error = ?, last_checked_at = ? WHERE id = ?",
    )
    .bind(if error.is_some() { "error" } else { "active" })
    .bind(error)
    .bind(crate::text::now())
    .bind(channel.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn record_channel_poll(
    pool: &MySqlPool,
    channel: &StoredChannel,
    error: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_channels SET status = ?, last_error = ?, last_polled_at = ? WHERE id = ?",
    )
    .bind(if error.is_some() { "error" } else { "active" })
    .bind(error)
    .bind(crate::text::now())
    .bind(channel.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn record_channel_error(
    pool: &MySqlPool,
    channel: &StoredChannel,
    error: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE messaging_channels SET last_error = ? WHERE id = ?")
        .bind(error)
        .bind(channel.id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn save_imap_state(
    pool: &MySqlPool,
    channel: &StoredChannel,
    uid_validity: Option<u64>,
    last_uid: Option<u64>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_channels SET imap_uid_validity = ?, imap_last_uid = ? WHERE id = ?",
    )
    .bind(uid_validity)
    .bind(last_uid)
    .bind(channel.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

const CONVERSATION_COLUMNS: &str = "id, channel_id, peer, peer_name, subject, contact_id, deal_id, status, assignee_user_id, unread_count, last_message_at, last_inbound_at, last_message_preview, last_message_direction, created_at";

fn conversation_from_row(row: &MySqlRow) -> Result<StoredConversation, AppError> {
    Ok(StoredConversation {
        id: uuid_column(row, "id")?,
        channel_id: uuid_column(row, "channel_id")?,
        peer: row.try_get("peer")?,
        peer_name: row.try_get("peer_name")?,
        subject: row.try_get("subject")?,
        contact_id: optional_uuid_column(row, "contact_id")?,
        deal_id: optional_uuid_column(row, "deal_id")?,
        status: row.try_get("status")?,
        assignee_user_id: optional_uuid_column(row, "assignee_user_id")?,
        unread_count: row.try_get("unread_count")?,
        last_message_at: row.try_get("last_message_at")?,
        last_inbound_at: row.try_get("last_inbound_at")?,
        last_message_preview: row.try_get("last_message_preview")?,
        last_message_direction: row.try_get("last_message_direction")?,
        created_at: row.try_get("created_at")?,
    })
}

pub(crate) async fn find_conversation<'e, E>(
    executor: E,
    id: Uuid,
) -> Result<Option<StoredConversation>, AppError>
where
    E: sqlx::Executor<'e, Database = MySql>,
{
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONVERSATION_COLUMNS} FROM messaging_conversations WHERE id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(executor)
    .await?;
    row.as_ref().map(conversation_from_row).transpose()
}

pub(crate) async fn find_conversation_for_update(
    tx: &mut MySqlTx,
    id: Uuid,
) -> Result<Option<StoredConversation>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONVERSATION_COLUMNS} FROM messaging_conversations WHERE id = ? FOR UPDATE"
    )))
    .bind(id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    row.as_ref().map(conversation_from_row).transpose()
}

pub(crate) async fn latest_conversation_with_peer(
    pool: &MySqlPool,
    channel_id: Uuid,
    peer: &str,
) -> Result<Option<StoredConversation>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONVERSATION_COLUMNS} FROM messaging_conversations WHERE channel_id = ? AND peer = ? ORDER BY last_message_at DESC, id DESC LIMIT 1"
    )))
    .bind(channel_id.to_string())
    .bind(peer)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(conversation_from_row).transpose()
}

pub(crate) async fn conversation_by_message_ids(
    pool: &MySqlPool,
    channel_id: Uuid,
    external_ids: &[String],
) -> Result<Option<Uuid>, AppError> {
    if external_ids.is_empty() {
        return Ok(None);
    }
    let placeholders = vec!["?"; external_ids.len()].join(", ");
    let mut query = sqlx::query_scalar::<_, String>(AssertSqlSafe(format!(
        "SELECT conversation_id FROM messaging_messages WHERE channel_id = ? AND external_id IN ({placeholders}) ORDER BY created_at DESC LIMIT 1"
    )))
    .bind(channel_id.to_string());
    for id in external_ids {
        query = query.bind(id);
    }
    let found = query.fetch_optional(pool).await?;
    Ok(found.and_then(|id| Uuid::parse_str(&id).ok()))
}

pub(crate) async fn insert_conversation(
    tx: &mut MySqlTx,
    conversation: &StoredConversation,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO messaging_conversations (id, channel_id, peer, peer_name, subject, contact_id, deal_id, status, assignee_user_id, unread_count, last_message_at, last_inbound_at, last_message_preview, last_message_direction, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(conversation.id.to_string())
    .bind(conversation.channel_id.to_string())
    .bind(&conversation.peer)
    .bind(&conversation.peer_name)
    .bind(&conversation.subject)
    .bind(optional_id(conversation.contact_id))
    .bind(optional_id(conversation.deal_id))
    .bind(&conversation.status)
    .bind(optional_id(conversation.assignee_user_id))
    .bind(conversation.unread_count)
    .bind(conversation.last_message_at)
    .bind(conversation.last_inbound_at)
    .bind(&conversation.last_message_preview)
    .bind(&conversation.last_message_direction)
    .bind(conversation.created_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn save_conversation(
    tx: &mut MySqlTx,
    conversation: &StoredConversation,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_conversations SET peer_name = ?, subject = ?, contact_id = ?, deal_id = ?, status = ?, assignee_user_id = ?, unread_count = ?, last_message_at = ?, last_inbound_at = ?, last_message_preview = ?, last_message_direction = ? WHERE id = ?",
    )
    .bind(&conversation.peer_name)
    .bind(&conversation.subject)
    .bind(optional_id(conversation.contact_id))
    .bind(optional_id(conversation.deal_id))
    .bind(&conversation.status)
    .bind(optional_id(conversation.assignee_user_id))
    .bind(conversation.unread_count)
    .bind(conversation.last_message_at)
    .bind(conversation.last_inbound_at)
    .bind(&conversation.last_message_preview)
    .bind(&conversation.last_message_direction)
    .bind(conversation.id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn mark_conversation_read(pool: &MySqlPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE messaging_conversations SET unread_count = 0 WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('!', "!!")
        .replace('%', "!%")
        .replace('_', "!_");
    format!("%{escaped}%")
}

pub(crate) async fn list_conversations(
    pool: &MySqlPool,
    filter: &ConversationFilter,
    offset: u64,
    limit: u64,
) -> Result<(Vec<StoredConversation>, u64), AppError> {
    let mut conditions = vec!["1 = 1".to_owned()];
    let mut binds: Vec<String> = Vec::new();
    if let Some(channel_id) = filter.channel_id {
        conditions.push("channel_id = ?".to_owned());
        binds.push(channel_id.to_string());
    }
    if let Some(status) = &filter.status {
        conditions.push("status = ?".to_owned());
        binds.push(status.clone());
    }
    match &filter.assignee {
        Some(AssigneeFilter::Unassigned) => conditions.push("assignee_user_id IS NULL".to_owned()),
        Some(AssigneeFilter::User(user)) => {
            conditions.push("assignee_user_id = ?".to_owned());
            binds.push(user.to_string());
        }
        None => {}
    }
    if let Some(contact_id) = filter.contact_id {
        conditions.push("contact_id = ?".to_owned());
        binds.push(contact_id.to_string());
    }
    if let Some(deal_id) = filter.deal_id {
        conditions.push("deal_id = ?".to_owned());
        binds.push(deal_id.to_string());
    }
    if filter.unread {
        conditions.push("unread_count > 0".to_owned());
    }
    if let Some(search) = &filter.search {
        conditions.push(
            "(peer LIKE ? ESCAPE '!' OR peer_name LIKE ? ESCAPE '!' OR subject LIKE ? ESCAPE '!' OR last_message_preview LIKE ? ESCAPE '!')"
                .to_owned(),
        );
        let pattern = like_pattern(search);
        binds.extend(std::iter::repeat_n(pattern, 4));
    }
    let clause = conditions.join(" AND ");
    let mut count = sqlx::query_scalar::<_, i64>(AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM messaging_conversations WHERE {clause}"
    )));
    for bind in &binds {
        count = count.bind(bind);
    }
    let total = count.fetch_one(pool).await?;
    let mut query = sqlx::query(AssertSqlSafe(format!(
        "SELECT {CONVERSATION_COLUMNS} FROM messaging_conversations WHERE {clause} ORDER BY last_message_at DESC, id DESC LIMIT ? OFFSET ?"
    )));
    for bind in &binds {
        query = query.bind(bind);
    }
    let rows = query.bind(limit).bind(offset).fetch_all(pool).await?;
    let items = rows
        .iter()
        .map(conversation_from_row)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok((items, u64::try_from(total).unwrap_or_default()))
}

const MESSAGE_COLUMNS: &str = "id, conversation_id, channel_id, direction, external_id, sender, sender_name, subject, body, message_type, status, error_code, error_message, in_reply_to, thread_references, sent_by_user_id, provider_timestamp, sent_at, delivered_at, read_at, created_at";

fn message_from_row(row: &MySqlRow) -> Result<StoredMessage, AppError> {
    Ok(StoredMessage {
        id: uuid_column(row, "id")?,
        conversation_id: uuid_column(row, "conversation_id")?,
        channel_id: uuid_column(row, "channel_id")?,
        direction: row.try_get("direction")?,
        external_id: row.try_get("external_id")?,
        sender: row.try_get("sender")?,
        sender_name: row.try_get("sender_name")?,
        subject: row.try_get("subject")?,
        body: row.try_get("body")?,
        message_type: row.try_get("message_type")?,
        status: row.try_get("status")?,
        error_code: row.try_get("error_code")?,
        error_message: row.try_get("error_message")?,
        in_reply_to: row.try_get("in_reply_to")?,
        thread_references: row.try_get("thread_references")?,
        sent_by_user_id: optional_uuid_column(row, "sent_by_user_id")?,
        provider_timestamp: row.try_get("provider_timestamp")?,
        sent_at: row.try_get("sent_at")?,
        delivered_at: row.try_get("delivered_at")?,
        read_at: row.try_get("read_at")?,
        created_at: row.try_get("created_at")?,
    })
}

pub(crate) async fn message_exists(
    pool: &MySqlPool,
    channel_id: Uuid,
    external_id: &str,
) -> Result<bool, AppError> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT id FROM messaging_messages WHERE channel_id = ? AND external_id = ? LIMIT 1",
    )
    .bind(channel_id.to_string())
    .bind(external_id)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

pub(crate) async fn insert_message(
    tx: &mut MySqlTx,
    message: &StoredMessage,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "INSERT INTO messaging_messages (id, conversation_id, channel_id, direction, external_id, sender, sender_name, subject, body, message_type, status, error_code, error_message, in_reply_to, thread_references, sent_by_user_id, provider_timestamp, sent_at, delivered_at, read_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(message.id.to_string())
    .bind(message.conversation_id.to_string())
    .bind(message.channel_id.to_string())
    .bind(&message.direction)
    .bind(&message.external_id)
    .bind(&message.sender)
    .bind(&message.sender_name)
    .bind(&message.subject)
    .bind(&message.body)
    .bind(&message.message_type)
    .bind(&message.status)
    .bind(&message.error_code)
    .bind(&message.error_message)
    .bind(&message.in_reply_to)
    .bind(&message.thread_references)
    .bind(optional_id(message.sent_by_user_id))
    .bind(message.provider_timestamp)
    .bind(message.sent_at)
    .bind(message.delivered_at)
    .bind(message.read_at)
    .bind(message.created_at)
    .execute(&mut **tx)
    .await;
    match result {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn find_message(
    pool: &MySqlPool,
    id: Uuid,
) -> Result<Option<StoredMessage>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {MESSAGE_COLUMNS} FROM messaging_messages WHERE id = ?"
    )))
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(message_from_row).transpose()
}

pub(crate) async fn list_messages(
    pool: &MySqlPool,
    conversation_id: Uuid,
    limit: u64,
) -> Result<(Vec<StoredMessage>, u64), AppError> {
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messaging_messages WHERE conversation_id = ?")
            .bind(conversation_id.to_string())
            .fetch_one(pool)
            .await?;
    let rows = sqlx::query(AssertSqlSafe(format!(
        "SELECT {MESSAGE_COLUMNS} FROM messaging_messages WHERE conversation_id = ? ORDER BY created_at DESC, id DESC LIMIT ?"
    )))
    .bind(conversation_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut messages = rows
        .iter()
        .map(message_from_row)
        .collect::<Result<Vec<_>, AppError>>()?;
    messages.reverse();
    Ok((messages, u64::try_from(total).unwrap_or_default()))
}

pub(crate) async fn last_inbound_message(
    pool: &MySqlPool,
    conversation_id: Uuid,
) -> Result<Option<StoredMessage>, AppError> {
    let row = sqlx::query(AssertSqlSafe(format!(
        "SELECT {MESSAGE_COLUMNS} FROM messaging_messages WHERE conversation_id = ? AND direction = 'in' ORDER BY created_at DESC, id DESC LIMIT 1"
    )))
    .bind(conversation_id.to_string())
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(message_from_row).transpose()
}

pub(crate) async fn claim_message_for_sending(
    pool: &MySqlPool,
    id: Uuid,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "UPDATE messaging_messages SET status = 'sending' WHERE id = ? AND direction = 'out' AND status IN ('queued', 'sending')",
    )
    .bind(id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub(crate) async fn mark_message_sent(
    pool: &MySqlPool,
    message: &StoredMessage,
    external_id: &str,
    in_reply_to: Option<&str>,
    references: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_messages SET status = 'sent', external_id = ?, in_reply_to = ?, thread_references = ?, sent_at = ?, error_code = NULL, error_message = NULL WHERE id = ?",
    )
    .bind(external_id)
    .bind(in_reply_to)
    .bind(references)
    .bind(crate::text::now())
    .bind(message.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn mark_message_failed(
    tx: &mut MySqlTx,
    message: &StoredMessage,
    code: &str,
    error: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_messages SET status = 'failed', error_code = ?, error_message = ? WHERE id = ?",
    )
    .bind(crate::text::truncate(code, 64))
    .bind(crate::text::truncate(error, 500))
    .bind(message.id.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn record_message_attempt(
    pool: &MySqlPool,
    message: &StoredMessage,
    code: &str,
    error: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE messaging_messages SET status = 'queued', error_code = ?, error_message = ? WHERE id = ?",
    )
    .bind(crate::text::truncate(code, 64))
    .bind(crate::text::truncate(error, 500))
    .bind(message.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn apply_delivery_status(
    pool: &MySqlPool,
    channel: &StoredChannel,
    external_id: &str,
    status: &str,
    at: Option<NaiveDateTime>,
    error: Option<&str>,
) -> Result<(), AppError> {
    let at = at.unwrap_or_else(crate::text::now);
    let (sql, rank_guard) = match status {
        "delivered" => (
            "UPDATE messaging_messages SET status = 'delivered', delivered_at = COALESCE(delivered_at, ?) WHERE channel_id = ? AND external_id = ? AND direction = 'out' AND status IN ",
            "('sending', 'queued', 'sent')",
        ),
        "read" => (
            "UPDATE messaging_messages SET status = 'read', read_at = COALESCE(read_at, ?) WHERE channel_id = ? AND external_id = ? AND direction = 'out' AND status IN ",
            "('sending', 'queued', 'sent', 'delivered')",
        ),
        "sent" => (
            "UPDATE messaging_messages SET status = 'sent', sent_at = COALESCE(sent_at, ?) WHERE channel_id = ? AND external_id = ? AND direction = 'out' AND status IN ",
            "('sending', 'queued')",
        ),
        "failed" => {
            sqlx::query(
                "UPDATE messaging_messages SET status = 'failed', error_code = 'whatsapp_delivery_failed', error_message = ? WHERE channel_id = ? AND external_id = ? AND direction = 'out' AND status <> 'read'",
            )
            .bind(error.map(|error| crate::text::truncate(error, 500)))
            .bind(channel.id.to_string())
            .bind(external_id)
            .execute(pool)
            .await?;
            return Ok(());
        }
        _ => return Ok(()),
    };
    sqlx::query(AssertSqlSafe(format!("{sql}{rank_guard}")))
        .bind(at)
        .bind(channel.id.to_string())
        .bind(external_id)
        .execute(pool)
        .await?;
    Ok(())
}
