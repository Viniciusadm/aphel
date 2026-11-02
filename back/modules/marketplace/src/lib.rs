pub mod application;
mod dto;
mod http;
mod infrastructure;
mod settings;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "marketplace";
pub struct MarketplaceModule;
impl Module for MarketplaceModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Marketplace",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .adopts_core_feature(730)
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::application::organizations::OrganizationConfigExtension>(std::sync::Arc::new(settings::SourceConfiguration));
        app.openapi_tag(http::TAG, "Plugin and configuration marketplace")
            .routes(|_| http::router());
        Ok(())
    }
}
