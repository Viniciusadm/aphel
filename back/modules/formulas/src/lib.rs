pub mod application;
pub mod dto;
mod http;
pub mod infrastructure;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "formulas";
pub struct FormulasModule;
impl Module for FormulasModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Formulas",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create formula daily runs",
            include_str!("../migrations/1870_create_formula_daily_runs.sql"),
        ))
        .owned_tables(&["formula_daily_runs"])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(http::TAG, "Formulas")
            .routes(|_| http::router());
        app.contribute::<dyn aphel_kernel::application::fields::FieldDefinitionValidator>(
            std::sync::Arc::new(DefinitionValidator),
        );
        app.worker(|env| {
            aphel_kernel::modules::WorkerParts::default()
                .consumer(infrastructure::formula_events_consumer(
                    env.pool.clone(),
                    env.record_types.clone(),
                ))
                .handler(infrastructure::formula_recalculate_handler(
                    env.pool.clone(),
                    env.record_types.clone(),
                ))
                .handler(infrastructure::formula_daily_handler(env.pool.clone()))
                .schedule(application::formulas_daily_schedule())
        });
        Ok(())
    }
}

struct DefinitionValidator;
#[async_trait::async_trait]
impl aphel_kernel::application::fields::FieldDefinitionValidator for DefinitionValidator {
    async fn validate(
        &self,
        registry: &aphel_kernel::application::fields::RecordTypeRegistry,
        definitions: &dyn aphel_kernel::application::fields::FieldDefinitionReader,
        field: &aphel_kernel::domain::fields::FieldDefinition,
    ) -> Result<Vec<aphel_kernel::errors::FieldViolation>, AppError> {
        application::formula_definition_violations(registry, definitions, field).await
    }
}
