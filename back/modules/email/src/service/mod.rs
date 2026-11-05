pub(crate) mod capture;
pub mod compose;
pub(crate) mod linking;
pub mod mailboxes;
pub(crate) mod reading;
pub(crate) mod records;
pub(crate) mod sync;
pub mod templates;
pub mod tracking;

use std::sync::Arc;

use async_trait::async_trait;

use crate::compose::{ComposedEmail, OwnerEmailRequest, SendEmailCommand};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

#[async_trait]
pub trait Sender: Send + Sync {
    async fn send(
        &self,
        ctx: &Ctx<'_>,
        command: SendEmailCommand,
    ) -> Result<ComposedEmail, AppError>;

    async fn send_as_owner(
        &self,
        ctx: &Ctx<'_>,
        request: OwnerEmailRequest,
    ) -> Result<ComposedEmail, AppError>;
}

pub struct MailboxSender;

#[async_trait]
impl Sender for MailboxSender {
    async fn send(
        &self,
        ctx: &Ctx<'_>,
        command: SendEmailCommand,
    ) -> Result<ComposedEmail, AppError> {
        compose::send(ctx, command).await
    }

    async fn send_as_owner(
        &self,
        ctx: &Ctx<'_>,
        request: OwnerEmailRequest,
    ) -> Result<ComposedEmail, AppError> {
        compose::send_as_owner(ctx, request).await
    }
}

pub fn sender() -> Arc<dyn Sender> {
    aphel_sdk::platform::modules::app()
        .services()
        .get::<dyn Sender>()
        .unwrap_or_else(|| Arc::new(MailboxSender))
}
