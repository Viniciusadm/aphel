use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::EmailAddress;
use crate::compose::{RecordInput, SendEmailCommand};
use crate::templates::{
    CreateTemplateCommand, EmailTemplateView, PreviewCommand, RenderedEmail, TemplateVariable,
    UpdateTemplateCommand,
};
use crate::tracking::{TrackingObjection, TrackingSettings};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::common::timestamp as instant;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = EmailAddressInput)]
pub struct EmailAddressDto {
    #[schema(example = "maria@customer.com")]
    pub(crate) address: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
}

impl From<EmailAddressDto> for EmailAddress {
    fn from(value: EmailAddressDto) -> Self {
        Self {
            address: value.address,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct EmailRecordInput {
    #[schema(example = "contact")]
    pub(crate) record_type: String,
    pub(crate) record_id: String,
}

impl From<EmailRecordInput> for RecordInput {
    fn from(value: EmailRecordInput) -> Self {
        Self {
            record_type: value.record_type,
            record_id: value.record_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SendEmailRequest {
    pub(crate) mailbox_id: Option<String>,
    pub(crate) to: Vec<EmailAddressDto>,
    #[serde(default)]
    pub(crate) cc: Vec<EmailAddressDto>,
    #[serde(default)]
    pub(crate) bcc: Vec<EmailAddressDto>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) body_text: Option<String>,
    pub(crate) template_id: Option<String>,
    pub(crate) record: Option<EmailRecordInput>,
    pub(crate) reply_to_message_id: Option<String>,
    #[serde(default)]
    pub(crate) attachment_file_ids: Vec<String>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub(crate) scheduled_at: Option<DateTime<Utc>>,
    pub(crate) tracking: Option<bool>,
}

impl SendEmailRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SendEmailCommand {
        SendEmailCommand {
            mailbox_id: self.mailbox_id,
            to: self.to.into_iter().map(Into::into).collect(),
            cc: self.cc.into_iter().map(Into::into).collect(),
            bcc: self.bcc.into_iter().map(Into::into).collect(),
            subject: self.subject,
            body_html: self.body_html,
            body_text: self.body_text,
            template_id: self.template_id,
            record: self.record.map(Into::into),
            reply_to_message_id: self.reply_to_message_id,
            attachment_file_ids: self.attachment_file_ids,
            scheduled_at: self.scheduled_at,
            tracking: self.tracking,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewEmailRequest {
    pub(crate) template_id: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) record: Option<EmailRecordInput>,
}

impl From<PreviewEmailRequest> for PreviewCommand {
    fn from(request: PreviewEmailRequest) -> Self {
        let (record_type, record_id) = match request.record {
            Some(record) => (Some(record.record_type), Some(record.record_id)),
            None => (None, None),
        };
        Self {
            template_id: request.template_id,
            subject: request.subject,
            body_html: request.body_html,
            record_type,
            record_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PreviewEmailResponse {
    pub(crate) subject: String,
    pub(crate) html: String,
    pub(crate) text: String,
    pub(crate) missing_variables: Vec<String>,
}

impl From<RenderedEmail> for PreviewEmailResponse {
    fn from(rendered: RenderedEmail) -> Self {
        Self {
            subject: rendered.subject,
            html: rendered.html,
            text: rendered.text,
            missing_variables: rendered.missing_variables,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailTemplateResponse {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) name: String,
    #[schema(example = "organization")]
    pub(crate) scope: String,
    pub(crate) owner_id: Option<String>,
    pub(crate) record_type: Option<String>,
    pub(crate) subject: String,
    pub(crate) body_html: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<EmailTemplateView> for EmailTemplateResponse {
    fn from(template: EmailTemplateView) -> Self {
        Self {
            id: template.id.to_string(),
            key: template.key,
            name: template.name,
            scope: template.scope.as_str().to_owned(),
            owner_id: template.owner_id.map(|owner| owner.as_str().to_owned()),
            record_type: template.record_type,
            subject: template.subject,
            body_html: template.body_html,
            created_at: instant(template.created_at),
            updated_at: instant(template.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct EmailTemplatesQuery {
    #[param(example = "all")]
    pub(crate) scope: Option<String>,
    #[param(example = "contact")]
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct EmailTemplateVariablesQuery {
    #[param(example = "deal")]
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateEmailTemplateRequest {
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    #[schema(example = "personal")]
    pub(crate) scope: Option<String>,
    pub(crate) record_type: Option<String>,
    #[serde(default)]
    pub(crate) subject: String,
    #[serde(default)]
    pub(crate) body_html: String,
}

impl CreateEmailTemplateRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateTemplateCommand {
        CreateTemplateCommand {
            key: self.key,
            name: self.name,
            scope: self.scope,
            record_type: self.record_type,
            subject: self.subject,
            body_html: self.body_html,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateEmailTemplateRequest {
    pub(crate) key: Option<String>,
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) record_type: Option<Option<String>>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
}

impl UpdateEmailTemplateRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> UpdateTemplateCommand {
        UpdateTemplateCommand {
            key: self.key,
            name: self.name,
            record_type: self.record_type,
            subject: self.subject,
            body_html: self.body_html,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EmailTemplateVariableResponse {
    #[schema(example = "contact.first_name")]
    pub(crate) key: String,
    pub(crate) label: String,
    #[schema(example = "contact")]
    pub(crate) record_type: String,
}

impl From<TemplateVariable> for EmailTemplateVariableResponse {
    fn from(variable: TemplateVariable) -> Self {
        Self {
            key: variable.key,
            label: variable.label,
            record_type: variable.record_type,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EmailTrackingSettingsDto {
    pub(crate) opens_enabled: bool,
    pub(crate) clicks_enabled: bool,
    pub(crate) default_on: bool,
}

impl From<TrackingSettings> for EmailTrackingSettingsDto {
    fn from(settings: TrackingSettings) -> Self {
        Self {
            opens_enabled: settings.opens_enabled,
            clicks_enabled: settings.clicks_enabled,
            default_on: settings.default_on,
        }
    }
}

impl From<EmailTrackingSettingsDto> for TrackingSettings {
    fn from(settings: EmailTrackingSettingsDto) -> Self {
        Self {
            opens_enabled: settings.opens_enabled,
            clicks_enabled: settings.clicks_enabled,
            default_on: settings.default_on,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTrackingObjectionRequest {
    #[schema(example = "maria@customer.com")]
    pub(crate) address: String,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TrackingObjectionResponse {
    pub(crate) id: String,
    #[schema(example = "m***@c***.com")]
    pub(crate) label: String,
    pub(crate) reason: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_at: String,
}

impl From<TrackingObjection> for TrackingObjectionResponse {
    fn from(objection: TrackingObjection) -> Self {
        Self {
            id: objection.id.to_string(),
            label: objection.label,
            reason: objection.reason,
            created_by: objection.created_by.map(|user| user.as_str().to_owned()),
            created_at: instant(objection.created_at),
        }
    }
}
