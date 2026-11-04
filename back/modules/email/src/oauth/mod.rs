pub mod connect;
pub(crate) mod execution;
pub mod http;
pub mod mime;
pub mod tokens;
pub use aphel_sdk::email::oauth::{
    ALREADY_CONNECTED, MAIL_ACCESS_DENIED, MAIL_API_FAILED, MAIL_AUTHORIZATION_REVOKED,
    MAIL_CONNECT_FAILED, MAIL_CURSOR_EXPIRED, MAIL_INVALID_MESSAGE, MAIL_NO_REFRESH_TOKEN,
    MAIL_NOT_AUTHORIZED, MAIL_NOT_FOUND, MAIL_TOKEN_REJECTED, MailOAuth, fetch_concurrently,
    provider_error,
};
