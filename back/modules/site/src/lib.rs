pub(crate) mod data;
pub(crate) mod handler;
pub(crate) mod runtime;

use std::sync::Arc;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::SiteRequestHandler;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "site";

pub struct SiteModule;

impl Module for SiteModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Public site", APHEL_VERSION).description(
            "Public pages that plugins declare (blog posts, catalogs, landing pages), rendered on the server from their published records",
        )
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn SiteRequestHandler>(Arc::new(handler::SiteHandler));
        Ok(())
    }
}
