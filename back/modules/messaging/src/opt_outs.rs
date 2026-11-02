use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::domain::Email;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const OPT_OUT_EXISTS: &str = "communication_opt_out_exists";
pub const OPT_OUT_NOT_FOUND: &str = "communication_opt_out_not_found";
pub const RECIPIENT_OPTED_OUT: &str = "communication_recipient_opted_out";
pub const OPTED_OUT_EVENT: &str = "communication.opted_out";
pub const OPT_OUT_REMOVED_EVENT: &str = "communication.opt_out_removed";
pub const MAX_REASON_LENGTH: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptOutScope {
    All,
    Marketing,
}

impl OptOutScope {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "marketing" => Some(Self::Marketing),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Marketing => "marketing",
        }
    }

    pub fn blocks(&self, purpose: Purpose) -> bool {
        match self {
            Self::All => true,
            Self::Marketing => purpose == Purpose::Marketing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Transactional,
    Marketing,
}

impl Purpose {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "transactional" => Some(Self::Transactional),
            "marketing" => Some(Self::Marketing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptOut {
    pub id: Uuid,
    pub address: String,
    pub scope: OptOutScope,
    pub reason: Option<String>,
    pub source: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub fn normalize_address(value: &str) -> Result<String, AppError> {
    let address = value.trim().to_lowercase();
    Email::new(address.clone()).map_err(|_| {
        AppError::invalid_fields(
            "invalid opt-out",
            vec![FieldViolation::new(
                "address",
                "must be a valid email address",
            )],
        )
    })?;
    Ok(address)
}

pub fn opted_out_error(address: &str) -> AppError {
    AppError::forbidden_code(
        RECIPIENT_OPTED_OUT,
        format!("{address} asked not to receive this kind of message"),
    )
}
