pub(crate) mod dto;
pub(crate) mod history;
pub(crate) mod http;
pub(crate) mod normalize;
pub(crate) mod service;
pub(crate) mod sources;
pub(crate) mod store;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) use history::HistoryReport;
pub(crate) use normalize::*;
pub(crate) use service::{
    MigrationFileCommand, MigrationHistory, MigrationImport, MigrationPreview,
};
pub(crate) use sources::*;

pub(crate) const KEY: &str = "migrations_assistant";

pub struct MigrationsAssistantModule;

impl Module for MigrationsAssistantModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Migration assistant", APHEL_VERSION)
            .description("Assisted migration from other CRMs: recognized columns, conversion of their CSV exports and pre-filled imports")
            .enabled_by_default()
            .depends_on(imports::KEY, "*")
            .optional(activities::KEY)
            .optional(notes::KEY)
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(http::TAG, "Assisted migration from other CRMs (Pipedrive, HubSpot, RD Station CRM, Agendor): recognized columns, conversion of their CSV exports and pre-filled imports")
            .routes(|_| http::router());
        Ok(())
    }
}
