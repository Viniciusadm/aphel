use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiFeatureFlags {
    pub(crate) summary: bool,
    pub(crate) next_actions: bool,
    pub(crate) compose: bool,
    pub(crate) query: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiStatus {
    pub(crate) configured: bool,
    pub(crate) api_key_configured: bool,
    #[schema(example = "anthropic")]
    pub(crate) provider: String,
    #[schema(example = "https://api.anthropic.com")]
    pub(crate) base_url: String,
    #[schema(example = "claude-sonnet-5-5")]
    pub(crate) model: String,
    pub(crate) daily_limit: u32,
    pub(crate) used_today: u32,
    pub(crate) remaining_today: u32,
    pub(crate) user_hourly_limit: u32,
    pub(crate) send_personal_data: bool,
    pub(crate) features: AiFeatureFlags,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct AiRecordRequest {
    #[schema(example = "contacts.contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiSource {
    #[schema(example = "notes.note")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
    pub(crate) label: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiSummary {
    pub(crate) text: String,
    pub(crate) sources: Vec<AiSource>,
    pub(crate) provider: String,
    pub(crate) model: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiActionSuggestion {
    #[schema(example = "Call to confirm the meeting")]
    pub(crate) title: String,
    pub(crate) due_in_days: u32,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiNextActions {
    pub(crate) suggestions: Vec<AiActionSuggestion>,
    pub(crate) sources: Vec<AiSource>,
    pub(crate) provider: String,
    pub(crate) model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AiComposeContext {
    Note,
    Email,
    Message,
}

impl AiComposeContext {
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Self::Note => "an internal CRM note",
            Self::Email => "an e-mail to a customer",
            Self::Message => "a short chat message to a customer (WhatsApp or similar)",
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct AiComposeRequest {
    pub(crate) context: AiComposeContext,
    #[schema(example = "Make it more formal")]
    pub(crate) instruction: String,
    pub(crate) draft: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiComposition {
    pub(crate) text: String,
    pub(crate) sources: Vec<AiSource>,
    pub(crate) provider: String,
    pub(crate) model: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct AiQueryRequest {
    #[schema(example = "deals.deal")]
    pub(crate) record_type: String,
    #[schema(example = "open deals above 10k with no activity in the last 30 days")]
    pub(crate) question: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiQueryResult {
    #[schema(value_type = Object)]
    pub(crate) filter: Value,
    pub(crate) total: u64,
    pub(crate) explanation: String,
    pub(crate) provider: String,
    pub(crate) model: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiCall {
    pub(crate) id: String,
    pub(crate) user_id: Option<String>,
    #[schema(example = "summary")]
    pub(crate) feature: String,
    pub(crate) provider: String,
    pub(crate) model: String,
    #[schema(example = "success")]
    pub(crate) status: String,
    pub(crate) input_tokens: Option<u32>,
    pub(crate) output_tokens: Option<u32>,
    pub(crate) duration_ms: u32,
    pub(crate) error_code: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiUsageTotals {
    pub(crate) calls: u64,
    pub(crate) errors: u64,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiCallListMeta {
    pub(crate) total: u64,
    pub(crate) page: u32,
    pub(crate) per_page: u32,
    pub(crate) used_today: u32,
    pub(crate) daily_limit: u32,
    pub(crate) last_30_days: AiUsageTotals,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct AiCallList {
    pub(crate) data: Vec<AiCall>,
    pub(crate) meta: AiCallListMeta,
}
