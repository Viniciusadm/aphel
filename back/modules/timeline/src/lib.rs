pub mod domain;
pub(crate) mod http;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "timeline";

pub struct TimelineModule;

impl Module for TimelineModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Timeline", APHEL_VERSION)
            .description(
                "Unified timeline of a record: activities, notes, e-mails and domain events",
            )
            .optional(activities::KEY)
            .optional(notes::KEY)
            .required()
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(http::TAG, "Unified timeline of a record")
            .routes(|_| http::router());
        Ok(())
    }
}
