pub(crate) mod automation;
pub mod domain;
pub(crate) mod dto;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod section;
pub mod service;
pub mod store;

pub use domain::{AssignmentSource, Counter, DistributionRule, RuleStats, UserStatus};
pub use service::*;
pub use store::{DistributionStore, PickRecord, RuleListFilter, TargetInfo};

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "lead_distribution";

pub struct LeadDistributionModule;

impl Module for LeadDistributionModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Lead distribution", APHEL_VERSION)
            .description(
                "Automatic lead distribution rules (round robin, weighted, least loaded, routing by field)",
            )
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create lead distribution",
                include_str!("../migrations/0001_create_lead_distribution.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Automatic lead distribution rules (round robin, weighted, least loaded, routing by field), their simulation and the rotation state used by automations, incoming webhooks and imports",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::LEAD_DISTRIBUTION_MANAGE,
        ))
        .grant_to(["admin"]);
        app.action(Arc::new(automation::AssignByRule));
        app.contribute(Arc::new(
            aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
                135,
                |pool, registry| {
                    Arc::new(section::LeadDistributionSection::new(
                        registry.clone(),
                        store::DistributionStore::new(pool.clone()),
                    ))
                },
            ),
        ));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }
}
