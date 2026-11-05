use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::incoming::{
    ActionKind, ActionOutcome, ActionTarget, CreateIncomingWebhookCommand, HOOKS_PATH_PREFIX,
    IncomingRequestLog, IncomingWebhook, IssuedIncomingWebhook, MappableField, ReceiveOutcome,
    RequestStatus, SaveIncomingWebhookCommand, WebhookAction,
};
use aphel_sdk::platform::api::dto::common::double_option;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = IncomingWebhookActionType)]
pub(crate) enum IncomingWebhookActionTypeSchema {
    CreateRecord,
    UpsertRecord,
}
impl From<ActionKind> for IncomingWebhookActionTypeSchema {
    fn from(kind: ActionKind) -> Self {
        match kind {
            ActionKind::CreateRecord => Self::CreateRecord,
            ActionKind::UpsertRecord => Self::UpsertRecord,
        }
    }
}
#[derive(Debug, Serialize, ToSchema)]
#[schema(as = IncomingWebhookAction)]
pub(crate) struct IncomingWebhookActionSchema {
    #[serde(rename = "type")]
    pub(crate) action_type: IncomingWebhookActionTypeSchema,
    pub(crate) record_type: String,
    pub(crate) match_field: Option<String>,
    #[schema(value_type = std::collections::HashMap<String, Value>)]
    pub(crate) fields: Map<String, Value>,
    pub(crate) assignment_rule_id: Option<String>,
}
impl From<&WebhookAction> for IncomingWebhookActionSchema {
    fn from(action: &WebhookAction) -> Self {
        Self {
            action_type: action.kind().into(),
            record_type: action.record_type.clone(),
            match_field: action.match_field.clone(),
            fields: action.fields.clone(),
            assignment_rule_id: action.assignment_rule_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = IncomingWebhookRequestStatus)]
pub(crate) enum IncomingWebhookRequestStatusSchema {
    Succeeded,
    Partial,
    Failed,
    Rejected,
}

impl From<RequestStatus> for IncomingWebhookRequestStatusSchema {
    fn from(status: RequestStatus) -> Self {
        match status {
            RequestStatus::Succeeded => Self::Succeeded,
            RequestStatus::Partial => Self::Partial,
            RequestStatus::Failed => Self::Failed,
            RequestStatus::Rejected => Self::Rejected,
        }
    }
}

impl From<IncomingWebhookRequestStatusSchema> for RequestStatus {
    fn from(status: IncomingWebhookRequestStatusSchema) -> Self {
        match status {
            IncomingWebhookRequestStatusSchema::Succeeded => Self::Succeeded,
            IncomingWebhookRequestStatusSchema::Partial => Self::Partial,
            IncomingWebhookRequestStatusSchema::Failed => Self::Failed,
            IncomingWebhookRequestStatusSchema::Rejected => Self::Rejected,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateIncomingWebhookRequest {
    #[schema(example = "Website form")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_active: Option<bool>,
    #[serde(default)]
    #[schema(value_type = Vec<IncomingWebhookActionSchema>)]
    pub(crate) actions: Vec<Value>,
    #[schema(example = 300, minimum = 30, maximum = 86400)]
    pub(crate) signature_tolerance_seconds: Option<u32>,
    #[serde(default)]
    pub(crate) generate_signing_secret: bool,
}

impl CreateIncomingWebhookRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateIncomingWebhookCommand {
        CreateIncomingWebhookCommand {
            save: SaveIncomingWebhookCommand {
                name: Some(self.name),
                description: Some(self.description),
                is_active: self.is_active,
                actions: Some(self.actions),
                signature_tolerance_seconds: self.signature_tolerance_seconds,
                ip,
                expected_updated_at: None,
            },
            generate_signing_secret: self.generate_signing_secret,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateIncomingWebhookRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_active: Option<bool>,
    #[schema(value_type = Option<Vec<IncomingWebhookActionSchema>>)]
    pub(crate) actions: Option<Vec<Value>>,
    #[schema(minimum = 30, maximum = 86400)]
    pub(crate) signature_tolerance_seconds: Option<u32>,
}

impl UpdateIncomingWebhookRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveIncomingWebhookCommand {
        SaveIncomingWebhookCommand {
            name: self.name,
            description: self.description,
            is_active: self.is_active,
            actions: self.actions,
            signature_tolerance_seconds: self.signature_tolerance_seconds,
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingWebhookResponse {
    #[schema(example = "0192a1b2-0000-7000-8000-000000000001")]
    pub(crate) id: String,
    #[schema(example = "Website form")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[schema(example = "whk_3f9a1c2b")]
    pub(crate) token_prefix: String,
    pub(crate) is_active: bool,
    pub(crate) secret_configured: bool,
    #[schema(example = 300)]
    pub(crate) signature_tolerance_seconds: u32,
    pub(crate) actions: Vec<IncomingWebhookActionSchema>,
    pub(crate) created_by: String,
    pub(crate) created_by_name: Option<String>,
    pub(crate) last_received_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<IncomingWebhook> for IncomingWebhookResponse {
    fn from(webhook: IncomingWebhook) -> Self {
        Self {
            id: webhook.id.to_string(),
            name: webhook.name,
            description: webhook.description,
            token_prefix: webhook.token_prefix,
            is_active: webhook.is_active,
            secret_configured: webhook.signature_required,
            signature_tolerance_seconds: webhook.signature_tolerance_seconds,
            actions: webhook.actions.iter().map(Into::into).collect(),
            created_by: webhook.created_by.as_str().to_owned(),
            created_by_name: webhook.created_by_name,
            last_received_at: webhook.last_received_at.map(timestamp),
            created_at: timestamp(webhook.created_at),
            updated_at: timestamp(webhook.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IssuedIncomingWebhookResponse {
    #[serde(flatten)]
    pub(crate) webhook: IncomingWebhookResponse,
    #[schema(example = "whk_3f9a1c2b9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c")]
    pub(crate) token: Option<String>,
    #[schema(
        example = "/api/v1/hooks/whk_3f9a1c2b9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c"
    )]
    pub(crate) path: Option<String>,
    #[schema(example = "whsec_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a")]
    pub(crate) signing_secret: Option<String>,
}

impl From<IssuedIncomingWebhook> for IssuedIncomingWebhookResponse {
    fn from(issued: IssuedIncomingWebhook) -> Self {
        let token = issued.token.map(|token| token.expose().to_owned());
        Self {
            path: token
                .as_ref()
                .map(|token| format!("{HOOKS_PATH_PREFIX}{token}")),
            token,
            signing_secret: issued
                .signing_secret
                .map(|secret| secret.expose().to_owned()),
            webhook: issued.webhook.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct IncomingWebhookRequestListParams {
    pub(crate) status: Option<IncomingWebhookRequestStatusSchema>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingActionErrorFieldResponse {
    pub(crate) field: String,
    pub(crate) message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingActionErrorResponse {
    #[schema(example = "validation_error")]
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) fields: Vec<IncomingActionErrorFieldResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingActionOutcomeResponse {
    pub(crate) index: usize,
    #[serde(rename = "type")]
    pub(crate) action_type: IncomingWebhookActionTypeSchema,
    #[schema(example = "succeeded")]
    pub(crate) status: String,
    #[schema(example = "created")]
    pub(crate) operation: Option<String>,
    #[schema(example = "people.person")]
    pub(crate) record_type: Option<String>,
    pub(crate) record_id: Option<String>,
    pub(crate) error: Option<IncomingActionErrorResponse>,
}

impl From<ActionOutcome> for IncomingActionOutcomeResponse {
    fn from(outcome: ActionOutcome) -> Self {
        let status = serde_json::to_value(&outcome.status)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        Self {
            index: outcome.index,
            action_type: outcome.action_type.into(),
            status,
            operation: outcome.operation,
            record_type: outcome.record_type,
            record_id: outcome.record_id,
            error: outcome.error.map(|error| IncomingActionErrorResponse {
                code: error.code,
                message: error.message,
                fields: error
                    .fields
                    .into_iter()
                    .map(|field| IncomingActionErrorFieldResponse {
                        field: field.field,
                        message: field.message,
                    })
                    .collect(),
            }),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingWebhookRequestResponse {
    pub(crate) id: String,
    pub(crate) webhook_id: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) received_at: String,
    #[schema(example = "203.0.113.10")]
    pub(crate) ip: Option<String>,
    #[schema(example = "application/json")]
    pub(crate) content_type: Option<String>,
    pub(crate) body_bytes: u64,
    pub(crate) body_truncated: bool,
    pub(crate) status: IncomingWebhookRequestStatusSchema,
    #[schema(example = 200)]
    pub(crate) http_status: u16,
    #[schema(example = "webhook_mapping_failed")]
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) results: Vec<IncomingActionOutcomeResponse>,
    pub(crate) duration_ms: u64,
}

impl From<IncomingRequestLog> for IncomingWebhookRequestResponse {
    fn from(log: IncomingRequestLog) -> Self {
        Self {
            id: log.id.to_string(),
            webhook_id: log.webhook_id.to_string(),
            received_at: timestamp(log.received_at),
            ip: log.ip,
            content_type: log.content_type,
            body_bytes: log.body_bytes,
            body_truncated: log.body_truncated,
            status: log.status.into(),
            http_status: log.http_status,
            error_code: log.error_code,
            error_message: log.error_message,
            results: log.results.into_iter().map(Into::into).collect(),
            duration_ms: log.duration_ms,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingWebhookRequestDetailResponse {
    #[serde(flatten)]
    pub(crate) request: IncomingWebhookRequestResponse,
    #[schema(value_type = std::collections::HashMap<String, String>, example = json!({ "content-type": "application/json", "user-agent": "curl/8.5.0" }))]
    pub(crate) headers: Map<String, Value>,
    pub(crate) body: Option<String>,
}

impl From<IncomingRequestLog> for IncomingWebhookRequestDetailResponse {
    fn from(mut log: IncomingRequestLog) -> Self {
        let headers = std::mem::take(&mut log.headers);
        let body = log.body.take();
        Self {
            request: log.into(),
            headers,
            body,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingMappableFieldResponse {
    #[schema(example = "email")]
    pub(crate) key: String,
    #[schema(example = "Email")]
    pub(crate) label: String,
    #[schema(example = "email")]
    pub(crate) field_type: String,
    pub(crate) is_custom: bool,
    pub(crate) is_required: bool,
    pub(crate) is_unique: bool,
}

impl From<MappableField> for IncomingMappableFieldResponse {
    fn from(field: MappableField) -> Self {
        Self {
            key: field.key,
            label: field.label,
            field_type: field.field_type,
            is_custom: field.is_custom,
            is_required: field.is_required,
            is_unique: field.is_unique,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IncomingActionTargetResponse {
    #[serde(rename = "type")]
    pub(crate) action_type: IncomingWebhookActionTypeSchema,
    #[schema(example = "people.person")]
    pub(crate) record_type: String,
    pub(crate) fields: Vec<IncomingMappableFieldResponse>,
}

impl From<ActionTarget> for IncomingActionTargetResponse {
    fn from(target: ActionTarget) -> Self {
        Self {
            action_type: target.action_type.into(),
            record_type: target.record_type,
            fields: target.fields.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct HookReceivedResponse {
    pub(crate) request_id: String,
    pub(crate) status: IncomingWebhookRequestStatusSchema,
}

impl From<&ReceiveOutcome> for HookReceivedResponse {
    fn from(outcome: &ReceiveOutcome) -> Self {
        Self {
            request_id: outcome.request_id.to_string(),
            status: outcome.status.into(),
        }
    }
}
