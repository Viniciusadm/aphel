use std::sync::Arc;

use async_trait::async_trait;

use crate::application::mail::{EmailAttachment, EmailMessage, Mailer, SharedMailer};
use crate::errors::AppError;

#[async_trait]
pub trait OrganizationMailerResolver: Send + Sync {
    async fn mailer(&self) -> SharedMailer;
}

pub type SharedOrganizationMailerResolver = Arc<dyn OrganizationMailerResolver>;

pub struct GlobalMailerResolver {
    mailer: SharedMailer,
}

impl GlobalMailerResolver {
    pub fn new(mailer: SharedMailer) -> Self {
        Self { mailer }
    }
}

#[async_trait]
impl OrganizationMailerResolver for GlobalMailerResolver {
    async fn mailer(&self) -> SharedMailer {
        self.mailer.clone()
    }
}

#[derive(Clone)]
pub struct OrganizationMailer {
    resolver: SharedOrganizationMailerResolver,
}

impl OrganizationMailer {
    pub fn new(resolver: SharedOrganizationMailerResolver) -> Self {
        Self { resolver }
    }

    pub fn shared(resolver: SharedOrganizationMailerResolver) -> SharedMailer {
        Arc::new(Self::new(resolver))
    }
}

#[async_trait]
impl Mailer for OrganizationMailer {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        self.resolver.mailer().await.send(message).await
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        self.resolver
            .mailer()
            .await
            .send_with_attachments(message, attachments)
            .await
    }
}
