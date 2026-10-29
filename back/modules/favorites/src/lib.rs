pub(crate) mod domain;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "favorites";

pub struct FavoritesModule;

impl Module for FavoritesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Favorites", APHEL_VERSION)
            .description("Personal favorites: pinned records and saved views shown in the sidebar")
            .enabled_by_default()
            .owned_tables(&["favorites"])
            .migration(Migration::new(
                1,
                "create favorites",
                include_str!("../migrations/0001_create_favorites.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable("favorites"),
        ));
        app.openapi_tag(
            http::TAG,
            "Personal favorites: pinned records and saved views shown in the sidebar",
        )
        .routes(|_| http::router());
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
