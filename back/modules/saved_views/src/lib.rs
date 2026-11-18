mod store;
use store::favorites;
pub mod application;
pub mod dto;
mod http;
pub mod infrastructure;
mod reader;
pub mod section;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "saved_views";
pub struct SavedViewsModule;
impl Module for SavedViewsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "SavedViews",
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create saved views",
            include_str!("../migrations/0404_create_saved_views.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            2,
            "create saved view user defaults",
            include_str!("../migrations/0405_create_saved_view_user_defaults.sql"),
        ))
        .owned_tables(&["saved_views", "saved_view_user_defaults"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::views::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::views::SavedViewReader>(std::sync::Arc::new(reader::Reader));
        app.contribute::<dyn aphel_sdk::favorites::FavoriteViewSource>(std::sync::Arc::new(
            favorites::SavedViewFavorites,
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::versioning::fragments::SectionFragmentLayout {
                section: "views",
                collections: vec![
                    aphel_sdk::configuration::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "view",
                        key_fields: &["record_type", "key"],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::infrastructure::custom_objects::RecordTypeScopedTable(
                "saved_views",
            ),
        ));
        app.openapi_tag(http::TAG, "SavedViews")
            .routes(|_| http::router());
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(120, |pool, registry| {
                std::sync::Arc::new(section::ViewsSection::new(
                    registry.clone(),
                    infrastructure::MySqlSavedViewRepository::new(pool.clone()),
                    aphel_sdk::configuration::ConfigKeyClient::new(pool.clone()),
                    aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                        pool.clone(),
                    ),
                ))
            }),
        ));
        Ok(())
    }
}
