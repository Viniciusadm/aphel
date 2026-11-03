use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::channels::ChannelKind;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct ChannelRequest {
    #[schema(example = "whatsapp")]
    pub(crate) kind: Option<ChannelKind>,
    #[schema(example = "Sales WhatsApp")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) is_active: Option<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) acting_user_id: Option<Uuid>,
    #[schema(value_type = Object)]
    pub(crate) config: Value,
    #[serde(default)]
    pub(crate) secrets: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) clear_secrets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ChannelResponse {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    pub(crate) kind: ChannelKind,
    pub(crate) name: String,
    pub(crate) address: String,
    pub(crate) is_active: bool,
    #[schema(example = "active")]
    pub(crate) status: String,
    pub(crate) last_error: Option<String>,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) last_polled_at: Option<String>,
    #[schema(value_type = Option<String>)]
    pub(crate) acting_user_id: Option<Uuid>,
    #[schema(value_type = String)]
    pub(crate) created_by: Uuid,
    #[schema(value_type = Option<Object>)]
    pub(crate) config: Option<Value>,
    pub(crate) secrets: Option<BTreeMap<String, bool>>,
    pub(crate) webhook_url: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ChannelTestResult {
    pub(crate) ok: bool,
    pub(crate) detail: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConversationStatus {
    Open,
    Pending,
    Resolved,
}

impl ConversationStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pending => "pending",
            Self::Resolved => "resolved",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "pending" => Some(Self::Pending),
            "resolved" => Some(Self::Resolved),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LinkedRecord {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    pub title: Option<String>,
    pub(crate) accessible: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ConversationChannel {
    pub(crate) available: bool,
    pub(crate) uses_references: bool,
    pub(crate) max_body_chars: usize,
    pub(crate) reply_window_seconds: Option<i64>,
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) kind: ChannelKind,
    pub(crate) address: String,
    pub(crate) is_active: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ConversationSummary {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    pub(crate) channel: ConversationChannel,
    pub(crate) peer: String,
    pub(crate) peer_name: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) status: ConversationStatus,
    #[schema(value_type = Option<String>)]
    pub(crate) assignee_user_id: Option<Uuid>,
    pub(crate) unread_count: u32,
    pub(crate) last_message_at: String,
    pub(crate) last_message_preview: Option<String>,
    pub(crate) last_message_direction: Option<String>,
    pub(crate) contact: Option<LinkedRecord>,
    pub(crate) deal: Option<LinkedRecord>,
    pub(crate) reply_window_ends_at: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct MessageItem {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    #[schema(example = "in")]
    pub(crate) direction: String,
    pub(crate) sender: String,
    pub(crate) sender_name: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) body: String,
    #[schema(example = "text")]
    pub(crate) message_type: String,
    #[schema(example = "delivered")]
    pub(crate) status: String,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    #[schema(value_type = Option<String>)]
    pub(crate) sent_by_user_id: Option<Uuid>,
    pub(crate) provider_timestamp: Option<String>,
    pub(crate) sent_at: Option<String>,
    pub(crate) delivered_at: Option<String>,
    pub(crate) read_at: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct ConversationDetail {
    pub(crate) conversation: ConversationSummary,
    pub(crate) messages: Vec<MessageItem>,
    pub(crate) total_messages: u64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct ReplyRequest {
    pub(crate) body: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct AssignRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct StatusRequest {
    pub(crate) status: ConversationStatus,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct LinkRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) contact_id: Option<Uuid>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) deal_id: Option<Uuid>,
}
