pub mod application;
mod dto;
mod http;
pub mod infrastructure;
mod managed_writer;
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
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_sdk::objects::ManagedObjectWriter>(std::sync::Arc::new(
            managed_writer::Writer,
        ));
        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::versioning::fragments::SectionFragmentLayout {
                section: "custom_objects",
                collections: vec![
                    aphel_sdk::configuration::versioning::fragments::FragmentCollection {
                        path: Some("objects"),
                        kind: "custom_object",
                        key_fields: &["key"],
                    },
                    aphel_sdk::configuration::versioning::fragments::FragmentCollection {
                        path: Some("relationships"),
                        kind: "relationship",
                        key_fields: &["key"],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(10, |pool, registry| {
                std::sync::Arc::new(section::CustomObjectsSection::new(
                    registry.clone(),
                    aphel_sdk::platform::infrastructure::custom_objects::MySqlCustomObjectRepository::new(
                        pool.clone(),
                    ),
                    aphel_sdk::platform::infrastructure::custom_objects::MySqlRelationshipRepository::new(
                        pool.clone(),
                    ),
                    aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository::new(
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
