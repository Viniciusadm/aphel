use std::env;
use std::sync::OnceLock;

use async_trait::async_trait;
use tracing::info;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::mail::{EmailAttachment, EmailMessage, Mailer};

pub(crate) const DEFAULT_DEMO_VISITOR_EMAIL: &str = "demo@aphel.demo";
pub(crate) const DEFAULT_DEMO_VISITOR_PASSWORD: &str = "aphel-demo";
pub(crate) const DEFAULT_DEMO_RESET_TIME: &str = "03:00";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoSettings {
    pub enabled: bool,
    pub visitor_email: String,
    pub visitor_password: String,
    pub reset_time_utc: String,
}

fn variable(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub(crate) fn demo_settings_from_env() -> DemoSettings {
    DemoSettings {
        enabled: env::var("DEMO_MODE")
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false),
        visitor_email: variable("DEMO_USER_EMAIL", DEFAULT_DEMO_VISITOR_EMAIL),
        visitor_password: variable("DEMO_USER_PASSWORD", DEFAULT_DEMO_VISITOR_PASSWORD),
        reset_time_utc: variable("DEMO_RESET_TIME", DEFAULT_DEMO_RESET_TIME),
    }
}

pub fn demo_settings() -> &'static DemoSettings {
    static SETTINGS: OnceLock<DemoSettings> = OnceLock::new();
    SETTINGS.get_or_init(demo_settings_from_env)
}

pub struct DemoLogMailer;

#[async_trait]
impl Mailer for DemoLogMailer {
    async fn send(&self, message: EmailMessage) -> Result<(), AppError> {
        info!(
            subject = message.subject.as_str(),
            "demo mode: email discarded instead of sent"
        );
        Ok(())
    }

    async fn send_with_attachments(
        &self,
        message: EmailMessage,
        _attachments: Vec<EmailAttachment>,
    ) -> Result<(), AppError> {
        self.send(message).await
    }
}
