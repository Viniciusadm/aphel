use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) const MAX_FIELDS: usize = 30;
pub(crate) const MAX_OPTIONS: usize = 50;
pub(crate) const MAX_ORIGINS: usize = 20;
pub(crate) const MAX_NAME_LENGTH: usize = 120;
pub(crate) const MAX_LABEL_LENGTH: usize = 120;
pub(crate) const MAX_KEY_LENGTH: usize = 40;
pub(crate) const MAX_OPTION_LENGTH: usize = 100;
pub(crate) const MAX_PLACEHOLDER_LENGTH: usize = 120;
pub(crate) const MAX_TEMPLATE_LENGTH: usize = 200;
pub(crate) const MAX_MESSAGE_LENGTH: usize = 500;
pub(crate) const MAX_URL_LENGTH: usize = 500;
pub(crate) const MAX_SOURCE_LENGTH: usize = 120;
pub(crate) const MAX_SUBMIT_LABEL_LENGTH: usize = 60;
pub(crate) const MAX_TEXT_LENGTH: usize = 500;
pub(crate) const MAX_TEXTAREA_LENGTH: usize = 5000;
pub(crate) const FULL_NAME_TARGET: &str = "name";
pub(crate) const HONEYPOT_FIELD: &str = "_aphel_hp";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeadFormFieldKind {
    Text,
    Email,
    Phone,
    Textarea,
    Select,
    Checkbox,
}

impl LeadFormFieldKind {
    pub(crate) fn max_length(&self) -> usize {
        match self {
            Self::Textarea => MAX_TEXTAREA_LENGTH,
            _ => MAX_TEXT_LENGTH,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub(crate) struct LeadFormOption {
    #[schema(example = "website")]
    pub(crate) value: String,
    #[schema(example = "Website")]
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub(crate) struct LeadFormField {
    #[schema(example = "email")]
    pub(crate) key: String,
    #[schema(example = "Your email")]
    pub(crate) label: String,
    pub(crate) kind: LeadFormFieldKind,
    #[serde(default)]
    pub(crate) required: bool,
    #[serde(default)]
    pub(crate) placeholder: Option<String>,
    #[serde(default)]
    pub(crate) options: Vec<LeadFormOption>,
    #[serde(default)]
    #[schema(example = "email")]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub(crate) struct LeadFormDeal {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: String,
    #[schema(example = "{{name}} - website form")]
    pub(crate) title_template: String,
    #[serde(default)]
    #[schema(example = "1500.00")]
    pub(crate) amount: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub(crate) struct LeadFormDefinition {
    #[serde(default)]
    pub(crate) fields: Vec<LeadFormField>,
    #[serde(default)]
    pub(crate) deal: Option<LeadFormDeal>,
    #[serde(default)]
    pub(crate) tag_id: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) owner_id: Option<Uuid>,
    #[serde(default)]
    pub(crate) distribution_rule_id: Option<String>,
    #[serde(default)]
    #[schema(example = "Website form")]
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) success_message: Option<String>,
    #[serde(default)]
    pub(crate) redirect_url: Option<String>,
    #[serde(default)]
    pub(crate) allowed_origins: Vec<String>,
    #[serde(default)]
    pub(crate) dedupe_by_email: bool,
    #[serde(default)]
    pub(crate) submit_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct LeadFormRequest {
    #[schema(example = "Website contact")]
    pub(crate) name: String,
    #[serde(default = "default_true")]
    pub(crate) is_active: bool,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub(crate) acting_user_id: Option<Uuid>,
    #[serde(flatten)]
    pub(crate) definition: LeadFormDefinition,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormResponse {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    #[schema(example = "f_3b0d1c5e9a8f4f6e8b7a6c5d4e3f2a1b")]
    pub(crate) public_id: String,
    pub(crate) name: String,
    pub(crate) is_active: bool,
    #[schema(value_type = Option<String>)]
    pub(crate) acting_user_id: Option<Uuid>,
    #[schema(value_type = String)]
    pub(crate) created_by: Uuid,
    #[serde(flatten)]
    pub(crate) definition: LeadFormDefinition,
    pub(crate) accepted_count: i64,
    pub(crate) spam_count: i64,
    pub(crate) failed_count: i64,
    pub(crate) last_submission_at: Option<String>,
    #[schema(example = "/api/v1/public/lead-forms/forms/f_3b0d/page")]
    pub(crate) page_path: String,
    #[schema(example = "/api/v1/public/lead-forms/forms/f_3b0d/embed.js")]
    pub(crate) embed_path: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LeadFormSubmissionStatus {
    Accepted,
    Spam,
    Failed,
}

impl LeadFormSubmissionStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Spam => "spam",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        [Self::Accepted, Self::Spam, Self::Failed]
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormSubmission {
    #[schema(value_type = String)]
    pub(crate) id: Uuid,
    #[schema(value_type = String)]
    pub(crate) form_id: Uuid,
    pub(crate) status: LeadFormSubmissionStatus,
    #[schema(value_type = Option<String>)]
    pub(crate) contact_id: Option<Uuid>,
    #[schema(value_type = Option<String>)]
    pub(crate) deal_id: Option<Uuid>,
    pub(crate) contact_created: bool,
    #[schema(value_type = Object)]
    pub(crate) data: Value,
    pub(crate) ip_address: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormTargetField {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) field_type: String,
    pub(crate) custom: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormStageOption {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormPipelineOption {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) is_default: bool,
    pub(crate) stages: Vec<LeadFormStageOption>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormTagOption {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) color: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormUserOption {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormOptions {
    pub(crate) contact_fields: Vec<LeadFormTargetField>,
    pub(crate) workflow: Vec<LeadFormPipelineOption>,
    pub(crate) tags: Vec<LeadFormTagOption>,
    pub(crate) users: Vec<LeadFormUserOption>,
    pub(crate) distribution_rules: Vec<LeadFormDistributionRuleOption>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormDistributionRuleOption {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormPublicField {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: LeadFormFieldKind,
    pub(crate) required: bool,
    pub(crate) placeholder: Option<String>,
    pub(crate) options: Vec<LeadFormOption>,
    pub(crate) max_length: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormPublicDefinition {
    pub(crate) public_id: String,
    pub(crate) name: String,
    pub(crate) fields: Vec<LeadFormPublicField>,
    pub(crate) submit_label: Option<String>,
    pub(crate) success_message: Option<String>,
    pub(crate) redirect_url: Option<String>,
    #[schema(example = "_aphel_hp")]
    pub(crate) honeypot_field: String,
    pub(crate) submit_path: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct LeadFormSubmitResult {
    pub(crate) status: String,
    pub(crate) success_message: Option<String>,
    pub(crate) redirect_url: Option<String>,
}

pub(crate) fn public_field(field: &LeadFormField) -> LeadFormPublicField {
    LeadFormPublicField {
        key: field.key.clone(),
        label: field.label.clone(),
        kind: field.kind,
        required: field.required,
        placeholder: field.placeholder.clone(),
        options: field.options.clone(),
        max_length: field.kind.max_length(),
    }
}
