use crate::domain::{Email, NonEmptyString, UserId, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Invited,
    Active,
    Deactivated,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Active => "active",
            Self::Deactivated => "deactivated",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "invited" => Some(Self::Invited),
            "active" => Some(Self::Active),
            "deactivated" => Some(Self::Deactivated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub password_hash: Option<NonEmptyString>,
    pub display_name: NonEmptyString,
    pub email_verified_at: Option<UtcDateTime>,
    pub status: UserStatus,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub avatar_file_id: Option<String>,
    pub last_login_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

impl User {
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}
