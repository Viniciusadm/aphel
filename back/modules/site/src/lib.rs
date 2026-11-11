pub(crate) mod assets;
pub(crate) mod cache;
pub(crate) mod data;
pub(crate) mod dynamic;
pub(crate) mod handler;
pub(crate) mod http;
pub(crate) mod images;
pub(crate) mod invalidation;
pub(crate) mod redirects;
pub(crate) mod runtime;
pub(crate) mod settings;
pub(crate) mod sitemap;

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
            .owned_tables(&["site_cache_invalidations", "site_redirects"])
            .migration(Migration::new(
                1,
                "create site cache invalidations",
                include_str!("../migrations/0001_create_site_cache_invalidations.sql"),
            ))
            .migration(Migration::new(
                2,
                "create site redirects",
                include_str!("../migrations/0002_create_site_redirects.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn SiteRequestHandler>(Arc::new(handler::SiteHandler));
        app.contribute::<dyn aphel_sdk::platform::application::organization_profile::OrganizationConfigExtension>(
            Arc::new(settings::SiteConfiguration),
        );
        app.consumer(|env| invalidation::SiteInvalidationConsumer::shared(env.pool.clone()));
        app.openapi_tag(
            http::TAG,
            "Public site administration: themes, menu items, redirects and page cache",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
