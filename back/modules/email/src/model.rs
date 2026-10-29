use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::{
    EmailDirection, EmailKind, EmailStatus, EmailVisibility, LinkOrigin, MailboxStatus,
    ParticipantRole,
};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::{Secret, UserId};
use aphel_kernel::errors::AppError;

pub(crate) const MAILBOX_SECRET_OWNER: &str = "mailbox";
pub(crate) const MAILBOX_CREDENTIALS_SECRET: &str = "credentials";
pub const IMAP_PROVIDER: &str = "imap";

#[derive(Clone, PartialEq, Default)]
pub struct MailboxCredentials(pub Map<String, Value>);

impl MailboxCredentials {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    pub(crate) fn to_secret(&self) -> Secret {
        Secret::new(Value::Object(self.0.clone()).to_string())
    }

    pub(crate) fn from_secret(secret: &Secret) -> Result<Self, AppError> {
        match serde_json::from_str::<Value>(secret.expose()) {
            Ok(Value::Object(values)) => Ok(Self(values)),
            _ => Err(AppError::Internal(
                "stored mailbox credentials are not a JSON object".to_owned(),
            )),
        }
    }
}

impl std::fmt::Debug for MailboxCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailboxCredentials").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
pub(crate) trait CredentialSink: Send + Sync {
    async fn persist(
        &self,
        mailbox_id: Uuid,
        credentials: &MailboxCredentials,
    ) -> Result<(), AppError>;
}

#[derive(Clone, Default)]
pub struct CredentialPersistence(Option<std::sync::Arc<dyn CredentialSink>>);

impl CredentialPersistence {
    pub(crate) fn new(sink: std::sync::Arc<dyn CredentialSink>) -> Self {
        Self(Some(sink))
    }
}

impl std::fmt::Debug for CredentialPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CredentialPersistence")
            .field(&self.0.is_some())
            .finish()
    }
}

