pub mod domain;
pub(crate) mod dto;
pub(crate) mod http;
pub mod service;
pub(crate) mod store;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "onboarding";

pub struct OnboardingModule;

impl Module for OnboardingModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Setup assistant", APHEL_VERSION)
            .description("First-run setup assistant: questions, template recommendation and state")
            .enabled_by_default()
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "First-run setup assistant of the organization: questions, template recommendation and state",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
