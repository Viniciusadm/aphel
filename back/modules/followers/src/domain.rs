use chrono::{DateTime, Utc};

use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::shared::RecordRef;

pub(crate) const MAX_FOLLOWED_RECORDS: u32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FollowOrigin {
    Manual,
    Owner,
}

impl FollowOrigin {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Owner => "owner",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordFollower {
    pub(crate) user: UserReference,
    pub(crate) origin: FollowOrigin,
    pub(crate) followed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordFollowers {
    pub(crate) record: RecordRef,
    pub(crate) following: bool,
    pub(crate) followers: Vec<RecordFollower>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Follow {
    pub(crate) record: RecordRef,
    pub(crate) origin: FollowOrigin,
    pub(crate) followed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FollowedRecord {
    pub(crate) follow: Follow,
    pub(crate) title: Option<String>,
}
