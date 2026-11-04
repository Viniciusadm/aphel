use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{
    EmailDirection, EmailKind, EmailStatus, EmailVisibility, LinkOrigin, MailboxStatus,
    ParticipantRole,
};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;

pub(crate) const MAILBOX_SECRET_OWNER: &str = "mailbox";
pub(crate) const MAILBOX_CREDENTIALS_SECRET: &str = "credentials";
pub const IMAP_PROVIDER: &str = "imap";

pub use aphel_sdk::email::mailbox::{
    ConnectedMailbox, CredentialPersistence, CredentialSink, EmailAddress, EmailAttachmentContent,
    FetchedEmail, MailboxAccount, MailboxConnectRequest, MailboxConnectionKind, MailboxCredentials,
    ProviderEmail, SentEmail, SyncBatch, SyncRequest,
};

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
