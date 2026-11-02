pub mod application;
pub mod dto;
mod http;
pub mod infrastructure;
pub mod section;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "record_layouts";
pub struct RecordLayoutsModule;
impl Module for RecordLayoutsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "RecordLayouts",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .adopts_core_feature(950)
        .migration(
            aphel_kernel::modules::Migration::new(
                1,
                "create form layouts",
                include_str!("../migrations/0950_create_form_layouts.sql"),
            )
            .adopts_core(950),
        )
        .owned_tables(&["form_layouts"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "layouts",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "layout",
                        key_fields: &["record_type", "context"],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable("form_layouts"),
        ));
        app.openapi_tag(http::TAG, "RecordLayouts")
            .routes(|_| http::router());
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(110, |pool, registry| {
                std::sync::Arc::new(section::LayoutsSection::new(
                    registry.clone(),
                    infrastructure::MySqlFormLayoutRepository::new(pool.clone()),
                    aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                        pool.clone(),
                    ),
                ))
            }),
        ));
        app.contribute::<dyn aphel_kernel::application::fields::ConditionalRulesFactory>(
            std::sync::Arc::new(RulesFactory),
        );
        Ok(())
    }
}

struct RulesFactory;
impl aphel_kernel::application::fields::ConditionalRulesFactory for RulesFactory {
    fn create(
        &self,
        pool: &sqlx::MySqlPool,
    ) -> std::sync::Arc<dyn aphel_kernel::application::fields::ConditionalRules> {
        std::sync::Arc::new(application::LayoutConditionalRules::new(
            infrastructure::MySqlFormLayoutRepository::new(pool.clone()),
            aphel_kernel::infrastructure::auth::SystemClock,
        ))
    }
}