impl PartialEq for CredentialPersistence {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MailboxAccount {
    pub mailbox_id: Uuid,
    pub user_id: Uuid,
    pub address: String,
    pub settings: Value,
    pub credentials: MailboxCredentials,
    pub persistence: CredentialPersistence,
}

impl MailboxAccount {
    pub async fn persist_refreshed(&self, credentials: &MailboxCredentials) {
        let Some(sink) = &self.persistence.0 else {
            return;
        };
        if let Err(error) = sink.persist(self.mailbox_id, credentials).await {
            tracing::warn!(
                mailbox_id = %self.mailbox_id,
                "could not persist refreshed mailbox credentials: {error}"
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailboxConnectionKind {
    Credentials,
    OAuth { start_path: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MailboxConnectRequest {
    pub(crate) user_id: Uuid,
    pub(crate) settings: Value,
    pub credentials: MailboxCredentials,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedMailbox {
    pub address: String,
    pub display_name: Option<String>,
    pub settings: Value,
    pub credentials: MailboxCredentials,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAddress {
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct EmailAttachmentContent {
    pub name: String,
    pub mime_type: String,
    pub content: Vec<u8>,
    pub content_id: Option<String>,
    pub inline: bool,
}

impl std::fmt::Debug for EmailAttachmentContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAttachmentContent")
            .field("name", &self.name)
            .field("mime_type", &self.mime_type)
            .field("size", &self.content.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct FetchedEmail {
    pub provider_message_id: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub subject: String,
    pub from: Option<EmailAddress>,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub reply_to: Vec<EmailAddress>,
    pub date: Option<DateTime<Utc>>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Vec<EmailAttachmentContent>,
    pub content_type: Option<String>,
    pub auto_submitted: Option<String>,
    pub report_for_message_id: Option<String>,
    pub sent_by_account: bool,
}

impl std::fmt::Debug for FetchedEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchedEmail")
            .field("provider_message_id", &self.provider_message_id)
            .field("message_id", &self.message_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncRequest {
    pub cursor: Option<Value>,
    pub since: DateTime<Utc>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncBatch {
    pub messages: Vec<FetchedEmail>,
    pub cursor: Value,
    pub has_more: bool,
    pub credentials: Option<MailboxCredentials>,
}

#[derive(Clone, PartialEq)]
pub struct ProviderEmail {
    pub message_id: String,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub reply_to: Option<EmailAddress>,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub attachments: Vec<EmailAttachmentContent>,
    pub headers: Vec<(String, String)>,
}

impl std::fmt::Debug for ProviderEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderEmail")
            .field("message_id", &self.message_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SentEmail {
    pub message_id: String,
    pub provider_message_id: Option<String>,
    pub credentials: Option<MailboxCredentials>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct EmailBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) html: Option<String>,
}

impl EmailBody {
    pub(crate) const MIME_TYPE: &'static str = "application/vnd.aphel.email-body+json";

    pub(crate) fn encode(&self) -> Result<Vec<u8>, AppError> {
        serde_json::to_vec(self).map_err(|error| AppError::Internal(error.to_string()))
    }

    pub(crate) fn decode(content: &[u8]) -> Result<Self, AppError> {
        serde_json::from_slice(content).map_err(|error| AppError::Internal(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MailboxView {
    pub(crate) id: Uuid,
    pub(crate) owner: UserId,
    pub(crate) provider: String,
    pub(crate) address: String,
    pub(crate) display_name: Option<String>,
    pub(crate) status: MailboxStatus,
    pub(crate) status_reason: Option<String>,
    pub(crate) settings: Value,
    pub(crate) sync_cursor: Option<Value>,
    pub(crate) history_days: u32,
    pub(crate) default_visibility: EmailVisibility,
    pub(crate) sync_started_at: Option<DateTime<Utc>>,
    pub(crate) last_synced_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl MailboxView {
    pub(crate) fn is_owned_by(&self, user_id: &UserId) -> bool {
        &self.owner == user_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewMailbox {
    pub(crate) id: Uuid,
    pub(crate) owner: UserId,
    pub(crate) provider: String,
    pub(crate) address: String,
    pub(crate) display_name: Option<String>,
    pub(crate) settings: Value,
    pub(crate) history_days: u32,
    pub(crate) default_visibility: EmailVisibility,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct MailboxChanges {
    pub(crate) display_name: Option<Option<String>>,
    pub(crate) settings: Option<Value>,
    pub(crate) history_days: Option<u32>,
    pub(crate) default_visibility: Option<EmailVisibility>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MailboxSyncState {
    pub(crate) status: MailboxStatus,
    pub(crate) status_reason: Option<String>,
    pub(crate) cursor: Option<Value>,
    pub(crate) sync_started_at: Option<DateTime<Utc>>,
    pub(crate) last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueMailbox {
    pub(crate) mailbox_id: Uuid,
    pub(crate) owner: UserId,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordLinkView {
    pub(crate) id: Uuid,
    pub(crate) thread_id: Uuid,
    pub(crate) record: RecordRef,
    pub(crate) origin: LinkOrigin,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewRecordLink {
    pub(crate) thread_id: Uuid,
    pub(crate) record: RecordRef,
    pub(crate) origin: LinkOrigin,
    pub(crate) created_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadView {
    pub(crate) id: Uuid,
    pub(crate) mailbox_id: Uuid,
    pub(crate) owner: UserReference,
    pub(crate) mailbox_address: String,
    pub(crate) subject: String,
    pub(crate) visibility: EmailVisibility,
    pub(crate) visibility_override: Option<EmailVisibility>,
    pub(crate) message_count: u32,
    pub(crate) snippet: String,
    pub(crate) last_direction: Option<EmailDirection>,
    pub(crate) first_message_at: Option<DateTime<Utc>>,
    pub(crate) last_message_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ThreadView {
    pub(crate) fn is_owned_by(&self, user_id: &UserId) -> bool {
        &self.owner.id == user_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewThread {
    pub(crate) id: Uuid,
    pub(crate) mailbox_id: Uuid,
    pub(crate) subject: String,
    pub(crate) visibility: Option<EmailVisibility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParticipantView {
    pub(crate) role: ParticipantRole,
    pub(crate) address: String,
    pub(crate) name: Option<String>,
    pub(crate) contact_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewParticipant {
    pub(crate) role: ParticipantRole,
    pub(crate) address: String,
    pub(crate) name: Option<String>,
    pub(crate) contact_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmailAttachmentView {
    pub(crate) id: Uuid,
    pub(crate) file_id: Uuid,
    pub(crate) name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
    pub(crate) content_id: Option<String>,
    pub(crate) inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewEmailAttachment {
    pub(crate) file_id: Uuid,
    pub(crate) name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
    pub(crate) content_id: Option<String>,
    pub(crate) inline: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmailMessageView {
    pub id: Uuid,
    pub mailbox_id: Uuid,
    pub thread_id: Uuid,
    pub(crate) direction: EmailDirection,
    pub(crate) kind: EmailKind,
    pub(crate) status: EmailStatus,
    pub(crate) message_id: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) references: Vec<String>,
    pub(crate) report_for_message_id: Option<String>,
    pub(crate) provider_message_id: Option<String>,
    pub(crate) subject: String,
    pub(crate) preview: String,
    pub(crate) body_file_id: Option<Uuid>,
    pub(crate) has_html: bool,
    pub(crate) has_attachments: bool,
    pub(crate) tracking_enabled: bool,
    pub(crate) sent_at: Option<DateTime<Utc>>,
    pub(crate) scheduled_at: Option<DateTime<Utc>>,
    pub(crate) opened_at: Option<DateTime<Utc>>,
    pub(crate) open_count: u32,
    pub(crate) clicked_at: Option<DateTime<Utc>>,
    pub(crate) click_count: u32,
    pub(crate) replied_at: Option<DateTime<Utc>>,
    pub(crate) bounced_at: Option<DateTime<Utc>>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) participants: Vec<ParticipantView>,
    pub(crate) attachments: Vec<EmailAttachmentView>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl EmailMessageView {
    pub(crate) fn participants_with(
        &self,
        role: ParticipantRole,
    ) -> impl Iterator<Item = &ParticipantView> {
        self.participants
            .iter()
            .filter(move |participant| participant.role == role)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewEmailMessage {
    pub(crate) id: Uuid,
    pub(crate) mailbox_id: Uuid,
    pub(crate) thread_id: Uuid,
    pub(crate) direction: EmailDirection,
    pub(crate) kind: EmailKind,
    pub(crate) status: EmailStatus,
    pub(crate) message_id: String,
    pub(crate) in_reply_to: Option<String>,
    pub(crate) references: Vec<String>,
    pub(crate) report_for_message_id: Option<String>,
    pub(crate) provider_message_id: Option<String>,
    pub(crate) subject: String,
    pub(crate) preview: String,
    pub(crate) body_file_id: Option<Uuid>,
    pub(crate) has_html: bool,
    pub(crate) tracking_enabled: bool,
    pub(crate) sent_at: Option<DateTime<Utc>>,
    pub(crate) scheduled_at: Option<DateTime<Utc>>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) participants: Vec<NewParticipant>,
    pub(crate) attachments: Vec<NewEmailAttachment>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct MessageStatusChange {
    pub(crate) status: Option<EmailStatus>,
    pub(crate) message_id: Option<String>,
    pub(crate) provider_message_id: Option<Option<String>>,
    pub(crate) sent_at: Option<Option<DateTime<Utc>>>,
    pub(crate) scheduled_at: Option<Option<DateTime<Utc>>>,
    pub(crate) failure_reason: Option<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngagementKind {
    Opened,
    Clicked,
    Replied,
    Bounced,
}
