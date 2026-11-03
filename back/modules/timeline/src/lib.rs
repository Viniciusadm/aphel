pub mod domain;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
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
            .description("Owner-contributed record timeline")
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::timeline::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::timeline::TimelineSource>(std::sync::Arc::new(
            store::Events,
        ));
        app.openapi_tag(http::TAG, "Unified timeline of a record")
            .routes(|_| http::router());
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
