use std::fmt;

pub const RATE_LIMITED: &str = "rate_limited";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Configuration(String),
    Conflict(String),
    Forbidden(String),
    Internal(String),
    NotFound(String),
    Persistence(String),
    RateLimited(String),
    Unauthorized(String),
    Validation(String),
    InvalidFields {
        message: String,
        fields: Vec<FieldViolation>,
    },
    CodedFields {
        code: &'static str,
        message: String,
        fields: Vec<FieldViolation>,
    },
    /// A client-facing error carrying a stable machine `code` the frontend maps
    /// to user copy. `message` is the technical/fallback text (logs + clients
    /// that don't know the code).
    Coded {
        kind: CodedKind,
        code: &'static str,
        message: String,
    },
    CodedDetails {
        kind: CodedKind,
        code: &'static str,
        message: String,
        details: serde_json::Value,
    },
}

/// The HTTP semantics of a [`AppError::Coded`] error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodedKind {
    Conflict,
    Forbidden,
    NotFound,
    Unauthorized,
    Validation,
    TooManyRequests,
    PayloadTooLarge,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldViolation {
    pub field: String,
    pub message: String,
    pub code: Option<&'static str>,
}

impl FieldViolation {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }
}

impl AppError {
    pub fn conflict_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Conflict,
            code,
            message: message.into(),
        }
    }

    pub fn forbidden_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Forbidden,
            code,
            message: message.into(),
        }
    }

    pub fn validation_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Validation,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Unauthorized,
            code,
            message: message.into(),
        }
    }

    pub fn rate_limited(
        message: impl Into<String>,
        retry_after: std::time::Duration,
        limit: Option<u32>,
    ) -> Self {
        let seconds = retry_after.as_secs() + u64::from(retry_after.subsec_nanos() > 0);
        Self::CodedDetails {
            kind: CodedKind::TooManyRequests,
            code: RATE_LIMITED,
            message: message.into(),
            details: serde_json::json!({ "retry_after": seconds.max(1), "limit": limit }),
        }
    }

    pub fn payload_too_large_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::PayloadTooLarge,
            code,
            message: message.into(),
        }
    }

    pub fn too_many_requests_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::TooManyRequests,
            code,
            message: message.into(),
        }
    }

    pub fn unavailable_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Unavailable,
            code,
            message: message.into(),
        }
    }

    pub fn not_found_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::NotFound,
            code,
            message: message.into(),
        }
    }

    pub fn invalid_fields_code(
        code: &'static str,
        message: impl Into<String>,
        fields: Vec<FieldViolation>,
    ) -> Self {
        Self::CodedFields {
            code,
            message: message.into(),
            fields,
        }
    }

    pub fn with_details(
        kind: CodedKind,
        code: &'static str,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::CodedDetails {
            kind,
            code,
            message: message.into(),
            details,
        }
    }

    pub fn invalid_fields(message: impl Into<String>, fields: Vec<FieldViolation>) -> Self {
        Self::InvalidFields {
            message: message.into(),
            fields,
        }
    }

    /// The user-facing message without the technical type prefix, suitable for
    /// the HTTP response body. `Display` keeps the prefixed form for logs.
    pub fn client_message(&self) -> &str {
        match self {
            Self::Configuration(message)
            | Self::Conflict(message)
            | Self::Forbidden(message)
            | Self::Internal(message)
            | Self::NotFound(message)
            | Self::Persistence(message)
            | Self::RateLimited(message)
            | Self::Unauthorized(message)
            | Self::Validation(message)
            | Self::InvalidFields { message, .. }
            | Self::CodedFields { message, .. }
            | Self::Coded { message, .. }
            | Self::CodedDetails { message, .. } => message,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) => write!(f, "configuration error: {message}"),
            Self::Conflict(message) => write!(f, "conflict: {message}"),
            Self::Forbidden(message) => write!(f, "forbidden: {message}"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
            Self::NotFound(message) => write!(f, "not found: {message}"),
            Self::Persistence(message) => write!(f, "persistence error: {message}"),
            Self::RateLimited(message) => write!(f, "rate limited: {message}"),
            Self::Unauthorized(message) => write!(f, "unauthorized: {message}"),
            Self::Validation(message) | Self::InvalidFields { message, .. } => {
                write!(f, "validation error: {message}")
            }
            Self::Coded { code, message, .. }
            | Self::CodedFields { code, message, .. }
            | Self::CodedDetails { code, message, .. } => {
                write!(f, "{code}: {message}")
            }
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        let deadlock = value
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == "40001");
        if deadlock {
            return Self::conflict_code(
                "concurrent_modification",
                "the record was changed by another request; retry",
            );
        }
        Self::Persistence(value.to_string())
    }
}

impl From<crate::domain::DomainValidationError> for AppError {
    fn from(value: crate::domain::DomainValidationError) -> Self {
        Self::Validation(format!("{value:?}"))
    }
}
