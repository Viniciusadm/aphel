pub mod capture;
pub mod locale;
pub mod ports;
pub mod templates;

pub use capture::{MAIL_NOT_CONFIGURED, UndeliveredMailCapture, find_link, is_mail_not_configured};
pub use locale::MailLocale;
pub use ports::*;

pub trait MailDeliveryPolicy: Send + Sync {
    fn replacement(&self) -> Option<SharedMailer>;
}

pub fn delivery_override() -> Option<SharedMailer> {
    crate::modules::app()
        .contributions::<dyn MailDeliveryPolicy>()
        .into_iter()
        .find_map(|policy| policy.replacement())
}

pub mod resolver;
pub use resolver::*;

#[async_trait::async_trait]
pub trait MailRecipientPolicy: Send + Sync {
    async fn check(
        &self,
        pool: &sqlx::MySqlPool,
        address: &str,
        purpose: &str,
    ) -> Result<(), crate::errors::AppError>;
}
pub async fn check_recipient(
    pool: &sqlx::MySqlPool,
    address: &str,
    purpose: &str,
) -> Result<(), crate::errors::AppError> {
    for policy in crate::modules::app().contributions::<dyn MailRecipientPolicy>() {
        policy.check(pool, address, purpose).await?;
    }
    Ok(())
}
