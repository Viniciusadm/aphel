#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainId(String);

impl DomainId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(
        value: impl Into<String>,
        field: &'static str,
    ) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(DomainValidationError::Empty(field));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_lowercase();
        if !value.contains('@') || value.starts_with('@') || value.ends_with('@') {
            return Err(DomainValidationError::Invalid("email"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtcDateTime(String);

impl UtcDateTime {
    pub fn new_iso8601(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("utc_datetime"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainValidationError {
    Empty(&'static str),
    Invalid(&'static str),
}

macro_rules! string_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainValidationError::Empty($field));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(UserId, "user_id");
string_id!(RoleId, "role_id");
string_id!(TeamId, "team_id");
string_id!(SessionId, "session_id");

impl UserId {
    pub const SYSTEM: &'static str = "system";

    pub fn system() -> Self {
        Self(Self::SYSTEM.to_owned())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(value)
    }
}
