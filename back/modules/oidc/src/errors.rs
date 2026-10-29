use aphel_kernel::errors::{AppError, CodedKind};

pub fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => code,
        AppError::Conflict(_) => "conflict",
        AppError::Forbidden(_) => "forbidden",
        AppError::NotFound(_) => "not_found",
        AppError::RateLimited(_) => "rate_limited",
        AppError::Unauthorized(_) => "unauthorized",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        AppError::Configuration(_) | AppError::Internal(_) | AppError::Persistence(_) => {
            "internal_error"
        }
    }
}

fn coded_kind(error: &AppError) -> Option<CodedKind> {
    match error {
        AppError::Coded { kind, .. } | AppError::CodedDetails { kind, .. } => Some(*kind),
        _ => None,
    }
}

pub(crate) fn is_unavailable(error: &AppError) -> bool {
    matches!(error, AppError::Configuration(_))
}

pub(crate) fn is_internal(error: &AppError) -> bool {
    matches!(error, AppError::Internal(_) | AppError::Persistence(_))
}

pub(crate) fn is_not_found(error: &AppError) -> bool {
    matches!(error, AppError::NotFound(_)) || coded_kind(error) == Some(CodedKind::NotFound)
}

pub(crate) fn is_conflict(error: &AppError) -> bool {
    matches!(error, AppError::Conflict(_)) || coded_kind(error) == Some(CodedKind::Conflict)
}

pub(crate) fn is_too_many_requests(error: &AppError) -> bool {
    matches!(error, AppError::RateLimited(_))
        || coded_kind(error) == Some(CodedKind::TooManyRequests)
}

pub(crate) fn is_authentication(error: &AppError) -> bool {
    matches!(error, AppError::Unauthorized(_) | AppError::Forbidden(_))
        || matches!(
            coded_kind(error),
            Some(CodedKind::Unauthorized | CodedKind::Forbidden)
        )
}

pub(crate) fn unavailable(message: impl Into<String>) -> AppError {
    AppError::Configuration(message.into())
}

pub fn internal(message: impl Into<String>) -> AppError {
    AppError::Internal(message.into())
}
