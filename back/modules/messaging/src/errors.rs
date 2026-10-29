use aphel_kernel::errors::AppError;

pub(crate) const CHANNEL_NOT_FOUND: &str = "messaging_channel_not_found";
pub(crate) const CHANNEL_INVALID: &str = "messaging_channel_invalid";
pub(crate) const CHANNEL_IN_USE: &str = "messaging_channel_in_use";
pub(crate) const CHANNEL_INACTIVE: &str = "messaging_channel_inactive";
pub(crate) const CHANNEL_LIMIT_REACHED: &str = "messaging_channel_limit_reached";
pub(crate) const CONVERSATION_NOT_FOUND: &str = "messaging_conversation_not_found";
pub(crate) const INVALID_QUERY: &str = "messaging_invalid_query";
pub(crate) const INVALID_REPLY: &str = "messaging_invalid_reply";
pub(crate) const INVALID_ASSIGNEE: &str = "messaging_invalid_assignee";
pub(crate) const RECORD_NOT_ACCESSIBLE: &str = "messaging_record_not_accessible";
pub(crate) const WHATSAPP_WINDOW_CLOSED: &str = "whatsapp_window_closed";
pub(crate) const WHATSAPP_INVALID_SIGNATURE: &str = "whatsapp_invalid_signature";
pub(crate) const WHATSAPP_INVALID_VERIFY_TOKEN: &str = "whatsapp_invalid_verify_token";
pub(crate) const WEBHOOK_PAYLOAD_TOO_LARGE: &str = "messaging_payload_too_large";
pub(crate) const WEBHOOK_INVALID_PAYLOAD: &str = "messaging_invalid_payload";
pub(crate) const ACTING_USER_UNAVAILABLE: &str = "messaging_acting_user_unavailable";

pub(crate) fn channel_not_found() -> AppError {
    AppError::not_found_code(CHANNEL_NOT_FOUND, "messaging channel not found")
}

pub(crate) fn conversation_not_found() -> AppError {
    AppError::not_found_code(CONVERSATION_NOT_FOUND, "conversation not found")
}
