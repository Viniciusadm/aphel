use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::files::UserReference;
use crate::application::shared::RecordRef;
use crate::domain::UserId;

pub const RECORD_SHARED: &str = "record.shared";
pub const RECORD_SHARE_UPDATED: &str = "record.share_updated";
pub const RECORD_UNSHARED: &str = "record.unshared";

pub const RECORD_NOT_FOUND: &str = "record_not_found";
pub const RECORD_SHARE_NOT_FOUND: &str = "record_share_not_found";
pub const RECORD_SHARE_EXISTS: &str = "record_share_exists";
pub const RECORD_SHARE_NOT_ALLOWED: &str = "record_share_not_allowed";
pub const RECORD_SHARES_LIMIT_REACHED: &str = "record_shares_limit_reached";

pub const MAX_RECORD_SHARES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShareGranteeType {
    User,
    Team,
}

impl ShareGranteeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Team => "team",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "team" => Some(Self::Team),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShareAccess {
    Read,
    Edit,
}

impl ShareAccess {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "edit" => Some(Self::Edit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareGrantee {
    pub kind: ShareGranteeType,
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordShare {
    pub id: Uuid,
    pub record: RecordRef,
    pub grantee: ShareGrantee,
    pub access: ShareAccess,
    pub shared_by: Option<UserReference>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecordHeader {
    pub owner_id: Option<UserId>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordShares {
    pub record: RecordRef,
    pub owner_id: Option<UserId>,
    pub can_manage: bool,
    pub shares: Vec<RecordShare>,
}
