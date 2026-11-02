use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::capture::{AddBlockCommand, UpdateCaptureSettingsCommand};
use crate::compose::ComposedEmail;
use crate::domain::EmailAccess;
use crate::mailboxes::{
    ConnectMailboxCommand, MailProviderInfo, MailboxDetails, UpdateMailboxCommand,
};
use crate::reading::{MessageDetail, ThreadDetail, ThreadSummary};
use crate::sync::{BlockedEntry, CaptureSettings};
use crate::{EmailAttachmentView, MailboxConnectionKind, ParticipantView, RecordLinkView};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::common::timestamp as instant;

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MailProviderConnectionResponse {
    #[schema(example = "credentials")]
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_path: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MailProviderResponse {
    #[schema(example = "imap")]
    pub(crate) key: String,
    #[schema(example = "IMAP/SMTP")]
    pub(crate) label: String,
    pub(crate) connection: MailProviderConnectionResponse,
}

impl From<MailProviderInfo> for MailProviderResponse {
    fn from(provider: MailProviderInfo) -> Self {
        let connection = match provider.connection {
            MailboxConnectionKind::Credentials => MailProviderConnectionResponse {
                kind: "credentials".to_owned(),
                start_path: None,
            },
            MailboxConnectionKind::OAuth { start_path } => MailProviderConnectionResponse {
                kind: "oauth".to_owned(),
                start_path: Some(start_path),
            },
        };
        Self {
            key: provider.key,
            label: provider.label,
            connection,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MailboxResponse {
    pub(crate) id: String,
    #[schema(example = "imap")]
    pub(crate) provider: String,
    pub(crate) provider_label: String,
    #[schema(example = "ana@example.com")]
    pub(crate) address: String,
    pub(crate) display_name: Option<String>,
    #[schema(example = "active")]
    pub(crate) status: String,
    pub(crate) status_reason: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) settings: Value,
    pub(crate) credentials_configured: bool,
    pub(crate) history_days: u32,
    #[schema(example = "metadata")]
    pub(crate) default_visibility: String,
    pub(crate) last_synced_at: Option<String>,
    pub(crate) sync_started_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<MailboxDetails> for MailboxResponse {
    fn from(details: MailboxDetails) -> Self {
        let mailbox = details.mailbox;
        Self {
            id: mailbox.id.to_string(),
            provider: mailbox.provider,
            provider_label: details.provider_label,
            address: mailbox.address,
            display_name: mailbox.display_name,
            status: mailbox.status.as_str().to_owned(),
            status_reason: mailbox.status_reason,
            settings: mailbox.settings,
            credentials_configured: details.credentials_configured,
            history_days: mailbox.history_days,
            default_visibility: mailbox.default_visibility.as_str().to_owned(),
            last_synced_at: mailbox.last_synced_at.map(instant),
            sync_started_at: mailbox.sync_started_at.map(instant),
            created_at: instant(mailbox.created_at),
            updated_at: instant(mailbox.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateMailboxRequest {
    #[schema(example = "imap")]
    pub(crate) provider: String,
    pub(crate) display_name: Option<String>,
    pub(crate) history_days: Option<u32>,
    #[schema(example = "metadata")]
    pub(crate) default_visibility: Option<String>,
    #[schema(value_type = Object, example = json!({"imap_host": "imap.example.com", "imap_port": 993, "imap_security": "tls", "smtp_host": "smtp.example.com", "smtp_port": 465, "smtp_security": "tls", "username": "ana@example.com"}))]
    pub(crate) settings: Option<Value>,
    #[schema(value_type = Object, example = json!({"password": "secret"}))]
    #[serde(default)]
    pub(crate) credentials: Map<String, Value>,
}

impl CreateMailboxRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ConnectMailboxCommand {
        ConnectMailboxCommand {
            provider: self.provider,
            display_name: self.display_name,
            history_days: self.history_days,
            default_visibility: self.default_visibility,
            settings: self.settings,
            credentials: self.credentials,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateMailboxRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) display_name: Option<Option<String>>,
    pub(crate) default_visibility: Option<String>,
    pub(crate) history_days: Option<u32>,
    #[schema(value_type = Option<Object>)]
    pub(crate) settings: Option<Value>,
    #[schema(value_type = Option<Object>)]
    pub(crate) credentials: Option<Map<String, Value>>,
}

impl UpdateMailboxRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> UpdateMailboxCommand {
        UpdateMailboxCommand {
            display_name: self.display_name,
            default_visibility: self.default_visibility,
            history_days: self.history_days,
            settings: self.settings,
            credentials: self.credentials,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailParticipantResponse {
    #[schema(example = "from")]
    pub(crate) role: String,
    pub(crate) address: String,
    pub(crate) name: Option<String>,
    pub(crate) contact_id: Option<String>,
}

impl From<ParticipantView> for EmailParticipantResponse {
    fn from(participant: ParticipantView) -> Self {
        Self {
            role: participant.role.as_str().to_owned(),
            address: participant.address,
            name: participant.name,
            contact_id: participant.contact_id.map(|id| id.to_string()),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailThreadRecordResponse {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    #[schema(example = "automatic")]
    pub(crate) origin: String,
}

impl From<RecordLinkView> for EmailThreadRecordResponse {
    fn from(link: RecordLinkView) -> Self {
        Self {
            record_type: link.record.record_type.as_str().to_owned(),
            record_id: link.record.record_id.to_string(),
            origin: link.origin.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailThreadOwnerResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailThreadResponse {
    pub(crate) id: String,
    pub(crate) mailbox_id: String,
    pub(crate) owner: EmailThreadOwnerResponse,
    pub(crate) mailbox_address: String,
    pub(crate) subject: String,
    #[schema(example = "metadata")]
    pub(crate) visibility: String,
    pub(crate) visibility_is_default: bool,
    #[schema(example = "full")]
    pub(crate) access: String,
    pub(crate) message_count: u32,
    pub(crate) snippet: Option<String>,
    pub(crate) last_direction: Option<String>,
    pub(crate) first_message_at: Option<String>,
    pub(crate) last_message_at: Option<String>,
    pub(crate) participants: Vec<EmailParticipantResponse>,
    pub(crate) records: Vec<EmailThreadRecordResponse>,
}

fn access_name(access: EmailAccess) -> &'static str {
    if access.can_read_content() {
        "full"
    } else {
        "metadata"
    }
}

impl From<ThreadSummary> for EmailThreadResponse {
    fn from(summary: ThreadSummary) -> Self {
        let thread = summary.thread;
        let full = summary.access.can_read_content();
        Self {
            id: thread.id.to_string(),
            mailbox_id: thread.mailbox_id.to_string(),
            owner: EmailThreadOwnerResponse {
                id: thread.owner.id.as_str().to_owned(),
                display_name: thread.owner.display_name,
            },
            mailbox_address: thread.mailbox_address,
            subject: thread.subject,
            visibility: thread.visibility.as_str().to_owned(),
            visibility_is_default: thread.visibility_override.is_none(),
            access: access_name(summary.access).to_owned(),
            message_count: thread.message_count,
            snippet: full.then_some(thread.snippet),
            last_direction: thread
                .last_direction
                .map(|direction| direction.as_str().to_owned()),
            first_message_at: thread.first_message_at.map(instant),
            last_message_at: thread.last_message_at.map(instant),
            participants: summary.participants.into_iter().map(Into::into).collect(),
            records: summary.records.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailBodyResponse {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailAttachmentResponse {
    pub(crate) id: String,
    pub(crate) file_id: String,
    pub(crate) name: String,
    pub(crate) mime_type: String,
    pub(crate) size: u64,
    pub(crate) inline: bool,
}

impl From<EmailAttachmentView> for EmailAttachmentResponse {
    fn from(attachment: EmailAttachmentView) -> Self {
        Self {
            id: attachment.id.to_string(),
            file_id: attachment.file_id.to_string(),
            name: attachment.name,
            mime_type: attachment.mime_type,
            size: attachment.size,
            inline: attachment.inline,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailTrackingResponse {
    pub(crate) enabled: bool,
    pub(crate) opened_at: Option<String>,
    pub(crate) open_count: u32,
    pub(crate) clicked_at: Option<String>,
    pub(crate) click_count: u32,
    pub(crate) replied_at: Option<String>,
    pub(crate) bounced_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailMessageResponse {
    pub(crate) id: String,
    pub(crate) thread_id: String,
    #[schema(example = "inbound")]
    pub(crate) direction: String,
    #[schema(example = "message")]
    pub(crate) kind: String,
    #[schema(example = "received")]
    pub(crate) status: String,
    pub(crate) subject: String,
    pub(crate) preview: Option<String>,
    pub(crate) sent_at: Option<String>,
    pub(crate) scheduled_at: Option<String>,
    pub(crate) participants: Vec<EmailParticipantResponse>,
    pub(crate) body: Option<EmailBodyResponse>,
    pub(crate) attachments: Vec<EmailAttachmentResponse>,
    pub(crate) tracking: Option<EmailTrackingResponse>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_by: Option<String>,
}

impl EmailMessageResponse {
    pub(crate) fn build(detail: MessageDetail, access: EmailAccess, is_owner: bool) -> Self {
        let message = detail.message;
        let full = access.can_read_content();
        Self {
            id: message.id.to_string(),
            thread_id: message.thread_id.to_string(),
            direction: message.direction.as_str().to_owned(),
            kind: message.kind.as_str().to_owned(),
            status: message.status.as_str().to_owned(),
            subject: message.subject,
            preview: full.then_some(message.preview),
            sent_at: message.sent_at.map(instant),
            scheduled_at: message.scheduled_at.map(instant),
            participants: message.participants.into_iter().map(Into::into).collect(),
            body: if full {
                detail.body.map(|body| EmailBodyResponse {
                    text: body.text,
                    html: body.html,
                })
            } else {
                None
            },
            attachments: if full {
                message.attachments.into_iter().map(Into::into).collect()
            } else {
                Vec::new()
            },
            tracking: is_owner.then(|| EmailTrackingResponse {
                enabled: message.tracking_enabled,
                opened_at: message.opened_at.map(instant),
                open_count: message.open_count,
                clicked_at: message.clicked_at.map(instant),
                click_count: message.click_count,
                replied_at: message.replied_at.map(instant),
                bounced_at: message.bounced_at.map(instant),
            }),
            failure_reason: if is_owner {
                message.failure_reason
            } else {
                None
            },
            created_by: message.created_by.map(|user| user.as_str().to_owned()),
        }
    }
}

impl From<ComposedEmail> for EmailMessageResponse {
    fn from(composed: ComposedEmail) -> Self {
        Self::build(
            MessageDetail {
                message: composed.message,
                body: Some(composed.body),
            },
            EmailAccess::Full,
            true,
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailThreadDetailResponse {
    #[serde(flatten)]
    pub(crate) thread: EmailThreadResponse,
    pub(crate) messages: Vec<EmailMessageResponse>,
}

impl From<ThreadDetail> for EmailThreadDetailResponse {
    fn from(detail: ThreadDetail) -> Self {
        let access = detail.summary.access;
        let is_owner = detail.summary.is_owner;
        Self {
            messages: detail
                .messages
                .into_iter()
                .map(|message| EmailMessageResponse::build(message, access, is_owner))
                .collect(),
            thread: detail.summary.into(),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct EmailThreadsQueryParams {
    #[param(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct MyEmailThreadsQueryParams {
    pub(crate) mailbox_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateEmailThreadRequest {
    #[schema(example = "shared")]
    pub(crate) visibility: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct LinkEmailThreadRequest {
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct EmailCaptureSettingsDto {
    pub(crate) create_contacts: bool,
    pub(crate) create_companies: bool,
    pub(crate) link_open_deals: bool,
    pub(crate) initial_history_days: u32,
}

impl From<CaptureSettings> for EmailCaptureSettingsDto {
    fn from(settings: CaptureSettings) -> Self {
        Self {
            create_contacts: settings.create_contacts,
            create_companies: settings.create_companies,
            link_open_deals: settings.link_open_deals,
            initial_history_days: settings.initial_history_days,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateEmailCaptureSettingsRequest {
    pub(crate) create_contacts: Option<bool>,
    pub(crate) create_companies: Option<bool>,
    pub(crate) link_open_deals: Option<bool>,
    pub(crate) initial_history_days: Option<u32>,
}

impl UpdateEmailCaptureSettingsRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> UpdateCaptureSettingsCommand {
        UpdateCaptureSettingsCommand {
            create_contacts: self.create_contacts,
            create_companies: self.create_companies,
            link_open_deals: self.link_open_deals,
            initial_history_days: self.initial_history_days,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailBlockResponse {
    pub(crate) id: String,
    #[schema(example = "domain")]
    pub(crate) kind: String,
    #[schema(example = "newsletter.example.com")]
    pub(crate) value: String,
    pub(crate) reason: Option<String>,
    pub(crate) created_at: String,
}

impl From<BlockedEntry> for EmailBlockResponse {
    fn from(entry: BlockedEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            kind: entry.kind.as_str().to_owned(),
            value: entry.value,
            reason: entry.reason,
            created_at: instant(entry.created_at),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateEmailBlockRequest {
    #[schema(example = "domain")]
    pub(crate) kind: String,
    #[schema(example = "newsletter.example.com")]
    pub(crate) value: String,
    pub(crate) reason: Option<String>,
}

impl CreateEmailBlockRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> AddBlockCommand {
        AddBlockCommand {
            kind: self.kind,
            value: self.value,
            reason: self.reason,
            ip,
        }
    }
}
