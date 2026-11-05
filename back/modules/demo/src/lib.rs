pub(crate) mod cli;
pub(crate) mod dataset;
pub(crate) mod domain;
mod policy;
pub(crate) mod service;
pub mod settings;
mod status;
pub(crate) mod store;

pub(crate) use dataset::DEMO_SEED;
pub(crate) use domain::*;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;

pub(crate) const KEY: &str = "demo";

pub struct DemoModule;

impl Module for DemoModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Demo", APHEL_VERSION)
            .description(
                "Fictional demo data (seed, removal and reset) and the public demo mode status",
            )
            .optional("onboarding")
            .optional(aphel_sdk::money::KEY)
            .optional("workflow")
            .optional(aphel_sdk::catalog_items::KEY)
            .optional("pricing")
            .optional(aphel_sdk::line_items::KEY)
            .optional("quotes")
            .optional("approvals")
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create demo records",
                include_str!("../migrations/0001_create_demo_records.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_kernel::modules::HttpRequestPolicy>(std::sync::Arc::new(
            policy::DemoPolicy,
        ));
        app.contribute::<dyn aphel_sdk::platform::application::mail::MailDeliveryPolicy>(
            std::sync::Arc::new(policy::DemoPolicy),
        );
        app.openapi_tag(status::TAG, "Demonstration environment status")
            .public_routes(|_| status::router());
        app.contribute::<dyn aphel_kernel::modules::ModuleCommand>(std::sync::Arc::new(
            cli::DemoCli,
        ));
        Ok(())
    }
}
