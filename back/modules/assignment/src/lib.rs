mod host_selection;
mod selector;
use aphel_contract_automation::AutomationBuilder;
use aphel_sdk::assignment::AssignmentPermissions;
pub(crate) mod automation;
pub mod domain;
pub(crate) mod dto;
pub(crate) mod exposed_operations;
pub(crate) mod http;
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

pub const KEY: &str = "assignment";

pub struct AssignmentModule;

impl Module for AssignmentModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Assignment", APHEL_VERSION)
            .description(
                "Record assignment rules (round robin, weighted, least loaded, routing by field)",
            )
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create lead distribution",
                include_str!("../migrations/0001_create_assignment.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_sdk::assignment::HostAssignment>(Arc::new(
            host_selection::HostAssignments,
        ));
        app.contribute::<dyn aphel_sdk::assignment::AssignmentSelector>(std::sync::Arc::new(
            selector::NativeAssignmentSelector,
        ));
        app.contract(aphel_sdk::assignment::DESCRIPTOR);
        app.contribute::<dyn aphel_sdk::points::ReferenceProvider>(Arc::new(AssignmentRuleRefs));
        app.openapi_tag(
            http::TAG,
            "Record assignment rules (round robin, weighted, least loaded, routing by field), their simulation and the rotation state used by automations, incoming webhooks and imports",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(
            Permission::ASSIGNMENT_MANAGE,
        ))
        .grant_to(["admin"]);
        app.action(Arc::new(automation::AssignByRule));
        app.contribute(Arc::new(
            aphel_sdk::configuration::ConfigSectionFactory::new(135, |pool, registry| {
                Arc::new(section::LeadDistributionSection::new(
                    registry.clone(),
                    store::DistributionStore::new(pool.clone()),
                ))
            }),
        ));
        exposed_operations::register(app);
        Ok(())
    }
}

struct AssignmentRuleRefs;
#[async_trait::async_trait]
impl aphel_sdk::points::ReferenceProvider for AssignmentRuleRefs {
    fn kind(&self) -> aphel_sdk::ReferenceKind {
        aphel_sdk::assignment::RULE
    }
    async fn exists(&self, ctx: &aphel_sdk::Ctx<'_>, id: &str) -> Result<bool, AppError> {
        aphel_sdk::points::lookup_exists(service::get(ctx, id).await)
    }
}
