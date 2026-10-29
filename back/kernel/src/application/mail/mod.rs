pub mod capture;
pub mod locale;
pub mod ports;
pub mod templates;

pub use capture::{MAIL_NOT_CONFIGURED, UndeliveredMailCapture, find_link, is_mail_not_configured};
pub use locale::MailLocale;
pub use ports::*;
