pub(crate) mod domain;
pub(crate) mod exposed_operations;
pub(crate) mod http;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

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
            aphel_sdk::platform::infrastructure::custom_objects::RecordTypeScopedTable("favorites"),
        ));
        app.openapi_tag(
            http::TAG,
            "Personal favorites: pinned records and saved views shown in the sidebar",
        )
        .routes(|_| http::router());
        exposed_operations::register(app);
        Ok(())
    }
}
