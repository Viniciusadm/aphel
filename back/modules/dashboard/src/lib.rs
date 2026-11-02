pub mod engine;
pub mod engine_domain;
pub mod repository;
pub type DashboardWidget =
    std::sync::Arc<dyn engine::WidgetProvider<repository::MySqlDashboardPorts>>;
pub mod dto;
pub(crate) mod http;
pub(crate) mod layout;
pub(crate) mod queries;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod widgets;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use crate::widgets::{
    DealsByStageWidget, PendingCenterWidget, RecentActivityWidget, ReportTotalsWidget,
    SalesIndicatorsWidget,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "dashboard";

pub struct DashboardModule;

impl Module for DashboardModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Dashboard", APHEL_VERSION)
            .description(
                "Home dashboard: widget catalog and data, per-user layout and the organization default layout",
            )
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create dashboard layouts",
                include_str!("../migrations/0001_create_dashboard_layouts.sql"),
            ))
            .migration(Migration::new(
                2,
                "create dashboard default layouts",
                include_str!("../migrations/0002_create_dashboard_default_layouts.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            ::config_packages::application::versioning::fragments::SectionFragmentLayout {
                section: "dashboard",
                collections: vec![
                    ::config_packages::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "dashboard_layout",
                        key_fields: &[],
                    },
                ],
            },
        ));

        app.contribute(std::sync::Arc::new(::config_packages::infrastructure::ConfigSectionFactory::new(
            130,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::DashboardSection::new(crate::engine::WidgetRegistry::<crate::repository::MySqlDashboardPorts>::builtin().providers().map(|provider| provider.key().to_owned()).collect(), crate::store::LayoutStore::new(pool.clone()), pipelines::store::PipelineStore::new(pool.clone()), ::config_packages::infrastructure::MySqlConfigKeyRepository::new(pool.clone())))
            },
        )));
        app.openapi_tag(
            http::TAG,
            "Dashboard widgets, their data and the per-user layout",
        )
        .routes(|_| http::router());
        let widgets: [DashboardWidget; 5] = [
            Arc::new(SalesIndicatorsWidget),
            Arc::new(PendingCenterWidget),
            Arc::new(RecentActivityWidget),
            Arc::new(ReportTotalsWidget),
            Arc::new(DealsByStageWidget),
        ];
        for widget in widgets {
            app.contribute::<dyn crate::engine::WidgetProvider<crate::repository::MySqlDashboardPorts>>(widget);
        }
        Ok(())
    }
}
