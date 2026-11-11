pub(crate) mod cache;
pub(crate) mod data;
pub(crate) mod handler;
pub(crate) mod images;
pub(crate) mod invalidation;
pub(crate) mod runtime;

use std::sync::Arc;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, SiteRequestHandler};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "site";

pub struct SiteModule;

impl Module for SiteModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Public site", APHEL_VERSION)
            .description(
                "Public pages that plugins declare (blog posts, catalogs, landing pages), rendered on the server from their published records",
            )
            .owned_tables(&["site_cache_invalidations"])
            .migration(Migration::new(
                1,
                "create site cache invalidations",
                include_str!("../migrations/0001_create_site_cache_invalidations.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn SiteRequestHandler>(Arc::new(handler::SiteHandler));
        app.consumer(|env| invalidation::SiteInvalidationConsumer::shared(env.pool.clone()));
        Ok(())
    }
}
