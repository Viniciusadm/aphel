use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::AppError;

pub(crate) const MAX_TEMPLATE_NAME_LENGTH: usize = 200;
pub(crate) const MAX_TEMPLATE_SUBJECT_LENGTH: usize = 998;
pub(crate) const MAX_TEMPLATE_BODY_BYTES: usize = 512 * 1024;
pub(crate) const EMAIL_TEMPLATE_NOT_FOUND: &str = "email_template_not_found";
pub(crate) const EMAIL_TEMPLATE_KEY_TAKEN: &str = "email_template_key_taken";
pub(crate) const EMAIL_TEMPLATE_CREATED: &str = "email_template.created";
pub(crate) const EMAIL_TEMPLATE_UPDATED: &str = "email_template.updated";
pub(crate) const EMAIL_TEMPLATE_DELETED: &str = "email_template.deleted";
pub(crate) const EMAIL_TEMPLATE_RECORD: RecordType =
    RecordType::from_static("email.email_template");

pub(crate) fn template_not_found() -> AppError {
    AppError::not_found_code(EMAIL_TEMPLATE_NOT_FOUND, "e-mail template was not found")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TemplateScope {
    Organization,
    Personal,
}

impl TemplateScope {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::Personal => "personal",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "organization" => Some(Self::Organization),
            "personal" => Some(Self::Personal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TemplateListScope {
    #[default]
    All,
    Organization,
    Personal,
}

impl TemplateListScope {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "organization" => Some(Self::Organization),
            "personal" => Some(Self::Personal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateFilter {
    pub(crate) scope: TemplateListScope,
    pub(crate) viewer: UserId,
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailTemplateView {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) scope: TemplateScope,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) record_type: Option<String>,
    pub(crate) subject: String,
    pub(crate) body_html: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl EmailTemplateView {
    pub(crate) fn is_visible_to(&self, user: &UserId) -> bool {
        match self.scope {
            TemplateScope::Organization => true,
            TemplateScope::Personal => self.owner_id.as_ref() == Some(user),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewEmailTemplate {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) scope: TemplateScope,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) record_type: Option<String>,
    pub(crate) subject: String,
    pub(crate) body_html: String,
    pub(crate) created_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateTemplateCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) scope: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) subject: String,
    pub(crate) body_html: String,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UpdateTemplateCommand {
    pub(crate) key: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) record_type: Option<Option<String>>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableField {
    pub(crate) key: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateVariable {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) record_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreviewCommand {
    pub template_id: Option<String>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
    pub subject: String,
    pub(crate) html: String,
    pub text: String,
    pub(crate) missing_variables: Vec<String>,
}

pub(crate) fn is_email_record_type(record_type: &str) -> bool {
    matches!(
        record_type,
        "contacts.contact" | "companies.company" | "deals.deal"
    ) || is_custom_record_type(record_type)
}
