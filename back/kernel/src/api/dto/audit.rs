use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::application::audit::{AuditLogQuery, AuditLogView};
use crate::application::shared::{Actor, EventCausation, FieldChange};

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditLogQueryParams {
    #[param(example = "member")]
    pub record_type: Option<String>,
    #[param(example = "0190a8f2-7c1e-7b3a-9f00-000000000000")]
    pub record_id: Option<String>,
    pub actor: Option<String>,
    #[param(format = DateTime)]
    pub from: Option<String>,
    #[param(format = DateTime)]
    pub to: Option<String>,
}

impl From<AuditLogQueryParams> for AuditLogQuery {
    fn from(params: AuditLogQueryParams) -> Self {
        Self {
            record_type: params.record_type,
            record_id: params.record_id,
            actor: params.actor,
            from: params.from,
            to: params.to,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditActorTypeDto {
    User,
    System,
    ApiToken,
    Automation,
}

impl From<&Actor> for AuditActorTypeDto {
    fn from(actor: &Actor) -> Self {
        match actor {
            Actor::User(_) => Self::User,
            Actor::System => Self::System,
            Actor::ApiToken(_) => Self::ApiToken,
            Actor::Automation(_) => Self::Automation,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditActorResponse {
    #[serde(rename = "type")]
    pub actor_type: AuditActorTypeDto,
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditFieldChangeResponse {
    pub field: String,
    pub old: Value,
    pub new: Value,
}

impl From<FieldChange> for AuditFieldChangeResponse {
    fn from(change: FieldChange) -> Self {
        Self {
            field: change.field,
            old: change.old,
            new: change.new,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditCausationResponse {
    pub automation_id: String,
    pub run_id: String,
    pub depth: u32,
    pub chain: Vec<String>,
}

impl From<EventCausation> for AuditCausationResponse {
    fn from(causation: EventCausation) -> Self {
        Self {
            automation_id: causation.automation_id.to_string(),
            run_id: causation.run_id.to_string(),
            depth: causation.depth,
            chain: causation.chain.iter().map(ToString::to_string).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: String,
    pub actor: AuditActorResponse,
    #[schema(example = "member.role_changed")]
    pub action: String,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub changes: Vec<AuditFieldChangeResponse>,
    pub ip: Option<String>,
    pub caused_by: Option<AuditCausationResponse>,
    #[schema(format = DateTime)]
    pub occurred_at: String,
}

impl From<AuditLogView> for AuditLogResponse {
    fn from(view: AuditLogView) -> Self {
        let (record_type, record_id) = match view.record {
            Some(record) => (
                Some(record.record_type.as_str().to_owned()),
                Some(record.record_id.to_string()),
            ),
            None => (None, None),
        };
        Self {
            id: view.id.to_string(),
            actor: AuditActorResponse {
                actor_type: (&view.actor).into(),
                id: view.actor.id().map(str::to_owned),
                name: view.actor_name,
                email: view.actor_email,
            },
            action: view.action,
            record_type,
            record_id,
            changes: view.changes.into_iter().map(Into::into).collect(),
            ip: view.ip,
            caused_by: view.causation.map(Into::into),
            occurred_at: timestamp(view.occurred_at),
        }
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
