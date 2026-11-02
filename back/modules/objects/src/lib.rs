pub mod application;
mod dto;
mod http;
pub mod infrastructure;
mod section;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "objects";
pub struct ObjectsModule;
impl Module for ObjectsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Objects",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .adopts_core_feature(730)
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "custom_objects",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: Some("objects"),
                        kind: "custom_object",
                        key_fields: &["key"],
                    },
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: Some("relationships"),
                        kind: "relationship",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(10, |pool, registry| {
                std::sync::Arc::new(section::CustomObjectsSection::new(
                    registry.clone(),
                    aphel_kernel::infrastructure::custom_objects::MySqlCustomObjectRepository::new(
                        pool.clone(),
                    ),
                    aphel_kernel::infrastructure::custom_objects::MySqlRelationshipRepository::new(
                        pool.clone(),
                    ),
                    aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository::new(
                        pool.clone(),
                    ),
                ))
            }),
        ));

        app.openapi_tag(http::ADMIN_TAG, "Object administration")
            .openapi_tag(http::RELATIONSHIPS_TAG, "Object relationships")
            .routes(|_| http::router());
        Ok(())
    }
}
