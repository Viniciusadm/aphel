pub mod application;
pub mod dto;
mod http;
pub mod infrastructure;
pub mod section;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::database::Db;
pub const KEY: &str = "record_layouts";
pub struct RecordLayoutsModule;
impl Module for RecordLayoutsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "RecordLayouts",
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create form layouts",
            include_str!("../migrations/0950_create_form_layouts.sql"),
        ))
        .owned_tables(&["form_layouts"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::versioning::fragments::SectionFragmentLayout {
                section: "layouts",
                collections: vec![
                    aphel_sdk::configuration::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "layout",
                        key_fields: &["record_type", "context"],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(
            aphel_sdk::platform::infrastructure::custom_objects::RecordTypeScopedTable(
                "form_layouts",
            ),
        ));
        app.openapi_tag(http::TAG, "RecordLayouts")
            .routes(|_| http::router());
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(110, |pool, registry| {
                std::sync::Arc::new(section::LayoutsSection::new(
                    registry.clone(),
                    infrastructure::MySqlFormLayoutRepository::new(pool.clone()),
                    aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                        pool.clone(),
                    ),
                ))
            }),
        ));
        app.contribute::<dyn aphel_sdk::platform::application::fields::ConditionalRulesFactory>(
            std::sync::Arc::new(RulesFactory),
        );
        Ok(())
    }
}

struct RulesFactory;
impl aphel_sdk::platform::application::fields::ConditionalRulesFactory for RulesFactory {
    fn create(
        &self,
        pool: &Db,
    ) -> std::sync::Arc<dyn aphel_sdk::platform::application::fields::ConditionalRules> {
        std::sync::Arc::new(application::LayoutConditionalRules::new(
            infrastructure::MySqlFormLayoutRepository::new(pool.clone()),
            aphel_sdk::platform::infrastructure::auth::SystemClock,
        ))
    }
}
