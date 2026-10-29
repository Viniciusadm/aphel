use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const TEMPLATE_NOT_FOUND: &str = "proposal_template_not_found";
pub(crate) const TEMPLATE_INVALID: &str = "proposal_template_invalid";
pub(crate) const TEMPLATE_KEY_TAKEN: &str = "proposal_template_key_taken";
pub(crate) const TEMPLATE_REQUIRED: &str = "proposal_template_required";
pub(crate) const TEMPLATE_INACTIVE: &str = "proposal_template_inactive";
pub(crate) const DEAL_NOT_FOUND: &str = "deal_not_found";
pub(crate) const QUOTE_NOT_FOUND: &str = "quote_not_found";
pub(crate) const INVALID_REQUEST: &str = "proposal_invalid_request";

pub(crate) fn template_not_found() -> AppError {
    AppError::not_found_code(TEMPLATE_NOT_FOUND, "proposal template not found")
}

pub(crate) fn deal_not_found() -> AppError {
    AppError::not_found_code(DEAL_NOT_FOUND, "deal not found")
}

pub(crate) fn quote_not_found() -> AppError {
    AppError::not_found_code(QUOTE_NOT_FOUND, "quote not found for this deal")
}

pub(crate) fn template_invalid(fields: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields_code(TEMPLATE_INVALID, "the proposal template is invalid", fields)
}
