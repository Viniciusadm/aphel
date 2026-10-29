use chrono::{DateTime, Utc};

use crate::application::auth::ClientMetadata;
use crate::application::auth::two_factor::TwoFactorPolicy;
use crate::domain::Secret;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoFactorOverview {
    pub enabled: bool,
    pub pending_setup: bool,
    pub enabled_at: Option<DateTime<Utc>>,
    pub recovery_codes_remaining: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoFactorSetup {
    pub secret: Secret,
    pub otpauth_uri: String,
    pub qr_code_svg: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmTwoFactorCommand {
    pub code: Secret,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableTwoFactorCommand {
    pub password: Secret,
    pub code: Secret,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenerateRecoveryCodesCommand {
    pub password: Option<Secret>,
    pub code: Option<Secret>,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTwoFactorPolicyCommand {
    pub policy: String,
    pub client: ClientMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoFactorPolicyView {
    pub policy: TwoFactorPolicy,
    pub members_without_two_factor: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedFactor {
    Totp,
    RecoveryCode,
}

impl VerifiedFactor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::RecoveryCode => "recovery_code",
        }
    }
}
