use crate::settings::{DemoLogMailer, demo_settings};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::HttpRequestPolicy;
use aphel_sdk::platform::application::mail::{MailDeliveryPolicy, SharedMailer};
use axum::http::Method;

pub const DEMO_RESTRICTED: &str = "demo_restricted";

const RESTRICTED_PREFIXES: [&str; 7] = [
    "/api/v1/auth/forgot-password",
    "/api/v1/auth/reset-password",
    "/api/v1/auth/logout-all",
    "/api/v1/api-tokens",
    "/api/v1/security/password",
    "/api/v1/security/sessions/revoke-others",
    "/api/v1/two-factor",
];

pub struct DemoPolicy;
impl HttpRequestPolicy for DemoPolicy {
    fn check(&self, method: &Method, path: &str) -> Result<(), AppError> {
        if demo_settings().enabled
            && !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
            && RESTRICTED_PREFIXES
                .iter()
                .any(|prefix| path.starts_with(prefix))
        {
            return Err(AppError::forbidden_code(
                DEMO_RESTRICTED,
                "this action is disabled in the demonstration environment",
            ));
        }
        Ok(())
    }
}
impl MailDeliveryPolicy for DemoPolicy {
    fn replacement(&self) -> Option<SharedMailer> {
        demo_settings()
            .enabled
            .then(|| std::sync::Arc::new(DemoLogMailer) as SharedMailer)
    }
}
