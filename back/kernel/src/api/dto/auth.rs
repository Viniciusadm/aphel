use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::organization::OrganizationResponse;
use crate::application::auth::two_factor::TwoFactorStatus;
use crate::application::auth::{AccountOverview, InvitationPreview, PendingTwoFactor};
use crate::domain::access::{RecordScope, permission_catalog};
use crate::domain::users::User;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "ana@example.com")]
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SsoDiscoverRequest {
    #[schema(example = "ana@example.com")]
    pub email: String,
}

#[derive(Serialize, ToSchema)]
pub struct SsoOptionResponse {
    #[schema(example = "google")]
    pub provider: String,
    pub label: String,
    pub url: String,
    pub required: bool,
}

#[derive(Serialize, ToSchema)]
pub struct SsoDiscoverResponse {
    pub data: Vec<SsoOptionResponse>,
}

impl From<crate::api::sso::LoginOption> for SsoOptionResponse {
    fn from(option: crate::api::sso::LoginOption) -> Self {
        Self {
            provider: option.provider,
            label: option.label,
            url: option.url,
            required: option.required,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    #[schema(example = "ana@example.com")]
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[schema(min_length = 8, max_length = 128)]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AcceptInvitationRequest {
    pub token: String,
    #[schema(example = "Bia Lima")]
    pub display_name: Option<String>,
    #[schema(min_length = 8, max_length = 128)]
    pub password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordScopeDto {
    Own,
    Team,
    Organization,
}

impl From<RecordScope> for RecordScopeDto {
    fn from(value: RecordScope) -> Self {
        match value {
            RecordScope::Own => Self::Own,
            RecordScope::Team => Self::Team,
            RecordScope::Organization => Self::Organization,
        }
    }
}

impl From<RecordScopeDto> for RecordScope {
    fn from(value: RecordScopeDto) -> Self {
        match value {
            RecordScopeDto::Own => Self::Own,
            RecordScopeDto::Team => Self::Team,
            RecordScopeDto::Organization => Self::Organization,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub email_verified: bool,
    pub locale: Option<String>,
    pub timezone: Option<String>,
}

impl From<&User> for MeUser {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.as_str().to_owned(),
            email: user.email.as_str().to_owned(),
            display_name: user.display_name.as_str().to_owned(),
            email_verified: user.is_email_verified(),
            locale: user.locale.clone(),
            timezone: user.timezone.clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionGrantDto {
    #[schema(example = "people.read")]
    pub permission: String,
    pub scope: RecordScopeDto,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeTwoFactor {
    pub enabled: bool,
    pub required: bool,
}

impl From<TwoFactorStatus> for MeTwoFactor {
    fn from(status: TwoFactorStatus) -> Self {
        Self {
            enabled: status.enabled,
            required: status.required,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub user: MeUser,
    pub organization: Option<OrganizationResponse>,
    pub role_key: String,
    pub permissions: Vec<PermissionGrantDto>,
    pub two_factor: MeTwoFactor,
    pub teams: Vec<crate::api::dto::members::TeamReferenceResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TwoFactorRequiredResponse {
    #[schema(example = true)]
    pub two_factor_required: bool,
    #[schema(example = "2026-10-20T13:55:00Z")]
    pub expires_at: String,
}

impl From<PendingTwoFactor> for TwoFactorRequiredResponse {
    fn from(pending: PendingTwoFactor) -> Self {
        Self {
            two_factor_required: true,
            expires_at: pending
                .expires_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum LoginResponse {
    Session(Box<MeResponse>),
    TwoFactorRequired(TwoFactorRequiredResponse),
}

impl From<AccountOverview> for MeResponse {
    fn from(account: AccountOverview) -> Self {
        let scope = &account.scope;
        let permissions = if scope.permissions().grants_everything() {
            permission_catalog()
                .into_iter()
                .map(|definition| definition.permission)
                .chain(
                    scope
                        .permissions()
                        .catalog_extension()
                        .iter()
                        .map(|definition| definition.permission.clone()),
                )
                .map(|permission| PermissionGrantDto {
                    permission: permission.as_str().to_owned(),
                    scope: RecordScopeDto::Organization,
                })
                .collect()
        } else {
            scope
                .permissions()
                .grants()
                .map(|grant| PermissionGrantDto {
                    permission: grant.permission.as_str().to_owned(),
                    scope: grant.scope.into(),
                })
                .collect()
        };

        Self {
            user: MeUser::from(&account.user),
            organization: account.organization.map(OrganizationResponse::from),
            role_key: account.scope.role_key().as_str().to_owned(),
            permissions,
            two_factor: account.two_factor.into(),
            teams: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationResponse {
    pub organization_name: String,
    pub email: String,
    pub requires_account: bool,
}

impl From<InvitationPreview> for InvitationResponse {
    fn from(preview: InvitationPreview) -> Self {
        Self {
            organization_name: preview.organization_name,
            email: preview.email,
            requires_account: preview.requires_account,
        }
    }
}
