use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::auth::two_factor::{
    TwoFactorOverview, TwoFactorPolicy, TwoFactorPolicyView, TwoFactorSetup,
};
use crate::domain::Secret;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Deserialize, ToSchema)]
pub struct TwoFactorChallengeRequest {
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ConfirmTwoFactorRequest {
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct DisableTwoFactorRequest {
    pub password: String,
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RegenerateRecoveryCodesRequest {
    pub password: Option<String>,
    #[schema(example = "123456")]
    pub code: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
    pub pending_setup: bool,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub enabled_at: Option<String>,
    pub recovery_codes_remaining: u32,
}

impl From<TwoFactorOverview> for TwoFactorStatusResponse {
    fn from(overview: TwoFactorOverview) -> Self {
        Self {
            enabled: overview.enabled,
            pending_setup: overview.pending_setup,
            enabled_at: overview.enabled_at.map(timestamp),
            recovery_codes_remaining: overview.recovery_codes_remaining,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TwoFactorSetupResponse {
    #[schema(example = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP")]
    pub secret: String,
    #[schema(
        example = "otpauth://totp/Aphel:ana%40example.com?secret=JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP&issuer=Aphel"
    )]
    pub otpauth_uri: String,
    pub qr_code_svg: String,
}

impl From<TwoFactorSetup> for TwoFactorSetupResponse {
    fn from(setup: TwoFactorSetup) -> Self {
        Self {
            secret: setup.secret.expose().to_owned(),
            otpauth_uri: setup.otpauth_uri,
            qr_code_svg: setup.qr_code_svg,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecoveryCodesResponse {
    #[schema(example = json!(["k7m2p-x9q4r", "a3b8c-d2e5f"]))]
    pub recovery_codes: Vec<String>,
}

impl From<Vec<Secret>> for RecoveryCodesResponse {
    fn from(codes: Vec<Secret>) -> Self {
        Self {
            recovery_codes: codes
                .into_iter()
                .map(|code| code.expose().to_owned())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TwoFactorPolicyDto {
    None,
    Admins,
    Everyone,
}

impl From<TwoFactorPolicy> for TwoFactorPolicyDto {
    fn from(policy: TwoFactorPolicy) -> Self {
        match policy {
            TwoFactorPolicy::None => Self::None,
            TwoFactorPolicy::Admins => Self::Admins,
            TwoFactorPolicy::Everyone => Self::Everyone,
        }
    }
}

impl From<TwoFactorPolicyDto> for TwoFactorPolicy {
    fn from(policy: TwoFactorPolicyDto) -> Self {
        match policy {
            TwoFactorPolicyDto::None => Self::None,
            TwoFactorPolicyDto::Admins => Self::Admins,
            TwoFactorPolicyDto::Everyone => Self::Everyone,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTwoFactorPolicyRequest {
    pub policy: TwoFactorPolicyDto,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TwoFactorPolicyResponse {
    pub policy: TwoFactorPolicyDto,
    pub members_without_two_factor: u64,
}

impl From<TwoFactorPolicyView> for TwoFactorPolicyResponse {
    fn from(view: TwoFactorPolicyView) -> Self {
        Self {
            policy: view.policy.into(),
            members_without_two_factor: view.members_without_two_factor,
        }
    }
}
