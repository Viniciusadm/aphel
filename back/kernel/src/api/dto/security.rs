use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::application::auth::ClientMetadata;
use crate::application::security::{
    ChangePasswordCommand, DeviceKind, SecurityEventKind, SecurityEventQuery, SecurityEventUser,
    SecurityEventView, SessionView, describe_device,
};
use crate::domain::Secret;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKindDto {
    Desktop,
    Mobile,
    Tablet,
    Bot,
    Unknown,
}

impl From<DeviceKind> for DeviceKindDto {
    fn from(kind: DeviceKind) -> Self {
        match kind {
            DeviceKind::Desktop => Self::Desktop,
            DeviceKind::Mobile => Self::Mobile,
            DeviceKind::Tablet => Self::Tablet,
            DeviceKind::Bot => Self::Bot,
            DeviceKind::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceResponse {
    #[schema(example = "Chrome")]
    pub browser: Option<String>,
    #[schema(example = "Windows")]
    pub os: Option<String>,
    pub kind: DeviceKindDto,
}

impl DeviceResponse {
    fn from_user_agent(user_agent: Option<&str>) -> Self {
        let device = describe_device(user_agent);
        Self {
            browser: device.browser.map(str::to_owned),
            os: device.os.map(str::to_owned),
            kind: device.kind.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecuritySessionResponse {
    pub id: String,
    pub current: bool,
    pub device: DeviceResponse,
    pub user_agent: Option<String>,
    #[schema(example = "203.0.113.10")]
    pub ip: Option<String>,
    #[schema(format = DateTime)]
    pub created_at: String,
    #[schema(format = DateTime)]
    pub last_seen_at: String,
    #[schema(format = DateTime)]
    pub expires_at: String,
}

impl From<SessionView> for SecuritySessionResponse {
    fn from(view: SessionView) -> Self {
        let session = view.session;
        Self {
            id: session.id.into_inner(),
            current: view.current,
            device: DeviceResponse::from_user_agent(session.user_agent.as_deref()),
            user_agent: session.user_agent,
            ip: session.ip,
            created_at: timestamp(session.created_at),
            last_seen_at: timestamp(session.last_seen_at),
            expires_at: timestamp(session.expires_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokedSessionsResponse {
    #[schema(example = 2)]
    pub revoked: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    #[schema(example = "a-long-new-password")]
    pub new_password: String,
}

impl ChangePasswordRequest {
    pub fn into_command(self, client: ClientMetadata) -> ChangePasswordCommand {
        ChangePasswordCommand {
            current_password: Secret::new(self.current_password),
            new_password: Secret::new(self.new_password),
            client,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordChangedResponse {
    #[schema(example = 1)]
    pub revoked_sessions: u64,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SecurityEventQueryParams {
    #[param(example = "login_failed,login_locked")]
    pub kind: Option<String>,
    #[param(format = DateTime, example = "2026-10-01T00:00:00Z")]
    pub from: Option<String>,
    #[param(format = DateTime, example = "2026-10-31T23:59:59Z")]
    pub to: Option<String>,
}

impl From<SecurityEventQueryParams> for SecurityEventQuery {
    fn from(params: SecurityEventQueryParams) -> Self {
        Self {
            kind: params.kind,
            from: params.from,
            to: params.to,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventKindDto {
    LoginSucceeded,
    LoginFailed,
    LoginLocked,
    TwoFactorEnabled,
    TwoFactorDisabled,
    TwoFactorChallengeFailed,
    RecoveryCodeUsed,
    RecoveryCodesRegenerated,
    PasswordChanged,
    SessionRevoked,
    ApiTokenCreated,
    ApiTokenRevoked,
    PermissionChanged,
}

impl From<SecurityEventKind> for SecurityEventKindDto {
    fn from(kind: SecurityEventKind) -> Self {
        match kind {
            SecurityEventKind::LoginSucceeded => Self::LoginSucceeded,
            SecurityEventKind::LoginFailed => Self::LoginFailed,
            SecurityEventKind::LoginLocked => Self::LoginLocked,
            SecurityEventKind::TwoFactorEnabled => Self::TwoFactorEnabled,
            SecurityEventKind::TwoFactorDisabled => Self::TwoFactorDisabled,
            SecurityEventKind::TwoFactorChallengeFailed => Self::TwoFactorChallengeFailed,
            SecurityEventKind::RecoveryCodeUsed => Self::RecoveryCodeUsed,
            SecurityEventKind::RecoveryCodesRegenerated => Self::RecoveryCodesRegenerated,
            SecurityEventKind::PasswordChanged => Self::PasswordChanged,
            SecurityEventKind::SessionRevoked => Self::SessionRevoked,
            SecurityEventKind::ApiTokenCreated => Self::ApiTokenCreated,
            SecurityEventKind::ApiTokenRevoked => Self::ApiTokenRevoked,
            SecurityEventKind::PermissionChanged => Self::PermissionChanged,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
}

impl From<SecurityEventUser> for SecurityEventUserResponse {
    fn from(user: SecurityEventUser) -> Self {
        Self {
            id: user.id.into_inner(),
            name: user.name,
            email: user.email,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityEventResponse {
    pub id: String,
    pub kind: SecurityEventKindDto,
    pub user: Option<SecurityEventUserResponse>,
    #[schema(example = "203.0.113.10")]
    pub ip: Option<String>,
    pub device: DeviceResponse,
    pub user_agent: Option<String>,
    #[schema(value_type = Object, example = json!({ "reason": "wrong_password" }))]
    pub metadata: Value,
    #[schema(format = DateTime)]
    pub occurred_at: String,
}

impl From<SecurityEventView> for SecurityEventResponse {
    fn from(event: SecurityEventView) -> Self {
        Self {
            id: event.id,
            kind: event.kind.into(),
            user: event.user.map(Into::into),
            device: DeviceResponse::from_user_agent(event.user_agent.as_deref()),
            ip: event.ip,
            user_agent: event.user_agent,
            metadata: event.metadata,
            occurred_at: timestamp(event.occurred_at),
        }
    }
}
